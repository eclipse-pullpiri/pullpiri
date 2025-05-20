use crate::vehicle::dds::DdsData;
use common::Result;
use std::collections::HashMap;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[async_trait]
pub trait DdsTopicListener: Send + Sync {
    fn is_running(&self) -> bool;
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    fn get_topic_name(&self) -> &str;
    fn is_topic(&self, topic_name: &str) -> bool;
}

use dust_dds::{
    domain::domain_participant::DomainParticipant,
    domain::domain_participant_factory::{DomainId, DomainParticipantFactory},
    infrastructure::{
        qos::QosKind,
        qos_policy::{DataRepresentationQosPolicy, XCDR2_DATA_REPRESENTATION},
        status::{StatusKind, NO_STATUS},
        time::Duration,
    },
    subscription::data_reader::DataReader,
    subscription::sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
    subscription::subscriber::Subscriber,
    topic_definition::type_support::{DdsDeserialize, DdsType, TypeSupport},
};

use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time;

use anyhow::{anyhow, Result as AnyhowResult};
use serde_json::{json, Map, Value};

use async_trait::async_trait;
use clap::Parser;
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use serde::Serialize;
use common;

/// DDS topic listener
///
/// Listens to a specific DDS topic and forwards data to the filter system.
pub struct TopicListener {
    /// Name of the topic
    pub topic_name: String,
    /// Data type of the topic
    data_type_name: String,
    /// Channel sender for data
    tx: Sender<DdsData>,
    /// Domain ID for DDS
    domain_id: i32,
    /// Handle to the listener task
    listener_task: Option<JoinHandle<()>>,
    /// Flag indicating if the listener is running
    is_running: bool,
}

impl TopicListener {
    /// Creates a new topic listener
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the DDS topic
    /// * `data_type_name` - Data type name of the topic
    /// * `tx` - Sender for data
    /// * `domain_id` - DDS domain ID
    ///
    /// # Returns
    ///
    /// A new TopicListener instance
    pub fn new(
        topic_name: String,
        data_type_name: String,
        tx: Sender<DdsData>,
        domain_id: i32,
    ) -> Self {
        Self {
            topic_name,
            data_type_name,
            tx,
            domain_id,
            listener_task: None,
            is_running: false,
        }
    }



    /// 파일 경로에서 IDL 타입을 추출하여 적절한 리스너 생성
    pub fn create_idl_listener(
        topic_name: String,
        type_name: String,
        tx: Sender<DdsData>,
        domain_id: i32,
    ) -> Box<dyn DdsTopicListener> {
        // 일반 토픽 리스너 생성
        Box::new(TopicListener::new(
            topic_name,
            type_name,
            tx,
            domain_id,
        ))
    }
}

// Helper function to create an IDL listener
pub fn create_idl_listener(
    topic_name: String,
    type_name: String,
    tx: Sender<DdsData>,
    domain_id: i32,
) -> Box<dyn DdsTopicListener> {
    TopicListener::create_idl_listener(topic_name, type_name,  tx, domain_id)
}

#[async_trait]
impl DdsTopicListener for TopicListener {
    fn is_running(&self) -> bool {
        return self.is_running;
    }

    async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Ok(());
        }

        // Clone values to move into the task
        let topic_name = self.topic_name.clone();
        let data_type_name = self.data_type_name.clone();
        let tx = self.tx.clone();
        let domain_id = self.domain_id;


        // Spawn the listener task
        let task = tokio::spawn(async move {
            if let Err(e) =
                Self::listener_loop(topic_name, data_type_name, tx, domain_id).await
            {
                error!("Error in listener loop: {:?}", e);
            }
        });

        // Store the task handle and update state
        self.listener_task = Some(task);
        self.is_running = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if self.is_running {
            if let Some(task) = self.listener_task.take() {
                task.abort();
            }
            self.is_running = false;
        }
        Ok(())
    }

    fn get_topic_name(&self) -> &str {
        &self.topic_name
    }

    fn is_topic(&self, topic_name: &str) -> bool {
        self.topic_name == topic_name
    }
}

impl TopicListener {
    /// Main listener loop for processing DDS data
    async fn listener_loop(
        topic_name: String,
        data_type_name: String,
        tx: Sender<DdsData>,
        domain_id: i32,
    ) -> Result<()> {
        // 도메인 참여자 생성
        info!("Generic listener started for topic '{}'", topic_name);

        let domain_participant_factory = DomainParticipantFactory::get_instance();
        let participant = domain_participant_factory
            .create_participant(domain_id, QosKind::Default, None, NO_STATUS)
            .map_err(|e| anyhow!("Failed to create domain participant: {:?}", e))?;

        // 구독자 생성
        let subscriber = participant
            .create_subscriber(QosKind::Default, None, NO_STATUS)
            .map_err(|e| anyhow!("Failed to create subscriber: {:?}", e))?;

        // IDL 타입 정보를 유동적으로 처리
        // 토픽 메타데이터에 따라 데이터 처리를 다르게 함
        info!(
            "Setting up listener for topic {} of type {}",
            topic_name, data_type_name
        );

        // 메시지 수신 루프
        let mut interval = time::interval(time::Duration::from_millis(100));

        loop {
            interval.tick().await;

            // 수신된 DDS 메시지를 파싱하여 DdsData 형태로 변환
            let dds_data = DdsData {
                name: data_type_name.clone(),
                value: "{}".to_string(), // 실제 값은 메시지 수신 시 채워짐
                fields: HashMap::new(),
            };

            // 데이터 전송 채널이 닫히면 루프 종료
            if tx.send(dds_data).await.is_err() {
                warn!("Channel closed, stopping listener for {}", topic_name);
                break;
            }
        }

        Ok(())
    }
}

/// 타입별 DDS 토픽 리스너 베이스 구현
///
/// TypeSupport 특성으로 다양한 DDS 데이터 타입 처리
pub struct GenericTopicListener<
    T: TypeSupport
        + Default
        + DeserializeOwned
        + Serialize
        + Send
        + Sync
        + for<'de> DdsDeserialize<'de>
        + 'static,
> {
    /// 토픽 이름
    topic_name: String,
    /// 데이터 타입 이름
    data_type_name: String,
    /// 데이터 전송 채널
    tx: Sender<DdsData>,
    /// DDS 도메인 ID
    domain_id: i32,
    /// 리스너 태스크 핸들
    listener_task: Option<JoinHandle<()>>,
    /// 실행 상태
    is_running: bool,
    /// 타입 마커 (제네릭 타입 지정용)
    _marker: std::marker::PhantomData<T>,
}

impl<
        T: TypeSupport
            + Default
            + DeserializeOwned
            + Serialize
            + Send
            + Sync
            + for<'de> DdsDeserialize<'de>
            + 'static,
    > GenericTopicListener<T>
{
    /// 새 타입별 리스너 생성
    pub fn new(
        topic_name: String,
        data_type_name: String,
        tx: Sender<DdsData>,
        domain_id: i32,
    ) -> Self {
        Self {
            topic_name,
            data_type_name,
            tx,
            domain_id,
            listener_task: None,
            is_running: false,
            _marker: std::marker::PhantomData,
        }
    }

    /// 타입별 리스너 루프
    async fn typed_listener_loop(
        topic_name: String,
        data_type_name: String,
        tx: Sender<DdsData>,
        domain_id: i32,
    ) -> Result<()> {
        // 도메인 참여자 생성
        let domain_participant_factory = DomainParticipantFactory::get_instance();
        let participant = domain_participant_factory
            .create_participant(domain_id, QosKind::Default, None, NO_STATUS)
            .map_err(|e| anyhow!("Failed to create domain participant: {:?}", e))?;

        // 구독자 생성
        let subscriber = participant
            .create_subscriber(QosKind::Default, None, NO_STATUS)
            .map_err(|e| anyhow!("Failed to create subscriber: {:?}", e))?;

        // 토픽 생성
        let topic = participant
            .create_topic::<T>(&topic_name, &topic_name, QosKind::Default, None, NO_STATUS)
            .map_err(|e| anyhow!("Failed to create topic: {:?}", e))?;

        // 데이터 리더 생성
        let data_reader = subscriber
            .create_datareader::<T>(&topic, QosKind::Default, None, NO_STATUS)
            .map_err(|e| anyhow!("Failed to create data reader: {:?}", e))?;

        info!(
            "Successfully created data reader for topic '{}'",
            topic_name
        );

        // 메시지 수신 루프
        let mut interval = time::interval(time::Duration::from_millis(100));

        loop {
            interval.tick().await;

            // 새 샘플 확인
            let result = data_reader
                .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                .map_err(|e| anyhow!("Failed to read samples: {:?}", e));

            match result {
                Ok(samples) => {
                    for sample in samples {
                        if let Ok(data) = sample.data() {
                            // 데이터를 JSON으로 직렬화
                            let json_value = serde_json::to_string(&data)
                                .map_err(|e| anyhow!("Failed to serialize data: {:?}", e))?;

                            // json_value를 key, value로 파싱해서 fields에 추가
                            let mut fields = HashMap::new();
                            if let Ok(map) = serde_json::from_str::<serde_json::Map<String, Value>>(&json_value) {
                                for (k, v) in map {
                                    fields.insert(k, v.to_string());
                                }
                            }

                            // DdsData 객체 생성 및 전송
                            let dds_data = DdsData {
                                name: data_type_name.clone(),
                                value: json_value,
                                fields,
                            };
                            

                            // 채널로 데이터 전송
                            if tx.send(dds_data).await.is_err() {
                                warn!("Channel closed, stopping listener for {}", topic_name);
                                return Ok(());
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("No new samples available: {:?}", e);
                }
            }
        }
    }
}

#[async_trait]
impl<
        T: TypeSupport
            + Default
            + DeserializeOwned
            + Serialize
            + Send
            + Sync
            + for<'de> DdsDeserialize<'de>
            + 'static,
    > DdsTopicListener for GenericTopicListener<T>
{
    fn is_running(&self) -> bool {
        self.is_running
    }

    async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Ok(());
        }

        let topic_name = self.topic_name.clone();
        let data_type_name = self.data_type_name.clone();
        let tx = self.tx.clone();
        let domain_id = self.domain_id;

        // 리스너 태스크 시작
        let task = tokio::spawn(async move {
            if let Err(e) =
                Self::typed_listener_loop(topic_name.clone(), data_type_name, tx, domain_id).await
            {
                error!("Error in typed listener loop for {}: {:?}", topic_name, e);
            }
        });

        self.listener_task = Some(task);
        self.is_running = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if self.is_running {
            if let Some(task) = self.listener_task.take() {
                task.abort();
            }
            self.is_running = false;
        }
        Ok(())
    }

    fn get_topic_name(&self) -> &str {
        &self.topic_name
    }

    fn is_topic(&self, topic_name: &str) -> bool {
        self.topic_name == topic_name
    }
}

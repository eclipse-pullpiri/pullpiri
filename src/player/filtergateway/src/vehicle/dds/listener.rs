use crate::vehicle::{self, dds::DdsData};
use dust_dds::{
    dds_async::{
        domain_participant_factory::DomainParticipantFactoryAsync,
        //subscriber::datareader::DataReaderAsync,
        wait_set::{ConditionAsync, WaitSetAsync},
    },
    domain::domain_participant_factory::{DomainId, DomainParticipantFactory},
    infrastructure::{
        qos::{DomainParticipantQos, QosKind},
        status::{StatusKind, NO_STATUS},
        time::Duration,
    },
    subscription::sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
    topic_definition::type_support::DdsType,
};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

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
    domain_id: u32,
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
        domain_id: u32,
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

    /// Starts listening to the topic
    ///
    /// Creates a DDS datareader for the specified topic and
    /// continuously forwards received data to the filter.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub async fn start(&mut self) -> common::Result<()> {
        // TODO: Implementation
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
            if let Err(e) = Self::listener_loop(topic_name, data_type_name, tx, domain_id).await {
                eprintln!("Error in listener loop: {:?}", e);
            }
        });

        // Store the task handle and update state
        self.listener_task = Some(task);
        self.is_running = true;
        Ok(())
    }

    /// The main listener loop for processing DDS data
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic
    /// * `data_type_name` - Data type name
    /// * `tx` - Sender for data
    /// * `domain_id` - DDS domain ID
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    async fn listener_loop(
        topic_name: String,
        data_type_name: String,
        tx: Sender<DdsData>,
        domain_id: u32,
    ) -> common::Result<()> {
        // TODO: Implementation
        // Get the domain participant factory

        let factory = DomainParticipantFactoryAsync::get_instance();

        // Create a domain participant
        let participant = factory
            .create_participant(domain_id as i32, QosKind::Default, None, NO_STATUS)
            .await
            .map_err(|e| format!("Failed to create participant: {:?}", e))?;

        let topic = participant
            .create_topic::<VehicleData>(
                &topic_name,
                &data_type_name,
                QosKind::Default,
                None,
                NO_STATUS,
            )
            .await
            .map_err(|e| format!("Failed to create topic: {:?}", e))?;

        // Create a subscriber with default QoS
        let subscriber = participant
            .create_subscriber(QosKind::Default, None, NO_STATUS)
            .await
            .unwrap();

        let reader = subscriber
            .create_datareader::<VehicleData>(&topic, QosKind::Default, None, NO_STATUS)
            .await
            .unwrap();

        let cond = reader.get_statuscondition();
        cond.set_enabled_statuses(&[StatusKind::DataAvailable])
            .await
            .unwrap();
        let mut reader_wait_set = WaitSetAsync::new();
        reader_wait_set
            .attach_condition(ConditionAsync::StatusCondition(cond))
            .await
            .unwrap();
        reader_wait_set.wait(Duration::new(10, 0)).await.unwrap();

        let samples = reader
            .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
            .await
            .unwrap();

        assert_eq!(samples.len(), 1);
        // assert_eq!(samples[0].data().unwrap(), data);

        Ok(())
    }

    /// Stops the listener
    ///
    /// Aborts the listener task and cleans up resources.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub async fn stop(&mut self) -> common::Result<()> {
        // TODO: Implementation
        if self.is_running {
            if let Some(task) = self.listener_task.take() {
                task.abort();
            }
            self.is_running = false;
        }
        Ok(())
    }

    /// Checks if the listener is for a specific topic
    ///
    /// # Arguments
    ///
    /// * `topic_name` - Name of the topic to check
    ///
    /// # Returns
    ///
    /// `true` if this listener is for the specified topic
    pub fn is_topic(&self, topic_name: &str) -> bool {
        self.topic_name == topic_name
    }

    /// Checks if the listener is running
    ///
    /// # Returns
    ///
    /// `true` if the listener is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

/// Generic DDS Data type for dynamic data handling
///
/// This is a placeholder. In a real implementation, you would define
/// proper data types for each vehicle data topic.
#[derive(Debug, PartialEq, DdsType)]
pub struct VehicleData {
    /// Identifier for the data
    #[dust_dds(key)]
    pub id: u32,
    /// Name of the topic
    pub topic_name: String,
    /// JSON-encoded value
    pub value: String,
}

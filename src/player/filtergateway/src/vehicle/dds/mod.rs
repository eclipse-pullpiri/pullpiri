use common::Result;
use anyhow::anyhow;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

mod listener;

// Re-export the modules
pub use listener::{create_idl_listener, DdsTopicListener};

use crate::grpc::receiver;
// DdsData structure to represent parsed IDL data
#[derive(Debug,  Clone, Serialize, Deserialize)]
pub struct DdsData {
    pub name: String,
    pub value: String,
    pub fields: HashMap<String, String>,
}


/// DDS 관리자 - 여러 DDS 리스너를 관리
pub struct DdsManager {
    /// 활성 리스너 맵 (토픽 이름 → 리스너)
    listeners: HashMap<String, Box<dyn DdsTopicListener>>,
    /// DDS 데이터 송신용 채널
    tx: Sender<DdsData>,
    /// DDS 데이터 수신용 채널
    rx: Mutex<Receiver<DdsData>>,
    /// DDS 도메인 ID
    domain_id: i32,

}

impl DdsManager {
    /// 새 DDS 관리자 생성
    pub fn new(tx:Sender<DdsData> ) -> Self {
        // let (tx, rx) = mpsc::channel(100);

        Self {
            listeners: HashMap::new(),
            tx,
            rx: Mutex::new(mpsc::channel(100).1),
            domain_id: 100,
        }
    }
    /// 런타임에 IDL 디렉토리 스캔 및 처리
    pub async fn scan_idl_directory(&mut self, dir: &Path) -> Result<Vec<String>> {
        info!("Scanning IDL directory at runtime: {:?}", dir);
        let mut found_types = Vec::new();
        
        // 디렉토리 존재 확인
        if !dir.exists() {
            return Ok(found_types);
        }
        
        // IDL 파일 검색
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "idl") {
                if let Some(stem) = path.file_stem() {
                    let type_name = stem.to_string_lossy().to_string();
                    found_types.push(type_name);
                }
            }
        }
        
        info!("Found {} IDL types at runtime", found_types.len());
        Ok(found_types)
    }
    /// 타입명에 맞는 특화된 리스너 생성
    pub async fn create_typed_listener(
        &mut self,
        topic_name: String,
        data_type_name: String,
    ) -> Result<()> {
        // 이미 존재하는 리스너인지 확인
        if self.listeners.contains_key(&topic_name) {
            return Ok(());
        }
        print!("DDSManager - Creating typed listener for topic '{}'", topic_name);
        
        // 레지스트리를 통한 타입별 리스너 생성 시도
        if let Some(mut typed_listener) = dds_type_registry::create_typed_listener(
            &data_type_name,
            topic_name.clone(),
            self.tx.clone(),
            self.domain_id,
        ) {
            // 리스너 시작
            typed_listener
                .start()
                .await
                .map_err(|e| anyhow!("Failed to start typed listener: {:?}", e))?;
                
            info!("Started typed listener for {} with specific type {}", topic_name, data_type_name);
            
            // 리스너 맵에 추가
            self.listeners.insert(topic_name, typed_listener);
            return Ok(());
        }
        
        // 타입별 리스너를 찾지 못한 경우 일반 리스너 생성
        warn!("No specific type handler for '{}', using generic listener", data_type_name);
        self.create_listener(topic_name, data_type_name).await
    }
    
    /// 사용 가능한 DDS 타입 목록 조회
    pub fn list_available_types(&self) -> Vec<String> {
        dds_type_metadata::get_available_types()
    }

    /// DDS 도메인 ID 설정
    pub fn set_domain_id(&mut self, domain_id: i32) {
        self.domain_id = domain_id;
    }

    /// DDS 데이터 송신자 얻기
    pub fn get_sender(&self) -> Sender<DdsData> {
        self.tx.clone()
    }

    /// DDS 데이터 수신자 얻기
    pub async fn get_receiver(&mut self) -> &mut Mutex<Receiver<DdsData>> {
        &mut self.rx
    }

    /// 리스너 생성 및 등록
    pub async fn create_listener(
        &mut self,
        topic_name: String,
        data_type_name: String,
    ) -> Result<()> {
        // 이미 존재하는 리스너인지 확인
        if self.listeners.contains_key(&topic_name) {
            return Ok(());
        }

        // 관련 IDL 파일 검색
        // let idl_path = self.find_idl_for_type(&data_type_name)?;


        // 리스너 생성
        let mut listener = create_idl_listener(
            topic_name.clone(),
            data_type_name,
            self.tx.clone(),
            self.domain_id,
        );


        // 리스너 시작
        listener
            .start()
            .await
            .map_err(|e| anyhow!("Failed to start listener: {:?}", e))?;

        // 리스너 맵에 추가
        self.listeners.insert(topic_name, listener);

        Ok(())
    }

    /// 리스너 제거
    /// 리스너 제거
    pub async fn remove_listener(&mut self, topic_name: &str) -> Result<()> {
        
        if let Some(mut listener) = self.listeners.remove(topic_name) {
            listener
                .stop()
                .await
                .map_err(|e| anyhow!("Failed to stop listener: {:?}", e))?;
        }


        Ok(())
    }


    /// 모든 리스너 중지
    pub async fn stop_all(&mut self) -> Result<()> {
        for (_, mut listener) in std::mem::take(&mut self.listeners) {
            if let Err(e) = listener.stop().await {
                eprintln!("Failed to stop listener: {:?}", e);
            }
        }

        Ok(())
    }


    /// DDS 관리자 초기화
    pub async fn init(&mut self) -> Result<()> {
        info!("Initializing DDS Manager");

        let default_domain_id = 0;

        // 프로젝트 루트 기준 설정 파일 경로 검색
        let mut settings_path = PathBuf::from("/home/edo/2025/projects/pullpiri/src/settings.yaml");


        info!("Reading settings from {:?}", settings_path);
        let content = fs::read_to_string(&settings_path)?;

        // JSON 또는 YAML 파싱
        let settings = serde_json::from_str::<serde_json::Value>(&content)?;



        let domain_id = settings
            .get("dds")
            .and_then(|dds| dds.get("domain_id"))
            .and_then(|id| id.as_i64())
            .map(|id| id as i32)
            .unwrap_or(default_domain_id);

        info!("Domain ID from settings: {}", domain_id);

        // OUT_DIR 값 확인 (런타임에서는 사용하지 않으나 로깅용)
        if let Some(out_dir) = settings
            .get("dds")
            .and_then(|dds| dds.get("out_dir"))
            .and_then(|path| path.as_str())
        {
            info!("Output directory from settings: {}", out_dir);
        }

 

        self.domain_id = domain_id;

        Ok(())
    }
}

// Include generated DDS types at runtime
#[allow(unused)]
pub mod dds_types {
    // Try including the generated code from build.rs
    // If no IDL files exist, this will just include an empty file
    // No placeholder types will be created
    #[allow(unused_variables, unused_imports)]            
    include! {
        concat! {
            env!("OUT_DIR"),
            "/dds_types.rs"
        }
    }
}
#[allow(unused)]
pub mod dds_type_metadata {
    // Try including the generated code from build.rs
    // If no IDL files exist, this will just include an empty file
    // No placeholder types will be created
    
        
    /// Returns a vector of available DDS type names obtained from the generated type metadata.
    pub fn get_available_types() -> Vec<String> {
        dds_type_metadata::get_type_metadata()
            .keys()
            .cloned()
            .collect()
    }
    
    // Always include the generated type metadata; this file is generated by build.rs.
    
    pub mod dds_type_metadata {
        #[allow(unused_variables, unused_imports)]
        include! {
            concat!(env!("OUT_DIR"), "/dds_type_metadata.rs")
        }

    }
}
// Include generated type registry
#[allow(unused)]
pub mod dds_type_registry {
    use super::*;
    use crate::vehicle::dds::listener::{DdsTopicListener, create_idl_listener};
    use tokio::sync::mpsc::Sender;
    
    // 빌드 중에 생성된 DDS 타입 레지스트리를 조건부로 포함
    #[cfg(feature = "dds_type_registry_exists")]
    include!(concat!(env!("OUT_DIR"), "/dds_type_registry.rs"));
    
    // 빌드 전이거나 레지스트리가 생성되지 않은 경우 기본 구현 제공
    #[cfg(not(feature = "dds_type_registry_exists"))]
    pub fn create_typed_listener(
        type_name: &str,
        topic_name: String,
        tx: Sender<DdsData>,
        domain_id: i32,
    ) -> Option<Box<dyn DdsTopicListener>> {
        log::info!("No type registry found. Looking for type handler: {}", type_name);
        None
    }
}
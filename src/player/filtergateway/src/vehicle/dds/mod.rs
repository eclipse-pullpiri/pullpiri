use common::Result;
use anyhow::anyhow;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

mod listener;

// Re-export the modules
pub use listener::{create_idl_listener, DdsTopicListener};


// DdsData structure to represent parsed IDL data
#[derive(Debug,  Clone, Serialize, Deserialize)]
pub struct DdsData {
    pub name: String,
    pub value: String,
    pub fields: HashMap<String, String>,
}

/// DDS Manager - Manages multiple DDS listeners
pub struct DdsManager {
    /// Active listener map (topic name → listener)
    listeners: HashMap<String, Box<dyn DdsTopicListener>>,
    /// Channel for sending DDS data
    tx: Sender<DdsData>,
    /// Channel for receiving DDS data
    rx: Mutex<Receiver<DdsData>>,
    /// DDS domain ID
    domain_id: i32,

}

impl DdsManager {
    /// Create new DDS manager
    pub fn new(tx:Sender<DdsData> ) -> Self {
        // let (tx, rx) = mpsc::channel(100);

        Self {
            listeners: HashMap::new(),
            tx,
            rx: Mutex::new(mpsc::channel(100).1),
            domain_id: 100,
        }
    }
    /// Scan and process IDL directory at runtime
    pub async fn scan_idl_directory(&mut self, dir: &Path) -> Result<Vec<String>> {
        info!("Scanning IDL directory at runtime: {:?}", dir);
        let mut found_types = Vec::new();
        
        // Check if directory exists
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
            warn!("Listener for topic '{}' already exists", topic_name);
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
                
            
            println!("Started typed listener for {} with specific type {}", topic_name, data_type_name);
            
            // 리스너 맵에 추가
            self.listeners.insert(topic_name, typed_listener);
            return Ok(());
        }
        
        // Create generic listener if no type-specific listener is found
        warn!("No specific type handler for '{}', using generic listener", data_type_name);
        self.create_listener(topic_name, data_type_name).await
    }
    
    /// Get list of available DDS types
    pub fn list_available_types(&self) -> Vec<String> {
        dds_type_metadata::get_available_types()
    }

    /// Set DDS domain ID
    pub fn set_domain_id(&mut self, domain_id: i32) {
        self.domain_id = domain_id;
    }

    /// Get DDS data sender
    pub fn get_sender(&self) -> Sender<DdsData> {
        self.tx.clone()
    }

    /// Get DDS data receiver
    pub async fn get_receiver(&mut self) -> &mut Mutex<Receiver<DdsData>> {
        &mut self.rx
    }

    /// Create and register listener
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
    pub async fn remove_listener(&mut self, topic_name: &str) -> Result<()> {
        
        if let Some(mut listener) = self.listeners.remove(topic_name) {
            listener
                .stop()
                .await
                .map_err(|e| anyhow!("Failed to stop listener: {:?}", e))?;
        }


        Ok(())
    }


    /// Stop all listeners
    pub async fn stop_all(&mut self) -> Result<()> {
        for (_, mut listener) in std::mem::take(&mut self.listeners) {
            if let Err(e) = listener.stop().await {
                eprintln!("Failed to stop listener: {:?}", e);
            }
        }

        Ok(())
    }


    /// Initialize DDS Manager
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

        // Check OUT_DIR value (not used at runtime, only for logging)
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
    #[allow(unused_variables, unused_imports)]
    include! {
        concat!(env!("OUT_DIR"), "/dds_types.rs")
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
//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tokio::sync::mpsc;

    // Mock implementation of DdsTopicListener for testing purposes
    struct MockDdsTopicListener {
        running: bool,
        topic_name: String,
    }

    #[async_trait::async_trait]
    impl DdsTopicListener for MockDdsTopicListener {
        // Simulates starting the listener
        async fn start(&mut self) -> Result<()> {
            self.running = true;
            Ok(())
        }

        // Simulates stopping the listener
        async fn stop(&mut self) -> Result<()> {
            self.running = false;
            Ok(())
        }

        // Checks if the listener is running
        fn is_running(&self) -> bool {
            self.running
        }

        // Returns the topic name associated with the listener
        fn get_topic_name(&self) -> &str {
            &self.topic_name
        }

        // Checks if the listener is associated with a specific topic
        fn is_topic(&self, topic: &str) -> bool {
            self.topic_name == topic
        }
    }

    // Test scanning a non-existent IDL directory
    #[tokio::test]
    async fn test_scan_idl_directory_with_nonexistent_path() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let dir = Path::new("/nonexistent/path");
        let result = manager.scan_idl_directory(dir).await.unwrap();
        assert!(result.is_empty());
    }

    // Test scanning an empty IDL directory
    #[tokio::test]
    async fn test_scan_idl_directory_with_empty_directory() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let temp_dir = tempfile::tempdir().unwrap();
        let result = manager.scan_idl_directory(temp_dir.path()).await.unwrap();
        assert!(result.is_empty());
    }

    // Test creating a typed listener when one already exists
    #[tokio::test]
    async fn test_create_typed_listener_with_existing_listener() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let topic_name = "test_topic".to_string();
        let data_type_name = "test_type".to_string();

        // Insert a mock listener into the manager
        manager.listeners.insert(
            topic_name.clone(),
            Box::new(MockDdsTopicListener {
                running: false,
                topic_name: topic_name.clone(),
            }),
        );

        // Attempt to create a typed listener for the same topic
        let result = manager.create_typed_listener(topic_name.clone(), data_type_name.clone()).await;
        assert!(result.is_ok());
    }

    // Test removing a listener that does not exist
    #[tokio::test]
    async fn test_remove_listener_with_nonexistent_listener() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let topic_name = "nonexistent_topic";
        let result = manager.remove_listener(topic_name).await;
        assert!(result.is_ok());
    }

    // Test stopping all listeners when none exist
    #[tokio::test]
    async fn test_stop_all_with_no_listeners() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let result = manager.stop_all().await;
        assert!(result.is_ok());
    }

    // Test initializing the manager with an invalid settings path
    #[tokio::test]
    async fn test_init_with_invalid_settings_path() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        manager.domain_id = 0;

        // Attempt to initialize the manager
        let result = manager.init().await;
        assert!(result.is_err());
    }

    // Test retrieving the sender from the manager
    #[tokio::test]
    async fn test_get_sender() {
        let (tx, _) = mpsc::channel(100);
        let manager = DdsManager::new(tx.clone());
        let sender = manager.get_sender();
        assert_eq!(sender.capacity(), tx.capacity());
    }
}
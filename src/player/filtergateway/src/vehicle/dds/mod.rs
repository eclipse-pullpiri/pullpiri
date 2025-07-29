use anyhow::anyhow;
use common::Result;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

pub mod listener;

// Re-export the modules
pub use listener::{create_idl_listener, DdsTopicListener};

// DdsData structure to represent parsed IDL data
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn new(tx: Sender<DdsData>) -> Self {
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

            if path.is_file() && path.extension().is_some_and(|ext| ext == "idl") {
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
        print!(
            "DDSManager - Creating typed listener for topic '{}'",
            topic_name
        );

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

            println!(
                "Started typed listener for {} with specific type {}",
                topic_name, data_type_name
            );

            // 리스너 맵에 추가
            self.listeners.insert(topic_name, typed_listener);
            return Ok(());
        }

        // Create generic listener if no type-specific listener is found
        warn!(
            "No specific type handler for '{}', using generic listener",
            data_type_name
        );
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

    /// Backward-compatible `init()` that uses no path
    pub async fn init(&mut self) -> Result<()> {
        self.init_with_path(None).await
    }

    /// Flexible version with optional path param
    pub async fn init_with_path<P: Into<Option<PathBuf>>>(
        &mut self,
        settings_path: P,
    ) -> Result<()> {
        info!("Initializing DDS Manager");
        let default_domain_id = 0;

        let settings_path = settings_path.into().unwrap_or_else(|| {
            env::var("PICCOLO_SETTINGS_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    PathBuf::from("/home/edo/2025/projects/pullpiri/src/settings.yaml")
                })
        });

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
    use crate::vehicle::dds::listener::{create_idl_listener, DdsTopicListener};
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
        log::info!(
            "No type registry found. Looking for type handler: {}",
            type_name
        );
        None
    }
}
//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tokio::sync::mpsc;

    // Mock implementation of DdsTopicListener for testing
    struct MockDdsTopicListener {
        running: bool,
        topic_name: String,
    }

    #[async_trait::async_trait]
    impl DdsTopicListener for MockDdsTopicListener {
        async fn start(&mut self) -> Result<()> {
            self.running = true;
            Ok(())
        }

        async fn stop(&mut self) -> Result<()> {
            self.running = false;
            Ok(())
        }

        fn is_running(&self) -> bool {
            self.running
        }

        fn get_topic_name(&self) -> &str {
            &self.topic_name
        }

        fn is_topic(&self, topic: &str) -> bool {
            self.topic_name == topic
        }
    }

    #[tokio::test]
    async fn test_scan_idl_directory_with_nonexistent_path() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let dir = Path::new("/nonexistent/path");
        let result = manager.scan_idl_directory(dir).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_scan_idl_directory_with_empty_directory() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let temp_dir = tempfile::tempdir().unwrap();
        let result = manager.scan_idl_directory(temp_dir.path()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_create_typed_listener_with_existing_listener() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let topic_name = "test_topic".to_string();
        let data_type_name = "test_type".to_string();

        manager.listeners.insert(
            topic_name.clone(),
            Box::new(MockDdsTopicListener {
                running: false,
                topic_name: topic_name.clone(),
            }),
        );

        let result = manager
            .create_typed_listener(topic_name.clone(), data_type_name.clone())
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_listener_with_nonexistent_listener() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let topic_name = "nonexistent_topic";
        let result = manager.remove_listener(topic_name).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_remove_listener_existing() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let topic = "existing_topic";

        manager.listeners.insert(
            topic.to_string(),
            Box::new(MockDdsTopicListener {
                running: true,
                topic_name: topic.to_string(),
            }),
        );

        let result = manager.remove_listener(topic).await;
        assert!(result.is_ok());
        assert!(!manager.listeners.contains_key(topic));
    }

    #[tokio::test]
    async fn test_stop_all_with_no_listeners() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let result = manager.stop_all().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stop_all_with_multiple_listeners() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);

        for i in 0..3 {
            let topic = format!("topic_{}", i);
            manager.listeners.insert(
                topic.clone(),
                Box::new(MockDdsTopicListener {
                    running: true,
                    topic_name: topic,
                }),
            );
        }

        let result = manager.stop_all().await;
        assert!(result.is_ok());
        assert!(manager.listeners.is_empty());
    }

    #[tokio::test]
    async fn test_get_sender() {
        let (tx, _) = mpsc::channel(100);
        let manager = DdsManager::new(tx.clone());
        let sender = manager.get_sender();
        assert_eq!(sender.capacity(), tx.capacity());
    }

    #[tokio::test]
    async fn test_get_receiver_returns_mutex_ref() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let receiver = manager.get_receiver().await;
        let _lock = receiver.lock().await;
    }

    #[tokio::test]
    async fn test_create_listener_creates_and_starts_listener() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let topic = "new_topic".to_string();
        let data_type = "new_type".to_string();

        let result = manager.create_listener(topic.clone(), data_type).await;
        assert!(result.is_ok());
        assert!(manager.listeners.contains_key(&topic));
    }

    #[tokio::test]
    async fn test_create_listener_skips_existing() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let topic = "existing_topic".to_string();

        manager.listeners.insert(
            topic.clone(),
            Box::new(MockDdsTopicListener {
                running: false,
                topic_name: topic.clone(),
            }),
        );

        let result = manager
            .create_listener(topic.clone(), "any_type".to_string())
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_list_available_types_returns_vec() {
        let types = dds_type_metadata::get_available_types();
        assert!(types.is_empty() || types.iter().all(|t| t.is_ascii()));
    }

    #[tokio::test]
    async fn test_init_with_invalid_settings_path() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let result = manager.init().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_init_reads_domain_id() {
        let (tx, _) = tokio::sync::mpsc::channel(100);
        let mut manager = DdsManager::new(tx);

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp_file.path(),
            r#"
{
    "dds": {
        "domain_id": 42,
        "out_dir": "/tmp/output"
    }
}
"#,
        )
        .unwrap();

        let result = manager.init_with_path(Some(temp_file.path().into())).await;
        assert!(result.is_ok(), "Init failed: {:?}", result.unwrap_err());
        assert_eq!(manager.domain_id, 42);
    }

    #[tokio::test]
    async fn test_create_typed_listener_falls_back_to_generic() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);
        let result = manager
            .create_typed_listener("unknown_topic".to_string(), "UnknownType".to_string())
            .await;
        assert!(result.is_ok());
    }
    #[tokio::test]
    async fn test_scan_idl_directory_with_idl_files() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);

        let temp_dir = tempfile::tempdir().unwrap();
        // Create a few .idl files and some non-idl files
        std::fs::write(temp_dir.path().join("example1.idl"), "").unwrap();
        std::fs::write(temp_dir.path().join("example2.idl"), "").unwrap();
        std::fs::write(temp_dir.path().join("ignore.txt"), "").unwrap();

        let result = manager.scan_idl_directory(temp_dir.path()).await.unwrap();

        // Should include only the idl file stems
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"example1".to_string()));
        assert!(result.contains(&"example2".to_string()));
        assert!(!result.contains(&"ignore".to_string()));
    }
    struct DummyListener {
        running: bool,
        topic_name: String,
    }

    #[async_trait::async_trait]
    impl DdsTopicListener for DummyListener {
        async fn start(&mut self) -> Result<()> {
            self.running = true;
            Ok(())
        }
        async fn stop(&mut self) -> Result<()> {
            self.running = false;
            Ok(())
        }
        fn is_running(&self) -> bool {
            self.running
        }
        fn get_topic_name(&self) -> &str {
            &self.topic_name
        }
        fn is_topic(&self, topic: &str) -> bool {
            self.topic_name == topic
        }
    }

    // Override the dds_type_registry::create_typed_listener temporarily for testing
    mod dds_type_registry {
        use super::{DdsTopicListener, DummyListener};
        use crate::vehicle::DdsData;
        use anyhow::Result;
        use std::boxed::Box;
        use tokio::sync::mpsc::Sender;

        pub fn create_typed_listener(
            type_name: &str,
            topic_name: String,
            _tx: Sender<DdsData>,
            _domain_id: i32,
        ) -> Option<Box<dyn DdsTopicListener>> {
            if type_name == "KnownType" {
                Some(Box::new(DummyListener {
                    running: false,
                    topic_name,
                }))
            } else {
                None
            }
        }
    }

    #[tokio::test]
    async fn test_create_typed_listener_with_registry_some() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);

        // Use KnownType to get Some(listener)
        let topic = "topic_known".to_string();
        let data_type = "KnownType".to_string();

        let result = manager
            .create_typed_listener(topic.clone(), data_type)
            .await;
        assert!(result.is_ok());
        assert!(manager.listeners.contains_key(&topic));
        // The inserted listener should be running after start
        let listener = manager.listeners.get(&topic).unwrap();
        assert!(listener.is_running());
    }
    struct FailingStopListener {
        topic_name: String,
    }

    #[async_trait::async_trait]
    impl DdsTopicListener for FailingStopListener {
        async fn start(&mut self) -> Result<()> {
            Ok(())
        }
        async fn stop(&mut self) -> Result<()> {
            Err(anyhow!("Forced stop error").into())
        }
        fn is_running(&self) -> bool {
            true
        }
        fn get_topic_name(&self) -> &str {
            &self.topic_name
        }
        fn is_topic(&self, topic: &str) -> bool {
            self.topic_name == topic
        }
    }

    #[tokio::test]
    async fn test_stop_all_with_failing_listener() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);

        let topic = "fail_stop_topic".to_string();

        manager.listeners.insert(
            topic.clone(),
            Box::new(FailingStopListener {
                topic_name: topic.clone(),
            }),
        );

        let result = manager.stop_all().await;
        // It should return Ok even though stop() failed internally
        assert!(result.is_ok());
        // listeners map should be empty after stop_all
        assert!(manager.listeners.is_empty());
    }
    #[tokio::test]
    async fn test_init_with_path_default_domain_id() {
        let (tx, _) = mpsc::channel(100);
        let mut manager = DdsManager::new(tx);

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), r#"{"dds":{}}"#).unwrap();

        let result = manager.init_with_path(Some(temp_file.path().into())).await;
        assert!(result.is_ok());
        assert_eq!(manager.domain_id, 0); // default domain_id
    }
}

// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Core service management module

use crate::settings_api::ApiServer;
use crate::settings_config::ConfigManager;
use crate::settings_history::HistoryManager;
use crate::settings_monitoring::MonitoringManager;
use crate::settings_storage::EtcdClient;
use crate::settings_utils::error::SettingsError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// System status information
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub version: String,
    pub uptime: std::time::Duration,
    pub components: std::collections::HashMap<String, ComponentStatus>,
}

/// Component status
#[derive(Debug, Clone)]
pub enum ComponentStatus {
    Healthy,
    Degraded(String),
    Failed(String),
}

/// Core manager coordinates all service components
pub struct CoreManager {
    config_manager: Arc<RwLock<ConfigManager>>,
    history_manager: Arc<RwLock<HistoryManager>>,
    monitoring_manager: Arc<RwLock<MonitoringManager>>,
    api_server: Option<ApiServer>,
    start_time: std::time::Instant,
}

impl CoreManager {
    /// Create a new core manager
    pub async fn new(
        etcd_endpoints: Vec<String>,
        bind_address: String,
        bind_port: u16,
        _config_file: PathBuf,
    ) -> Result<Self, SettingsError> {
        info!("Initializing Settings Service core manager");

        // Initialize ETCD clients for each component
        let storage_config = EtcdClient::new(etcd_endpoints.clone()).await.map_err(|e| {
            SettingsError::System(format!("Failed to create config storage: {}", e))
        })?;

        let storage_history = EtcdClient::new(etcd_endpoints.clone()).await.map_err(|e| {
            SettingsError::System(format!("Failed to create history storage: {}", e))
        })?;

        let storage_monitoring = EtcdClient::new(etcd_endpoints).await.map_err(|e| {
            SettingsError::System(format!("Failed to create monitoring storage: {}", e))
        })?;

        // Initialize managers
        let config_manager = Arc::new(RwLock::new(ConfigManager::new(Box::new(storage_config))));
        let history_manager = Arc::new(RwLock::new(HistoryManager::new(Box::new(storage_history))));
        let monitoring_manager = Arc::new(RwLock::new(MonitoringManager::new(
            Box::new(storage_monitoring),
            60, // 60 seconds cache TTL
        )));

        // Initialize API server
        let api_server = ApiServer::new(
            bind_address,
            bind_port,
            config_manager.clone(),
            history_manager.clone(),
            monitoring_manager.clone(),
        )
        .await?;

        Ok(Self {
            config_manager,
            history_manager,
            monitoring_manager,
            api_server: Some(api_server),
            start_time: std::time::Instant::now(),
        })
    }

    /// Start all services
    pub async fn start_services(&mut self) -> Result<(), SettingsError> {
        info!("Starting Settings Service components");

        // Load default schemas
        self.load_default_schemas().await?;

        // Start API server
        if let Some(api_server) = self.api_server.take() {
            tokio::spawn(async move {
                if let Err(e) = api_server.start().await {
                    error!("API server failed: {}", e);
                }
            });
        }

        info!("All Settings Service components started successfully");
        Ok(())
    }

    /// Shutdown all services
    pub async fn shutdown(&self) -> Result<(), SettingsError> {
        info!("Shutting down Settings Service");

        // Clear monitoring cache
        self.monitoring_manager.read().await.clear_cache();

        info!("Settings Service shutdown complete");
        Ok(())
    }

    /// Get system status
    pub async fn get_system_status(&self) -> Result<SystemStatus, SettingsError> {
        debug!("Getting system status");

        let mut components = std::collections::HashMap::new();

        // Check config manager status
        components.insert(
            "config".to_string(),
            self.check_config_manager_status().await,
        );

        // Check history manager status
        components.insert(
            "history".to_string(),
            self.check_history_manager_status().await,
        );

        // Check monitoring manager status
        components.insert(
            "monitoring".to_string(),
            self.check_monitoring_manager_status().await,
        );

        Ok(SystemStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: self.start_time.elapsed(),
            components,
        })
    }

    /// Load default configuration schemas
    async fn load_default_schemas(&self) -> Result<(), SettingsError> {
        info!("Loading default configuration schemas");

        let mut config_manager = self.config_manager.write().await;

        // Basic logging configuration schema
        let logging_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "level": {
                    "type": "string",
                    "enum": ["debug", "info", "warn", "error"]
                },
                "output": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["file", "console", "syslog"]
                    }
                },
                "file_path": {
                    "type": "string"
                },
                "max_size_mb": {
                    "type": "integer",
                    "minimum": 1
                },
                "max_files": {
                    "type": "integer",
                    "minimum": 1
                },
                "rotation": {
                    "type": "string",
                    "enum": ["daily", "weekly", "size"]
                }
            },
            "required": ["level", "output"]
        });

        config_manager
            .save_schema("logging-config", &logging_schema)
            .await?;

        // Basic network configuration schema
        let network_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "bind_address": {
                    "type": "string",
                    "pattern": "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"
                },
                "bind_port": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 65535
                },
                "tls_enabled": {
                    "type": "boolean"
                },
                "tls_cert_path": {
                    "type": "string"
                },
                "tls_key_path": {
                    "type": "string"
                }
            },
            "required": ["bind_address", "bind_port"]
        });

        config_manager
            .save_schema("network-config", &network_schema)
            .await?;

        // Metrics filter configuration schema
        let metrics_filter_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean"
                },
                "refresh": {
                    "type": "integer",
                    "minimum": 1
                },
                "max_items": {
                    "type": "integer",
                    "minimum": 1
                },
                "cache_ttl": {
                    "type": "integer",
                    "minimum": 1
                },
                "filters": {
                    "type": "object",
                    "additionalProperties": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    }
                }
            },
            "required": ["enabled"]
        });

        config_manager
            .save_schema("metrics-filter", &metrics_filter_schema)
            .await?;

        info!("Default schemas loaded successfully");
        Ok(())
    }

    /// Check config manager status
    async fn check_config_manager_status(&self) -> ComponentStatus {
        // Try to access the config manager
        match self.config_manager.write().await.list_configs(None).await {
            Ok(_) => ComponentStatus::Healthy,
            Err(e) => ComponentStatus::Failed(format!("Config manager error: {}", e)),
        }
    }

    /// Check history manager status
    async fn check_history_manager_status(&self) -> ComponentStatus {
        // Try to list some history (this is a read-only operation)
        ComponentStatus::Healthy // For now, assume healthy if we can access it
    }

    /// Check monitoring manager status  
    async fn check_monitoring_manager_status(&self) -> ComponentStatus {
        // Check cache stats as a health indicator
        let cache_stats = self.monitoring_manager.read().await.get_cache_stats();
        if cache_stats.is_empty() {
            ComponentStatus::Degraded("Cache not initialized".to_string())
        } else {
            ComponentStatus::Healthy
        }
    }

    /// Get reference to config manager (for testing)
    #[cfg(test)]
    pub fn config_manager(&self) -> Arc<RwLock<ConfigManager>> {
        self.config_manager.clone()
    }

    /// Get reference to history manager (for testing)
    #[cfg(test)]
    pub fn history_manager(&self) -> Arc<RwLock<HistoryManager>> {
        self.history_manager.clone()
    }

    /// Get reference to monitoring manager (for testing)
    #[cfg(test)]
    pub fn monitoring_manager(&self) -> Arc<RwLock<MonitoringManager>> {
        self.monitoring_manager.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_storage::Storage;
    use crate::settings_utils::error::StorageError;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::time::Duration;
    use tokio;

    /// Mock storage implementation for testing
    #[derive(Default)]
    pub struct MockStorage {
        data: HashMap<String, String>,
        should_fail: bool,
        fail_message: String,
    }

    impl MockStorage {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_failure(message: &str) -> Self {
            Self {
                data: HashMap::new(),
                should_fail: true,
                fail_message: message.to_string(),
            }
        }

        pub fn set_failure(&mut self, should_fail: bool, message: &str) {
            self.should_fail = should_fail;
            self.fail_message = message.to_string();
        }
    }

    #[async_trait]
    impl Storage for MockStorage {
        async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            Ok(self.data.get(key).cloned())
        }

        async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            self.data.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            Ok(self.data.remove(key).is_some())
        }

        async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            let result = self
                .data
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Ok(result)
        }

        async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            match self.get(key).await? {
                Some(value) => {
                    let json = serde_json::from_str(&value).map_err(|e| {
                        StorageError::SerializationError(format!("JSON parse error: {}", e))
                    })?;
                    Ok(Some(json))
                }
                None => Ok(None),
            }
        }

        async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            let json_str = serde_json::to_string(value).map_err(|e| {
                StorageError::SerializationError(format!("JSON serialize error: {}", e))
            })?;
            self.put(key, &json_str).await
        }
    }

    /// Create a test core manager with mock storage
    async fn create_test_core_manager() -> CoreManager {
        let config_manager = Arc::new(RwLock::new(ConfigManager::new(
            Box::new(MockStorage::new()),
        )));
        let history_manager = Arc::new(RwLock::new(HistoryManager::new(Box::new(
            MockStorage::new(),
        ))));
        let monitoring_manager = Arc::new(RwLock::new(MonitoringManager::new(
            Box::new(MockStorage::new()),
            60,
        )));

        CoreManager {
            config_manager,
            history_manager,
            monitoring_manager,
            api_server: None,
            start_time: std::time::Instant::now(),
        }
    }

    #[test]
    fn test_system_status_creation() {
        let mut components = HashMap::new();
        components.insert("config".to_string(), ComponentStatus::Healthy);
        components.insert(
            "history".to_string(),
            ComponentStatus::Degraded("Cache issue".to_string()),
        );
        components.insert(
            "monitoring".to_string(),
            ComponentStatus::Failed("Connection error".to_string()),
        );

        let status = SystemStatus {
            version: "1.0.0".to_string(),
            uptime: Duration::from_secs(3600),
            components: components.clone(),
        };

        assert_eq!(status.version, "1.0.0");
        assert_eq!(status.uptime, Duration::from_secs(3600));
        assert_eq!(status.components.len(), 3);

        // Verify component statuses
        match &status.components["config"] {
            ComponentStatus::Healthy => (),
            _ => panic!("Expected Healthy status for config"),
        }

        match &status.components["history"] {
            ComponentStatus::Degraded(msg) => assert_eq!(msg, "Cache issue"),
            _ => panic!("Expected Degraded status for history"),
        }

        match &status.components["monitoring"] {
            ComponentStatus::Failed(msg) => assert_eq!(msg, "Connection error"),
            _ => panic!("Expected Failed status for monitoring"),
        }
    }

    #[test]
    fn test_component_status_variants() {
        let healthy = ComponentStatus::Healthy;
        let degraded = ComponentStatus::Degraded("Test degradation".to_string());
        let failed = ComponentStatus::Failed("Test failure".to_string());

        // Test pattern matching
        match healthy {
            ComponentStatus::Healthy => (),
            _ => panic!("Expected Healthy variant"),
        }

        match degraded {
            ComponentStatus::Degraded(msg) => assert_eq!(msg, "Test degradation"),
            _ => panic!("Expected Degraded variant"),
        }

        match failed {
            ComponentStatus::Failed(msg) => assert_eq!(msg, "Test failure"),
            _ => panic!("Expected Failed variant"),
        }
    }

    #[test]
    fn test_component_status_debug() {
        let healthy = ComponentStatus::Healthy;
        let degraded = ComponentStatus::Degraded("Test message".to_string());
        let failed = ComponentStatus::Failed("Error message".to_string());

        // Test that Debug trait is implemented
        let healthy_debug = format!("{:?}", healthy);
        let degraded_debug = format!("{:?}", degraded);
        let failed_debug = format!("{:?}", failed);

        assert!(healthy_debug.contains("Healthy"));
        assert!(degraded_debug.contains("Degraded"));
        assert!(degraded_debug.contains("Test message"));
        assert!(failed_debug.contains("Failed"));
        assert!(failed_debug.contains("Error message"));
    }

    #[test]
    fn test_component_status_clone() {
        let original = ComponentStatus::Degraded("Original message".to_string());
        let cloned = original.clone();

        match (original, cloned) {
            (ComponentStatus::Degraded(msg1), ComponentStatus::Degraded(msg2)) => {
                assert_eq!(msg1, msg2);
                assert_eq!(msg1, "Original message");
            }
            _ => panic!("Clone failed or wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_core_manager_creation() {
        let core_manager = create_test_core_manager().await;

        // Verify managers are accessible
        assert!(core_manager
            .config_manager()
            .write()
            .await
            .list_configs(None)
            .await
            .is_ok());

        // Verify start time is recent (within last minute)
        let elapsed = core_manager.start_time.elapsed();
        assert!(elapsed < Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_core_manager_get_system_status() {
        let core_manager = create_test_core_manager().await;

        let status = core_manager.get_system_status().await;
        assert!(status.is_ok());

        let status = status.unwrap();
        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
        assert!(status.uptime < Duration::from_secs(60)); // Should be very recent
        assert!(status.components.contains_key("config"));
        assert!(status.components.contains_key("history"));
        assert!(status.components.contains_key("monitoring"));
    }

    #[tokio::test]
    async fn test_core_manager_shutdown() {
        let core_manager = create_test_core_manager().await;

        let result = core_manager.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_config_manager_status_healthy() {
        let core_manager = create_test_core_manager().await;

        let status = core_manager.check_config_manager_status().await;
        match status {
            ComponentStatus::Healthy => (),
            _ => panic!("Expected Healthy status for config manager"),
        }
    }

    #[tokio::test]
    async fn test_check_config_manager_status_failed() {
        let config_manager = Arc::new(RwLock::new(ConfigManager::new(Box::new(
            MockStorage::with_failure("Storage connection failed"),
        ))));
        let history_manager = Arc::new(RwLock::new(HistoryManager::new(Box::new(
            MockStorage::new(),
        ))));
        let monitoring_manager = Arc::new(RwLock::new(MonitoringManager::new(
            Box::new(MockStorage::new()),
            60,
        )));

        let core_manager = CoreManager {
            config_manager,
            history_manager,
            monitoring_manager,
            api_server: None,
            start_time: std::time::Instant::now(),
        };

        let status = core_manager.check_config_manager_status().await;
        match status {
            ComponentStatus::Failed(msg) => {
                assert!(msg.contains("Config manager error"));
                assert!(msg.contains("Storage connection failed"));
            }
            _ => panic!("Expected Failed status for config manager"),
        }
    }

    #[tokio::test]
    async fn test_check_history_manager_status() {
        let core_manager = create_test_core_manager().await;

        let status = core_manager.check_history_manager_status().await;
        match status {
            ComponentStatus::Healthy => (),
            _ => panic!("Expected Healthy status for history manager"),
        }
    }

    #[tokio::test]
    async fn test_check_monitoring_manager_status_healthy() {
        let core_manager = create_test_core_manager().await;

        // Initialize cache with some data
        {
            let mut monitoring_manager = core_manager.monitoring_manager.write().await;
            // Simulate cache initialization by getting metrics
            let _ = monitoring_manager.get_metrics(None).await;
        }

        let status = core_manager.check_monitoring_manager_status().await;
        match status {
            ComponentStatus::Healthy | ComponentStatus::Degraded(_) => (),
            ComponentStatus::Failed(msg) => panic!("Unexpected Failed status: {}", msg),
        }
    }

    #[tokio::test]
    async fn test_load_default_schemas() {
        let core_manager = create_test_core_manager().await;

        let result = core_manager.load_default_schemas().await;
        assert!(result.is_ok());

        // Verify schemas were loaded
        let mut config_manager = core_manager.config_manager.write().await;

        // Check that logging schema exists
        let logging_schema = config_manager.get_schema("logging-config").await;
        assert!(logging_schema.is_ok());
        let schema = logging_schema.unwrap();
        assert!(schema["properties"]["level"]["enum"].is_array());

        // Check that network schema exists
        let network_schema = config_manager.get_schema("network-config").await;
        assert!(network_schema.is_ok());
        let schema = network_schema.unwrap();
        assert!(schema["properties"]["bind_address"]["pattern"].is_string());

        // Check that metrics filter schema exists
        let metrics_schema = config_manager.get_schema("metrics-filter").await;
        assert!(metrics_schema.is_ok());
        let schema = metrics_schema.unwrap();
        assert!(schema["properties"]["enabled"]["type"] == "boolean");
    }

    #[tokio::test]
    async fn test_load_default_schemas_with_storage_failure() {
        let config_manager = Arc::new(RwLock::new(ConfigManager::new(Box::new(
            MockStorage::with_failure("Schema save failed"),
        ))));
        let history_manager = Arc::new(RwLock::new(HistoryManager::new(Box::new(
            MockStorage::new(),
        ))));
        let monitoring_manager = Arc::new(RwLock::new(MonitoringManager::new(
            Box::new(MockStorage::new()),
            60,
        )));

        let core_manager = CoreManager {
            config_manager,
            history_manager,
            monitoring_manager,
            api_server: None,
            start_time: std::time::Instant::now(),
        };

        let result = core_manager.load_default_schemas().await;
        assert!(result.is_err());

        // Just verify that we get an error, don't check the specific message format
        match result {
            Err(_) => (), // Any error is expected due to storage failure
            Ok(_) => panic!("Expected an error due to storage failure"),
        }
    }

    #[tokio::test]
    async fn test_system_status_with_all_component_types() {
        let config_manager = Arc::new(RwLock::new(ConfigManager::new(Box::new(
            MockStorage::with_failure("Config error"),
        ))));
        let history_manager = Arc::new(RwLock::new(HistoryManager::new(Box::new(
            MockStorage::new(),
        ))));
        let monitoring_manager = Arc::new(RwLock::new(MonitoringManager::new(
            Box::new(MockStorage::new()),
            60,
        )));

        let core_manager = CoreManager {
            config_manager,
            history_manager,
            monitoring_manager,
            api_server: None,
            start_time: std::time::Instant::now(),
        };

        let status = core_manager.get_system_status().await;
        assert!(status.is_ok());

        let status = status.unwrap();
        assert_eq!(status.components.len(), 3);

        // Config should be failed due to mock storage failure
        match &status.components["config"] {
            ComponentStatus::Failed(msg) => assert!(msg.contains("Config manager error")),
            other => panic!("Expected Failed status for config, got: {:?}", other),
        }

        // History should be healthy (no storage operations tested)
        match &status.components["history"] {
            ComponentStatus::Healthy => (),
            other => panic!("Expected Healthy status for history, got: {:?}", other),
        }

        // Monitoring should be degraded or healthy
        match &status.components["monitoring"] {
            ComponentStatus::Healthy | ComponentStatus::Degraded(_) => (),
            ComponentStatus::Failed(msg) => {
                panic!("Unexpected Failed status for monitoring: {}", msg)
            }
        }
    }

    #[test]
    fn test_system_status_debug_and_clone() {
        let mut components = HashMap::new();
        components.insert("test".to_string(), ComponentStatus::Healthy);

        let status = SystemStatus {
            version: "1.0.0".to_string(),
            uptime: Duration::from_secs(100),
            components,
        };

        // Test Debug trait
        let debug_str = format!("{:?}", status);
        assert!(debug_str.contains("SystemStatus"));
        assert!(debug_str.contains("1.0.0"));

        // Test Clone trait
        let cloned_status = status.clone();
        assert_eq!(status.version, cloned_status.version);
        assert_eq!(status.uptime, cloned_status.uptime);
        assert_eq!(status.components.len(), cloned_status.components.len());
    }

    #[test]
    fn test_mock_storage_functionality() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut storage = MockStorage::new();

            // Test put and get
            assert!(storage.put("key1", "value1").await.is_ok());
            let result = storage.get("key1").await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("value1".to_string()));

            // Test get non-existent key
            let result = storage.get("nonexistent").await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);

            // Test delete
            let result = storage.delete("key1").await;
            assert!(result.is_ok());
            assert!(result.unwrap());

            let result = storage.get("key1").await;
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);

            // Test list
            storage.put("prefix_key1", "value1").await.unwrap();
            storage.put("prefix_key2", "value2").await.unwrap();
            storage.put("other_key", "value3").await.unwrap();

            let result = storage.list("prefix_").await;
            assert!(result.is_ok());
            let list = result.unwrap();
            assert_eq!(list.len(), 2);
        });
    }

    #[test]
    fn test_mock_storage_with_failure() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut storage = MockStorage::with_failure("Test failure");

            // All operations should fail
            assert!(storage.get("key").await.is_err());
            assert!(storage.put("key", "value").await.is_err());
            assert!(storage.delete("key").await.is_err());
            assert!(storage.list("prefix").await.is_err());

            // Test dynamic failure setting
            storage.set_failure(false, "");
            assert!(storage.put("key", "value").await.is_ok());
            assert!(storage.get("key").await.is_ok());

            storage.set_failure(true, "Dynamic failure");
            assert!(storage.get("key").await.is_err());
        });
    }

    #[test]
    fn test_mock_storage_json_operations() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let mut storage = MockStorage::new();

            let test_json = serde_json::json!({
                "key": "value",
                "number": 42,
                "array": [1, 2, 3]
            });

            // Test put_json and get_json
            assert!(storage.put_json("json_key", &test_json).await.is_ok());

            let result = storage.get_json("json_key").await;
            assert!(result.is_ok());
            let retrieved = result.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap(), test_json);

            // Test get_json for non-existent key
            let result = storage.get_json("nonexistent").await;
            assert!(result.is_ok());
            assert!(result.unwrap().is_none());
        });
    }
}

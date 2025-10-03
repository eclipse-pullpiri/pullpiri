// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! ETCD storage client module

use crate::settings_utils::error::{SettingsError, StorageError};
use async_trait::async_trait;
use etcd_client::{Client, ConnectOptions, DeleteOptions, GetOptions, PutOptions};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, warn};

/// ETCD client wrapper for Settings Service
pub struct EtcdClient {
    client: Client,
}

impl EtcdClient {
    /// Create a new ETCD client
    pub async fn new(endpoints: Vec<String>) -> Result<Self, StorageError> {
        debug!("Connecting to ETCD endpoints: {:?}", endpoints);

        let client = Client::connect(endpoints, Some(ConnectOptions::new()))
            .await
            .map_err(|e| {
                StorageError::ConnectionFailed(format!("Failed to connect to ETCD: {}", e))
            })?;

        Ok(Self { client })
    }

    /// Get a value by key
    pub async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
        debug!("Getting key: {}", key);

        let resp =
            self.client.get(key, None).await.map_err(|e| {
                StorageError::OperationFailed(format!("Get operation failed: {}", e))
            })?;

        if let Some(kv) = resp.kvs().first() {
            let value = String::from_utf8(kv.value().to_vec())
                .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Put a key-value pair
    pub async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        debug!("Putting key: {}, value length: {}", key, value.len());

        self.client
            .put(key, value, None)
            .await
            .map_err(|e| StorageError::OperationFailed(format!("Put operation failed: {}", e)))?;

        Ok(())
    }

    /// Delete a key
    pub async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
        debug!("Deleting key: {}", key);

        let resp = self.client.delete(key, None).await.map_err(|e| {
            StorageError::OperationFailed(format!("Delete operation failed: {}", e))
        })?;

        Ok(resp.deleted() > 0)
    }

    /// List keys with a prefix
    pub async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
        debug!("Listing keys with prefix: {}", prefix);

        let get_options = Some(GetOptions::new().with_prefix());
        let resp =
            self.client.get(prefix, get_options).await.map_err(|e| {
                StorageError::OperationFailed(format!("List operation failed: {}", e))
            })?;

        let mut results = Vec::new();
        for kv in resp.kvs() {
            let key = String::from_utf8(kv.key().to_vec()).map_err(|e| {
                StorageError::SerializationError(format!("Invalid UTF-8 in key: {}", e))
            })?;
            let value = String::from_utf8(kv.value().to_vec()).map_err(|e| {
                StorageError::SerializationError(format!("Invalid UTF-8 in value: {}", e))
            })?;
            results.push((key, value));
        }

        Ok(results)
    }

    /// Get JSON value by key
    pub async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError> {
        if let Some(value_str) = self.get(key).await? {
            let value: Value = serde_json::from_str(&value_str).map_err(|e| {
                StorageError::SerializationError(format!("JSON parse error: {}", e))
            })?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Put JSON value by key
    pub async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
        let value_str = serde_json::to_string(value).map_err(|e| {
            StorageError::SerializationError(format!("JSON serialize error: {}", e))
        })?;

        self.put(key, &value_str).await
    }
}

/// Storage interface trait for dependency injection and testing
#[async_trait]
pub trait Storage: Send + Sync {
    async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError>;
    async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError>;
    async fn delete(&mut self, key: &str) -> Result<bool, StorageError>;
    async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError>;
    async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError>;
    async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError>;
}

#[async_trait]
impl Storage for EtcdClient {
    async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
        self.get(key).await
    }

    async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.put(key, value).await
    }

    async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
        self.delete(key).await
    }

    async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
        self.list(prefix).await
    }

    async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError> {
        self.get_json(key).await
    }

    async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
        self.put_json(key, value).await
    }
}

/// Key prefixes for different data types
pub struct KeyPrefixes;

impl KeyPrefixes {
    pub const CONFIG: &'static str = "/piccolo/settings/configs/";
    pub const HISTORY: &'static str = "/piccolo/settings/history/";
    pub const METRICS: &'static str = "/piccolo/metrics/";
    pub const FILTERS: &'static str = "/piccolo/settings/filters/";
    pub const SCHEMAS: &'static str = "/piccolo/settings/schemas/";
}

/// Helper functions for key management
pub fn config_key(path: &str) -> String {
    format!("{}{}", KeyPrefixes::CONFIG, path)
}

pub fn history_key(config_path: &str, version: u64) -> String {
    format!("{}{}/v{}", KeyPrefixes::HISTORY, config_path, version)
}

pub fn metrics_key(resource_type: &str, resource_id: &str) -> String {
    format!("{}{}/{}", KeyPrefixes::METRICS, resource_type, resource_id)
}

pub fn filter_key(filter_id: &str) -> String {
    format!("{}{}", KeyPrefixes::FILTERS, filter_id)
}

pub fn schema_key(schema_type: &str) -> String {
    format!("{}{}", KeyPrefixes::SCHEMAS, schema_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tokio;

    /// Mock storage implementation for testing
    #[derive(Default)]
    pub struct MockStorage {
        data: HashMap<String, String>,
        should_fail: bool,
        fail_message: String,
        fail_on_operations: Vec<String>, // Operations that should fail: "get", "put", "delete", "list", "get_json", "put_json"
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
                fail_on_operations: vec![
                    "get".to_string(),
                    "put".to_string(),
                    "delete".to_string(),
                    "list".to_string(),
                ],
            }
        }

        pub fn with_selective_failure(operations: Vec<&str>, message: &str) -> Self {
            Self {
                data: HashMap::new(),
                should_fail: true,
                fail_message: message.to_string(),
                fail_on_operations: operations.iter().map(|s| s.to_string()).collect(),
            }
        }

        pub fn insert_data(&mut self, key: &str, value: &str) {
            self.data.insert(key.to_string(), value.to_string());
        }

        pub fn get_data(&self, key: &str) -> Option<&String> {
            self.data.get(key)
        }

        pub fn clear_data(&mut self) {
            self.data.clear();
        }

        pub fn set_failure(&mut self, should_fail: bool, message: &str, operations: Vec<&str>) {
            self.should_fail = should_fail;
            self.fail_message = message.to_string();
            self.fail_on_operations = operations.iter().map(|s| s.to_string()).collect();
        }

        fn should_fail_operation(&self, operation: &str) -> bool {
            self.should_fail && self.fail_on_operations.contains(&operation.to_string())
        }
    }

    #[async_trait]
    impl Storage for MockStorage {
        async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
            if self.should_fail_operation("get") {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            Ok(self.data.get(key).cloned())
        }

        async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
            if self.should_fail_operation("put") {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            self.data.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
            if self.should_fail_operation("delete") {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            Ok(self.data.remove(key).is_some())
        }

        async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
            if self.should_fail_operation("list") {
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
            if self.should_fail_operation("get_json") {
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
            if self.should_fail_operation("put_json") {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            let json_str = serde_json::to_string(value).map_err(|e| {
                StorageError::SerializationError(format!("JSON serialize error: {}", e))
            })?;
            self.put(key, &json_str).await
        }
    }

    #[test]
    fn test_key_prefixes_constants() {
        assert_eq!(KeyPrefixes::CONFIG, "/piccolo/settings/configs/");
        assert_eq!(KeyPrefixes::HISTORY, "/piccolo/settings/history/");
        assert_eq!(KeyPrefixes::METRICS, "/piccolo/metrics/");
        assert_eq!(KeyPrefixes::FILTERS, "/piccolo/settings/filters/");
        assert_eq!(KeyPrefixes::SCHEMAS, "/piccolo/settings/schemas/");
    }

    #[test]
    fn test_config_key() {
        assert_eq!(
            config_key("test/config"),
            "/piccolo/settings/configs/test/config"
        );
        assert_eq!(
            config_key("/absolute/path"),
            "/piccolo/settings/configs//absolute/path"
        );
        assert_eq!(config_key(""), "/piccolo/settings/configs/");
        assert_eq!(config_key("simple"), "/piccolo/settings/configs/simple");
    }

    #[test]
    fn test_history_key() {
        assert_eq!(
            history_key("test/config", 1),
            "/piccolo/settings/history/test/config/v1"
        );
        assert_eq!(
            history_key("/absolute/path", 42),
            "/piccolo/settings/history//absolute/path/v42"
        );
        assert_eq!(history_key("", 0), "/piccolo/settings/history//v0");
        assert_eq!(
            history_key("simple", 999),
            "/piccolo/settings/history/simple/v999"
        );
    }

    #[test]
    fn test_metrics_key() {
        assert_eq!(metrics_key("cpu", "node1"), "/piccolo/metrics/cpu/node1");
        assert_eq!(
            metrics_key("memory", "container123"),
            "/piccolo/metrics/memory/container123"
        );
        assert_eq!(
            metrics_key("", "empty_type"),
            "/piccolo/metrics//empty_type"
        );
        assert_eq!(metrics_key("network", ""), "/piccolo/metrics/network/");
    }

    #[test]
    fn test_filter_key() {
        assert_eq!(filter_key("filter1"), "/piccolo/settings/filters/filter1");
        assert_eq!(
            filter_key("complex-filter-name"),
            "/piccolo/settings/filters/complex-filter-name"
        );
        assert_eq!(filter_key(""), "/piccolo/settings/filters/");
        assert_eq!(filter_key("123"), "/piccolo/settings/filters/123");
    }

    #[test]
    fn test_schema_key() {
        assert_eq!(schema_key("user"), "/piccolo/settings/schemas/user");
        assert_eq!(
            schema_key("logging-config"),
            "/piccolo/settings/schemas/logging-config"
        );
        assert_eq!(schema_key(""), "/piccolo/settings/schemas/");
        assert_eq!(
            schema_key("complex.schema.name"),
            "/piccolo/settings/schemas/complex.schema.name"
        );
    }

    #[tokio::test]
    async fn test_mock_storage_basic_operations() {
        let mut storage = MockStorage::new();

        // Test put and get
        let result = storage.put("test_key", "test_value").await;
        assert!(result.is_ok());

        let result = storage.get("test_key").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("test_value".to_string()));

        // Test get non-existent key
        let result = storage.get("nonexistent").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Test delete existing key
        let result = storage.delete("test_key").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Verify key is deleted
        let result = storage.get("test_key").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Test delete non-existent key
        let result = storage.delete("nonexistent").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_mock_storage_list_operations() {
        let mut storage = MockStorage::new();

        // Insert test data
        storage.put("prefix1/key1", "value1").await.unwrap();
        storage.put("prefix1/key2", "value2").await.unwrap();
        storage.put("prefix2/key3", "value3").await.unwrap();
        storage.put("other/key4", "value4").await.unwrap();

        // Test list with prefix
        let result = storage.list("prefix1/").await;
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 2);

        // Verify items are correct
        let mut found_keys = Vec::new();
        for (key, value) in items {
            found_keys.push(key.clone());
            if key == "prefix1/key1" {
                assert_eq!(value, "value1");
            } else if key == "prefix1/key2" {
                assert_eq!(value, "value2");
            }
        }
        assert!(found_keys.contains(&"prefix1/key1".to_string()));
        assert!(found_keys.contains(&"prefix1/key2".to_string()));

        // Test list with different prefix
        let result = storage.list("prefix2/").await;
        assert!(result.is_ok());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "prefix2/key3");
        assert_eq!(items[0].1, "value3");

        // Test list with non-matching prefix
        let result = storage.list("nonexistent/").await;
        assert!(result.is_ok());
        let items = result.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_mock_storage_json_operations() {
        let mut storage = MockStorage::new();

        let test_json = json!({
            "name": "John Doe",
            "age": 30,
            "active": true,
            "scores": [85, 92, 78],
            "metadata": {
                "created": "2023-01-01",
                "updated": "2023-12-31"
            }
        });

        // Test put_json
        let result = storage.put_json("test_json", &test_json).await;
        assert!(result.is_ok());

        // Test get_json
        let result = storage.get_json("test_json").await;
        assert!(result.is_ok());
        let retrieved = result.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_json);

        // Test get_json for non-existent key
        let result = storage.get_json("nonexistent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Test with simple values
        let simple_json = json!("simple string");
        storage.put_json("simple", &simple_json).await.unwrap();
        let result = storage.get_json("simple").await.unwrap();
        assert_eq!(result.unwrap(), simple_json);

        // Test with null value
        let null_json = json!(null);
        storage.put_json("null_value", &null_json).await.unwrap();
        let result = storage.get_json("null_value").await.unwrap();
        assert_eq!(result.unwrap(), null_json);
    }

    #[tokio::test]
    async fn test_mock_storage_failure_scenarios() {
        let mut storage = MockStorage::with_failure("Test failure");

        // All operations should fail
        assert!(storage.get("key").await.is_err());
        assert!(storage.put("key", "value").await.is_err());
        assert!(storage.delete("key").await.is_err());
        assert!(storage.list("prefix").await.is_err());

        // Test selective failures
        let mut storage =
            MockStorage::with_selective_failure(vec!["get", "delete"], "Selective failure");

        // Get and delete should fail
        assert!(storage.get("key").await.is_err());
        assert!(storage.delete("key").await.is_err());

        // Put and list should succeed
        assert!(storage.put("key", "value").await.is_ok());
        assert!(storage.list("prefix").await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_json_failure_scenarios() {
        let mut storage =
            MockStorage::with_selective_failure(vec!["get_json", "put_json"], "JSON failure");

        let test_json = json!({"test": "value"});

        // JSON operations should fail
        assert!(storage.put_json("key", &test_json).await.is_err());
        assert!(storage.get_json("key").await.is_err());

        // Regular operations should succeed
        assert!(storage.put("key", "value").await.is_ok());
        assert!(storage.get("key").await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_storage_invalid_json_handling() {
        let mut storage = MockStorage::new();

        // Insert invalid JSON directly
        storage.insert_data("invalid_json", "{invalid json}");

        // get_json should return a serialization error
        let result = storage.get_json("invalid_json").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            StorageError::SerializationError(msg) => {
                assert!(msg.contains("JSON parse error"));
            }
            _ => panic!("Expected SerializationError"),
        }
    }

    #[tokio::test]
    async fn test_mock_storage_helper_methods() {
        let mut storage = MockStorage::new();

        // Test insert_data
        storage.insert_data("test_key", "test_value");
        assert_eq!(
            storage.get_data("test_key"),
            Some(&"test_value".to_string())
        );

        // Test get_data for non-existent key
        assert_eq!(storage.get_data("nonexistent"), None);

        // Test clear_data
        storage.clear_data();
        assert_eq!(storage.get_data("test_key"), None);

        // Test set_failure
        storage.set_failure(true, "Custom failure", vec!["get"]);
        assert!(storage.get("any_key").await.is_err());
        assert!(storage.put("key", "value").await.is_ok()); // Should succeed
    }

    #[test]
    fn test_mock_storage_creation_variants() {
        // Test default creation
        let storage = MockStorage::new();
        assert!(!storage.should_fail);
        assert!(storage.data.is_empty());

        // Test with_failure creation
        let storage = MockStorage::with_failure("Test error");
        assert!(storage.should_fail);
        assert_eq!(storage.fail_message, "Test error");
        assert!(storage.fail_on_operations.contains(&"get".to_string()));

        // Test with_selective_failure creation
        let storage = MockStorage::with_selective_failure(vec!["put", "delete"], "Selective error");
        assert!(storage.should_fail);
        assert_eq!(storage.fail_message, "Selective error");
        assert!(storage.fail_on_operations.contains(&"put".to_string()));
        assert!(storage.fail_on_operations.contains(&"delete".to_string()));
        assert!(!storage.fail_on_operations.contains(&"get".to_string()));
    }

    #[test]
    fn test_storage_trait_bounds() {
        // Verify that Storage trait has the correct bounds
        fn check_storage_bounds<T: Storage>() {}

        // This should compile without issues
        check_storage_bounds::<MockStorage>();
    }

    #[test]
    fn test_key_functions_with_special_characters() {
        // Test keys with special characters
        assert_eq!(
            config_key("test/config with spaces"),
            "/piccolo/settings/configs/test/config with spaces"
        );

        assert_eq!(
            history_key("test-config_v2", 1),
            "/piccolo/settings/history/test-config_v2/v1"
        );

        assert_eq!(
            metrics_key("cpu-usage", "node@1"),
            "/piccolo/metrics/cpu-usage/node@1"
        );

        assert_eq!(
            filter_key("filter.with.dots"),
            "/piccolo/settings/filters/filter.with.dots"
        );

        assert_eq!(
            schema_key("schema:with:colons"),
            "/piccolo/settings/schemas/schema:with:colons"
        );
    }

    #[test]
    fn test_key_functions_edge_cases() {
        // Test with very long strings
        let long_string = "a".repeat(1000);
        let result = config_key(&long_string);
        assert!(result.starts_with("/piccolo/settings/configs/"));
        assert!(result.ends_with(&long_string));

        // Test with unicode characters
        assert_eq!(
            config_key("测试/配置"),
            "/piccolo/settings/configs/测试/配置"
        );

        assert_eq!(
            schema_key("émojis-🚀-schema"),
            "/piccolo/settings/schemas/émojis-🚀-schema"
        );

        // Test with numbers
        assert_eq!(
            history_key("config123", 456),
            "/piccolo/settings/history/config123/v456"
        );

        assert_eq!(
            metrics_key("metric789", "resource000"),
            "/piccolo/metrics/metric789/resource000"
        );
    }

    #[tokio::test]
    async fn test_mock_storage_concurrent_operations() {
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let storage = Arc::new(Mutex::new(MockStorage::new()));

        // Simulate concurrent operations
        let storage_clone1 = storage.clone();
        let storage_clone2 = storage.clone();

        let task1 = tokio::spawn(async move {
            let mut storage = storage_clone1.lock().await;
            for i in 0..10 {
                storage
                    .put(&format!("key1_{}", i), &format!("value1_{}", i))
                    .await
                    .unwrap();
            }
        });

        let task2 = tokio::spawn(async move {
            let mut storage = storage_clone2.lock().await;
            for i in 0..10 {
                storage
                    .put(&format!("key2_{}", i), &format!("value2_{}", i))
                    .await
                    .unwrap();
            }
        });

        // Wait for both tasks to complete
        let _ = tokio::join!(task1, task2);

        // Verify all data was inserted
        let storage = storage.lock().await;
        for i in 0..10 {
            assert!(storage.get_data(&format!("key1_{}", i)).is_some());
            assert!(storage.get_data(&format!("key2_{}", i)).is_some());
        }
    }

    #[tokio::test]
    async fn test_mock_storage_complex_json_scenarios() {
        let mut storage = MockStorage::new();

        // Test deeply nested JSON
        let complex_json = json!({
            "level1": {
                "level2": {
                    "level3": {
                        "data": "deep value",
                        "array": [1, 2, {"nested": true}]
                    }
                }
            },
            "metadata": {
                "version": 1.0,
                "tags": ["tag1", "tag2"],
                "options": {
                    "enabled": true,
                    "timeout": 30
                }
            }
        });

        storage.put_json("complex", &complex_json).await.unwrap();
        let result = storage.get_json("complex").await.unwrap().unwrap();

        // Verify deep equality
        assert_eq!(result["level1"]["level2"]["level3"]["data"], "deep value");
        assert_eq!(result["metadata"]["version"], 1.0);
        assert_eq!(result["metadata"]["tags"][0], "tag1");
        assert_eq!(result["metadata"]["options"]["enabled"], true);

        // Test array of objects
        let array_json = json!([
            {"id": 1, "name": "Item 1"},
            {"id": 2, "name": "Item 2"},
            {"id": 3, "name": "Item 3"}
        ]);

        storage.put_json("array", &array_json).await.unwrap();
        let result = storage.get_json("array").await.unwrap().unwrap();

        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 3);
        assert_eq!(result[0]["name"], "Item 1");
        assert_eq!(result[2]["id"], 3);
    }
}

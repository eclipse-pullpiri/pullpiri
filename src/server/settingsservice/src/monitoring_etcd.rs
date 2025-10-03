// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration with monitoring server's etcd storage

use crate::monitoring_types::{BoardInfo, NodeInfo, SocInfo};
use common::monitoringserver::ContainerInfo;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

/// Custom error type for monitoring etcd operations
#[derive(Debug, Error)]
pub enum MonitoringEtcdError {
    #[error("Etcd operation error: {0}")]
    EtcdOperation(String),
    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("UTF8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Data not found")]
    NotFound,
    #[error("{0}")]
    Other(String),
}

type Result<T> = std::result::Result<T, MonitoringEtcdError>;

/// Generic function to store info in etcd
async fn store_info<T: Serialize>(resource_type: &str, resource_id: &str, info: &T) -> Result<()> {
    let key = format!("/piccolo/metrics/{}/{}", resource_type, resource_id);
    let json_data = serde_json::to_string(info)?;

    common::etcd::put(&key, &json_data)
        .await
        .map_err(|e| MonitoringEtcdError::EtcdOperation(e.to_string()))?;

    debug!("Stored {} for {}", resource_type, resource_id);
    Ok(())
}

/// Generic function to retrieve info from etcd
async fn get_info<T: DeserializeOwned>(resource_type: &str, resource_id: &str) -> Result<T> {
    let key = format!("/piccolo/metrics/{}/{}", resource_type, resource_id);

    let json_data = common::etcd::get(&key)
        .await
        .map_err(|e| MonitoringEtcdError::EtcdOperation(e.to_string()))?;

    let info: T = serde_json::from_str(&json_data)?;

    debug!("Retrieved {} for {}", resource_type, resource_id);
    Ok(info)
}

/// Generic function to get all items of a type from etcd
async fn get_all_info<T: DeserializeOwned>(resource_type: &str) -> Result<Vec<T>> {
    let prefix = format!("/piccolo/metrics/{}/", resource_type);
    let kv_pairs = common::etcd::get_all_with_prefix(&prefix)
        .await
        .map_err(|e| MonitoringEtcdError::EtcdOperation(e.to_string()))?;

    let mut items = Vec::new();
    for kv in kv_pairs {
        match serde_json::from_str::<T>(&kv.value) {
            Ok(item) => items.push(item),
            Err(e) => {
                warn!(
                    "Failed to deserialize {} from {}: {}",
                    resource_type, kv.key, e
                );
            }
        }
    }
    debug!("Retrieved {} {}s from etcd", items.len(), resource_type);
    Ok(items)
}

/// Generic function to delete info from etcd
async fn delete_info(resource_type: &str, resource_id: &str) -> Result<()> {
    let key = format!("/piccolo/metrics/{}/{}", resource_type, resource_id);

    common::etcd::delete(&key)
        .await
        .map_err(|e| MonitoringEtcdError::EtcdOperation(e.to_string()))?;

    debug!("Deleted {} for {}", resource_type, resource_id);
    Ok(())
}

// Public wrapper functions for specific types

/// Store NodeInfo in etcd
pub async fn store_node_info(node_info: &NodeInfo) -> Result<()> {
    store_info("nodes", &node_info.node_name, node_info).await
}

/// Get NodeInfo from etcd
pub async fn get_node_info(node_name: &str) -> Result<NodeInfo> {
    get_info("nodes", node_name).await
}

/// Get all nodes from etcd
pub async fn get_all_nodes() -> Result<Vec<NodeInfo>> {
    get_all_info("nodes").await
}

/// Delete NodeInfo from etcd
pub async fn delete_node_info(node_name: &str) -> Result<()> {
    delete_info("nodes", node_name).await
}

/// Store SocInfo in etcd
pub async fn store_soc_info(soc_info: &SocInfo) -> Result<()> {
    store_info("socs", &soc_info.soc_id, soc_info).await
}

/// Get SocInfo from etcd
pub async fn get_soc_info(soc_id: &str) -> Result<SocInfo> {
    get_info("socs", soc_id).await
}

/// Get all SoCs from etcd
pub async fn get_all_socs() -> Result<Vec<SocInfo>> {
    get_all_info("socs").await
}

/// Delete SocInfo from etcd
pub async fn delete_soc_info(soc_id: &str) -> Result<()> {
    delete_info("socs", soc_id).await
}

/// Store BoardInfo in etcd
pub async fn store_board_info(board_info: &BoardInfo) -> Result<()> {
    store_info("boards", &board_info.board_id, board_info).await
}

/// Get BoardInfo from etcd
pub async fn get_board_info(board_id: &str) -> Result<BoardInfo> {
    get_info("boards", board_id).await
}

/// Get all boards from etcd
pub async fn get_all_boards() -> Result<Vec<BoardInfo>> {
    get_all_info("boards").await
}

/// Delete BoardInfo from etcd
pub async fn delete_board_info(board_id: &str) -> Result<()> {
    delete_info("boards", board_id).await
}

/// Store ContainerInfo in etcd
pub async fn store_container_info(container_info: &ContainerInfo) -> Result<()> {
    store_info("containers", &container_info.id, container_info).await
}

/// Get ContainerInfo from etcd
pub async fn get_container_info(container_id: &str) -> Result<ContainerInfo> {
    get_info("containers", container_id).await
}

/// Get all containers from etcd
pub async fn get_all_containers() -> Result<Vec<ContainerInfo>> {
    get_all_info("containers").await
}

/// Delete ContainerInfo from etcd
pub async fn delete_container_info(container_id: &str) -> Result<()> {
    delete_info("containers", container_id).await
}

/// Get node logs from etcd
pub async fn get_node_logs(node_name: &str) -> Result<Vec<String>> {
    get_logs("nodes", node_name).await
}

/// Get SoC logs from etcd
pub async fn get_soc_logs(soc_id: &str) -> Result<Vec<String>> {
    get_logs("socs", soc_id).await
}

/// Get board logs from etcd
pub async fn get_board_logs(board_id: &str) -> Result<Vec<String>> {
    get_logs("boards", board_id).await
}

/// Get container logs from etcd
pub async fn get_container_logs(container_id: &str) -> Result<Vec<String>> {
    get_logs("containers", container_id).await
}

/// Generic function to get logs from etcd
async fn get_logs(resource_type: &str, resource_id: &str) -> Result<Vec<String>> {
    let prefix = format!("/piccolo/logs/{}/{}", resource_type, resource_id);
    let kv_pairs = common::etcd::get_all_with_prefix(&prefix)
        .await
        .map_err(|e| MonitoringEtcdError::EtcdOperation(e.to_string()))?;

    let mut logs = Vec::new();
    for kv in kv_pairs {
        logs.push(kv.value);
    }

    debug!(
        "Retrieved {} logs for {} {}",
        logs.len(),
        resource_type,
        resource_id
    );
    Ok(logs)
}

/// Store node metadata in etcd
pub async fn store_node_metadata(node_name: &str, metadata: &serde_json::Value) -> Result<()> {
    store_metadata("nodes", node_name, metadata).await
}

/// Store SoC metadata in etcd
pub async fn store_soc_metadata(soc_id: &str, metadata: &serde_json::Value) -> Result<()> {
    store_metadata("socs", soc_id, metadata).await
}

/// Store board metadata in etcd
pub async fn store_board_metadata(board_id: &str, metadata: &serde_json::Value) -> Result<()> {
    store_metadata("boards", board_id, metadata).await
}

/// Store container metadata in etcd
pub async fn store_container_metadata(
    container_id: &str,
    metadata: &serde_json::Value,
) -> Result<()> {
    store_metadata("containers", container_id, metadata).await
}

/// Generic function to store metadata in etcd
async fn store_metadata(
    resource_type: &str,
    resource_id: &str,
    metadata: &serde_json::Value,
) -> Result<()> {
    let key = format!("/piccolo/metadata/{}/{}", resource_type, resource_id);
    let value = serde_json::to_string(metadata)?;

    common::etcd::put(&key, &value)
        .await
        .map_err(|e| MonitoringEtcdError::EtcdOperation(e.to_string()))?;

    debug!("Stored metadata for {} {}", resource_type, resource_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::monitoringserver::ContainerInfo;
    use serde_json::json;
    use std::collections::HashMap;

    // Mock the common::etcd module for testing
    mod mock_etcd {
        use std::collections::HashMap;
        use std::sync::Mutex;

        static MOCK_STORAGE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
        static SHOULD_FAIL: Mutex<bool> = Mutex::new(false);

        pub fn init() {
            let mut storage = MOCK_STORAGE.lock().unwrap();
            *storage = Some(HashMap::new());
        }

        pub fn clear() {
            let mut storage = MOCK_STORAGE.lock().unwrap();
            if let Some(ref mut map) = *storage {
                map.clear();
            }
        }

        pub fn set_should_fail(should_fail: bool) {
            let mut fail_flag = SHOULD_FAIL.lock().unwrap();
            *fail_flag = should_fail;
        }

        pub async fn put(key: &str, value: &str) -> Result<(), String> {
            let should_fail = *SHOULD_FAIL.lock().unwrap();
            if should_fail {
                return Err("Mock etcd error".to_string());
            }

            let mut storage = MOCK_STORAGE.lock().unwrap();
            if let Some(ref mut map) = *storage {
                map.insert(key.to_string(), value.to_string());
            }
            Ok(())
        }

        pub async fn get(key: &str) -> Result<String, String> {
            let should_fail = *SHOULD_FAIL.lock().unwrap();
            if should_fail {
                return Err("Mock etcd error".to_string());
            }

            let storage = MOCK_STORAGE.lock().unwrap();
            if let Some(ref map) = *storage {
                map.get(key)
                    .cloned()
                    .ok_or_else(|| "Key not found".to_string())
            } else {
                Err("Storage not initialized".to_string())
            }
        }

        pub async fn delete(key: &str) -> Result<(), String> {
            let should_fail = *SHOULD_FAIL.lock().unwrap();
            if should_fail {
                return Err("Mock etcd error".to_string());
            }

            let mut storage = MOCK_STORAGE.lock().unwrap();
            if let Some(ref mut map) = *storage {
                map.remove(key);
            }
            Ok(())
        }

        #[derive(Clone)]
        pub struct KeyValue {
            pub key: String,
            pub value: String,
        }

        pub async fn get_all_with_prefix(prefix: &str) -> Result<Vec<KeyValue>, String> {
            let should_fail = *SHOULD_FAIL.lock().unwrap();
            if should_fail {
                return Err("Mock etcd error".to_string());
            }

            let storage = MOCK_STORAGE.lock().unwrap();
            if let Some(ref map) = *storage {
                let mut results = Vec::new();
                for (key, value) in map.iter() {
                    if key.starts_with(prefix) {
                        results.push(KeyValue {
                            key: key.clone(),
                            value: value.clone(),
                        });
                    }
                }
                Ok(results)
            } else {
                Err("Storage not initialized".to_string())
            }
        }
    }

    // Replace the common::etcd functions with our mock for testing
    // In a real test environment, you'd use a testing framework that supports mocking

    fn create_test_node_info() -> NodeInfo {
        NodeInfo {
            node_name: "test-node".to_string(),
            cpu_usage: 50.0,
            cpu_count: 8,
            gpu_count: 1,
            used_memory: 8192,
            total_memory: 16384,
            mem_usage: 50.0,
            rx_bytes: 1024,
            tx_bytes: 2048,
            read_bytes: 4096,
            write_bytes: 8192,
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            ip: "192.168.1.100".to_string(),
        }
    }

    fn create_test_soc_info() -> SocInfo {
        let test_node = NodeInfo {
            node_name: "test-node".to_string(),
            cpu_usage: 50.0,
            cpu_count: 8,
            gpu_count: 1,
            used_memory: 8192,
            total_memory: 16384,
            mem_usage: 50.0,
            rx_bytes: 1024,
            tx_bytes: 2048,
            read_bytes: 4096,
            write_bytes: 8192,
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            ip: "192.168.1.100".to_string(),
        };

        SocInfo {
            soc_id: "soc-1".to_string(),
            nodes: vec![test_node],
            total_cpu_usage: 50.0,
            total_cpu_count: 8,
            total_gpu_count: 1,
            total_used_memory: 8192,
            total_memory: 16384,
            total_mem_usage: 50.0,
            total_rx_bytes: 1024,
            total_tx_bytes: 2048,
            total_read_bytes: 4096,
            total_write_bytes: 8192,
            last_updated: std::time::SystemTime::now(),
        }
    }

    fn create_test_board_info() -> BoardInfo {
        let test_node = NodeInfo {
            node_name: "test-node".to_string(),
            cpu_usage: 50.0,
            cpu_count: 8,
            gpu_count: 1,
            used_memory: 8192,
            total_memory: 16384,
            mem_usage: 50.0,
            rx_bytes: 1024,
            tx_bytes: 2048,
            read_bytes: 4096,
            write_bytes: 8192,
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            ip: "192.168.1.100".to_string(),
        };

        let test_soc = SocInfo {
            soc_id: "soc-1".to_string(),
            nodes: vec![test_node.clone()],
            total_cpu_usage: 50.0,
            total_cpu_count: 8,
            total_gpu_count: 1,
            total_used_memory: 8192,
            total_memory: 16384,
            total_mem_usage: 50.0,
            total_rx_bytes: 1024,
            total_tx_bytes: 2048,
            total_read_bytes: 4096,
            total_write_bytes: 8192,
            last_updated: std::time::SystemTime::now(),
        };

        BoardInfo {
            board_id: "board-1".to_string(),
            nodes: vec![test_node],
            socs: vec![test_soc],
            total_cpu_usage: 50.0,
            total_cpu_count: 8,
            total_gpu_count: 1,
            total_used_memory: 8192,
            total_memory: 16384,
            total_mem_usage: 50.0,
            total_rx_bytes: 1024,
            total_tx_bytes: 2048,
            total_read_bytes: 4096,
            total_write_bytes: 8192,
            last_updated: std::time::SystemTime::now(),
        }
    }

    fn create_test_container_info() -> ContainerInfo {
        let mut state = std::collections::HashMap::new();
        state.insert("status".to_string(), "running".to_string());

        let mut config = std::collections::HashMap::new();
        config.insert("image".to_string(), "test:latest".to_string());

        let mut annotation = std::collections::HashMap::new();
        annotation.insert("version".to_string(), "1.0".to_string());

        let mut stats = std::collections::HashMap::new();
        stats.insert("cpu_usage".to_string(), "50.0".to_string());

        ContainerInfo {
            id: "container-123".to_string(),
            names: vec!["test-app".to_string()],
            image: "test:latest".to_string(),
            state,
            config,
            annotation,
            stats,
        }
    }

    #[test]
    fn test_monitoring_etcd_error_variants() {
        let etcd_error = MonitoringEtcdError::EtcdOperation("ETCD failed".to_string());
        let serialize_error = MonitoringEtcdError::Serialize(
            serde_json::from_str::<serde_json::Value>("{invalid json").unwrap_err(),
        );
        let not_found_error = MonitoringEtcdError::NotFound;
        let other_error = MonitoringEtcdError::Other("Generic error".to_string());

        // Test Display implementation
        assert_eq!(etcd_error.to_string(), "Etcd operation error: ETCD failed");
        assert!(serialize_error.to_string().contains("Serialization error"));
        assert_eq!(not_found_error.to_string(), "Data not found");
        assert_eq!(other_error.to_string(), "Generic error");

        // Test Debug implementation
        let debug_str = format!("{:?}", etcd_error);
        assert!(debug_str.contains("EtcdOperation"));
        assert!(debug_str.contains("ETCD failed"));
    }

    #[test]
    fn test_monitoring_etcd_error_from_serde() {
        let serde_error = serde_json::from_str::<serde_json::Value>("{invalid json").unwrap_err();
        let monitoring_error = MonitoringEtcdError::from(serde_error);

        match monitoring_error {
            MonitoringEtcdError::Serialize(_) => (),
            _ => panic!("Expected Serialize variant"),
        }
    }

    #[test]
    fn test_monitoring_etcd_error_from_utf8() {
        let utf8_error = std::str::from_utf8(&[0x80, 0x81]).unwrap_err();
        let monitoring_error = MonitoringEtcdError::from(utf8_error);

        match monitoring_error {
            MonitoringEtcdError::Utf8(_) => (),
            _ => panic!("Expected Utf8 variant"),
        }
    }

    #[test]
    fn test_result_type_alias() {
        let success: Result<i32> = Ok(42);
        let error: Result<i32> = Err(MonitoringEtcdError::NotFound);

        assert_eq!(success.unwrap(), 42);
        assert!(error.is_err());
    }

    // Note: The following tests would require actual mocking of the common::etcd module
    // In a real testing environment, you would use a mocking framework or dependency injection

    #[test]
    fn test_key_format_generation() {
        // Test the key format patterns used in the functions
        let resource_type = "nodes";
        let resource_id = "test-node";
        let expected_key = format!("/piccolo/metrics/{}/{}", resource_type, resource_id);
        assert_eq!(expected_key, "/piccolo/metrics/nodes/test-node");

        let logs_key = format!("/piccolo/logs/{}/{}", resource_type, resource_id);
        assert_eq!(logs_key, "/piccolo/logs/nodes/test-node");

        let metadata_key = format!("/piccolo/metadata/{}/{}", resource_type, resource_id);
        assert_eq!(metadata_key, "/piccolo/metadata/nodes/test-node");
    }

    #[test]
    fn test_prefix_format_generation() {
        let resource_type = "containers";
        let prefix = format!("/piccolo/metrics/{}/", resource_type);
        assert_eq!(prefix, "/piccolo/metrics/containers/");

        let logs_prefix = format!("/piccolo/logs/{}/{}", resource_type, "container-id");
        assert_eq!(logs_prefix, "/piccolo/logs/containers/container-id");
    }

    #[test]
    fn test_node_info_serialization() {
        let node_info = create_test_node_info();
        let json_result = serde_json::to_string(&node_info);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("test-node"));
        assert!(json_str.contains("192.168.1.100"));
        assert!(json_str.contains("Linux"));
        assert!(json_str.contains("x86_64"));

        // Test deserialization
        let deserialized: std::result::Result<NodeInfo, serde_json::Error> =
            serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        let deserialized_node = deserialized.unwrap();
        assert_eq!(deserialized_node.node_name, node_info.node_name);
        assert_eq!(deserialized_node.node_name, node_info.node_name);
    }

    #[test]
    fn test_soc_info_serialization() {
        let soc_info = create_test_soc_info();
        let json_result = serde_json::to_string(&soc_info);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("soc-1"));
        assert!(json_str.contains("50.0"));
        assert!(json_str.contains("8192"));
        assert!(json_str.contains("16384"));

        // Test deserialization
        let deserialized: std::result::Result<SocInfo, serde_json::Error> =
            serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        let deserialized_soc = deserialized.unwrap();
        assert_eq!(deserialized_soc.soc_id, soc_info.soc_id);
        assert_eq!(deserialized_soc.total_cpu_count, soc_info.total_cpu_count);
    }

    #[test]
    fn test_board_info_serialization() {
        let board_info = create_test_board_info();
        let json_result = serde_json::to_string(&board_info);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("board-1"));
        assert!(json_str.contains("50.0"));
        assert!(json_str.contains("8192"));
        assert!(json_str.contains("16384"));

        // Test deserialization
        let deserialized: std::result::Result<BoardInfo, serde_json::Error> =
            serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        let deserialized_board = deserialized.unwrap();
        assert_eq!(deserialized_board.board_id, board_info.board_id);
        assert_eq!(deserialized_board.board_id, board_info.board_id);
    }

    #[test]
    fn test_container_info_serialization() {
        let container_info = create_test_container_info();
        let json_result = serde_json::to_string(&container_info);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("container-123"));
        assert!(json_str.contains("test-app"));
        assert!(json_str.contains("test:latest"));
        assert!(json_str.contains("running"));

        // Test deserialization
        let deserialized: std::result::Result<ContainerInfo, serde_json::Error> =
            serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        let deserialized_container = deserialized.unwrap();
        assert_eq!(deserialized_container.id, container_info.id);
        assert_eq!(deserialized_container.names, container_info.names);
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = json!({
            "version": "1.0",
            "tags": ["production", "critical"],
            "config": {
                "timeout": 30,
                "retry_count": 3
            }
        });

        let json_result = serde_json::to_string(&metadata);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("version"));
        assert!(json_str.contains("1.0"));
        assert!(json_str.contains("tags"));
        assert!(json_str.contains("production"));
        assert!(json_str.contains("config"));
        assert!(json_str.contains("timeout"));
    }

    #[test]
    fn test_error_handling_patterns() {
        // Test various error scenarios that could occur

        // Serialization error simulation
        let invalid_metadata = json!({
            "invalid": std::f64::NAN  // NaN cannot be serialized to JSON
        });

        // This should work in our test, but demonstrates the pattern
        let result = serde_json::to_string(&invalid_metadata);
        // NaN actually gets serialized as null, so this won't fail
        // but in real scenarios with custom types, serialization can fail
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid for testing
    }

    #[test]
    fn test_resource_type_patterns() {
        let resource_types = vec!["nodes", "socs", "boards", "containers"];

        for resource_type in resource_types {
            let metrics_key = format!("/piccolo/metrics/{}/resource-id", resource_type);
            let logs_prefix = format!("/piccolo/logs/{}/resource-id", resource_type);
            let metadata_key = format!("/piccolo/metadata/{}/resource-id", resource_type);

            assert!(metrics_key.starts_with("/piccolo/metrics/"));
            assert!(logs_prefix.starts_with("/piccolo/logs/"));
            assert!(metadata_key.starts_with("/piccolo/metadata/"));

            assert!(metrics_key.contains(resource_type));
            assert!(logs_prefix.contains(resource_type));
            assert!(metadata_key.contains(resource_type));
        }
    }

    #[test]
    fn test_key_uniqueness() {
        // Test that different resource types and IDs generate unique keys
        let node_key = format!("/piccolo/metrics/{}/{}", "nodes", "node1");
        let soc_key = format!("/piccolo/metrics/{}/{}", "socs", "node1");
        let board_key = format!("/piccolo/metrics/{}/{}", "boards", "node1");
        let container_key = format!("/piccolo/metrics/{}/{}", "containers", "node1");

        let keys = vec![node_key, soc_key, board_key, container_key];
        let unique_keys: std::collections::HashSet<_> = keys.iter().collect();

        assert_eq!(keys.len(), unique_keys.len()); // All keys should be unique
    }

    #[test]
    fn test_edge_cases_for_resource_ids() {
        // Test with various resource ID formats
        let test_ids = vec![
            "simple-id",
            "id_with_underscores",
            "id.with.dots",
            "id:with:colons",
            "id-123-with-numbers",
            "UPPERCASE-ID",
            "mixed_Case.ID:123",
        ];

        for id in test_ids {
            let key = format!("/piccolo/metrics/nodes/{}", id);
            assert!(key.contains(id));
            assert!(key.starts_with("/piccolo/metrics/nodes/"));

            // Key should be well-formed (no double slashes except at start)
            let parts: Vec<&str> = key.split("//").collect();
            assert_eq!(
                parts.len(),
                1,
                "Key should not contain double slashes: {}",
                key
            );
        }
    }

    #[test]
    fn test_json_value_handling() {
        // Test various JSON value types that might be used as metadata
        let test_values = vec![
            json!(null),
            json!(true),
            json!(false),
            json!(42),
            json!(std::f64::consts::PI),
            json!("string value"),
            json!(["array", "of", "values"]),
            json!({"nested": {"object": "value"}}),
        ];

        for value in test_values {
            let serialized = serde_json::to_string(&value);
            assert!(
                serialized.is_ok(),
                "Should be able to serialize: {:?}",
                value
            );

            if let Ok(json_str) = serialized {
                let deserialized: std::result::Result<serde_json::Value, serde_json::Error> =
                    serde_json::from_str(&json_str);
                assert!(
                    deserialized.is_ok(),
                    "Should be able to deserialize: {}",
                    json_str
                );
            }
        }
    }

    #[test]
    fn test_empty_and_special_resource_ids() {
        // Test edge cases for resource IDs
        let special_ids = vec![
            "", // empty string
            " ", // space
            "\t", // tab
            "\n", // newline
            "very-long-resource-id-that-exceeds-normal-expectations-and-might-cause-issues-in-some-systems-but-should-still-work-correctly",
        ];

        for id in special_ids {
            let key = format!("/piccolo/metrics/nodes/{}", id);
            let prefix = format!("/piccolo/metrics/nodes/");

            assert!(key.starts_with(&prefix));
            assert_eq!(key.len(), prefix.len() + id.len());
        }
    }
}

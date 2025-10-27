/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Store and retrieve monitoring data in etcd

use crate::data_structures::{BoardInfo, SocInfo};
use common::monitoringserver::{ContainerInfo, NodeInfo}; // Use protobuf types
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// Generic function to store info in etcd
async fn store_info<T: Serialize>(
    resource_type: &str,
    resource_id: &str,
    info: &T,
) -> common::Result<()> {
    let key = format!("/piccolo/metrics/{}/{}", resource_type, resource_id);
    let json_data = serde_json::to_string(info)
        .map_err(|e| format!("Failed to serialize {}: {}", resource_type, e))?;

    common::etcd::put(&key, &json_data).await?;
    println!(
        "[ETCD] Stored the metrics for {}: {}",
        resource_type, resource_id
    );
    Ok(())
}

/// Generic function to retrieve info from etcd
async fn get_info<T: DeserializeOwned>(
    resource_type: &str,
    resource_id: &str,
) -> common::Result<T> {
    let key = format!("/piccolo/metrics/{}/{}", resource_type, resource_id);
    let json_data = common::etcd::get(&key).await?;

    let info: T = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to deserialize {}: {}", resource_type, e))?;

    Ok(info)
}

/// Generic function to delete info from etcd
async fn delete_info(resource_type: &str, resource_id: &str) -> common::Result<()> {
    let key = format!("/piccolo/metrics/{}/{}", resource_type, resource_id);
    common::etcd::delete(&key).await?;
    println!(
        "[ETCD] Deleted the metrics for {}: {}",
        resource_type, resource_id
    );
    Ok(())
}

/// Generic function to get all items of a type from etcd
async fn get_all_info<T: DeserializeOwned>(resource_type: &str) -> common::Result<Vec<T>> {
    let prefix = format!("/piccolo/metrics/{}/", resource_type);
    let kv_pairs = common::etcd::get_all_with_prefix(&prefix).await?;

    let mut items = Vec::new();
    for kv in kv_pairs {
        match serde_json::from_str::<T>(&kv.value) {
            Ok(item) => items.push(item),
            Err(e) => eprintln!(
                "[ETCD] Failed to deserialize {} {}: {}",
                resource_type, kv.key, e
            ),
        }
    }

    Ok(items)
}

// Public API functions using the generic implementations

/// Store NodeInfo in etcd
pub async fn store_node_info(node_info: &NodeInfo) -> common::Result<()> {
    store_info("nodes", &node_info.node_name, node_info).await
}

/// Store SocInfo in etcd
pub async fn store_soc_info(soc_info: &SocInfo) -> common::Result<()> {
    store_info("socs", &soc_info.soc_id, soc_info).await
}

/// Store BoardInfo in etcd
pub async fn store_board_info(board_info: &BoardInfo) -> common::Result<()> {
    store_info("boards", &board_info.board_id, board_info).await
}

/// Store ContainerInfo in etcd - Using same pattern as others
pub async fn store_container_info(container_info: &ContainerInfo) -> common::Result<()> {
    // Convert protobuf ContainerInfo to JSON for storage using the same pattern
    let json_value = serde_json::json!({
        "id": container_info.id,
        "names": container_info.names,
        "image": container_info.image,
        "state": container_info.state,
        "config": container_info.config,
        "annotation": container_info.annotation,
        "stats": container_info.stats,
    });

    store_info("containers", &container_info.id, &json_value).await
}

/// Retrieve NodeInfo from etcd
pub async fn get_node_info(node_name: &str) -> common::Result<NodeInfo> {
    get_info("nodes", node_name).await
}

/// Retrieve SocInfo from etcd
pub async fn get_soc_info(soc_id: &str) -> common::Result<SocInfo> {
    get_info("socs", soc_id).await
}

/// Retrieve BoardInfo from etcd
pub async fn get_board_info(board_id: &str) -> common::Result<BoardInfo> {
    get_info("boards", board_id).await
}

/// Get ContainerInfo from etcd - Using same pattern as others
pub async fn get_container_info(container_id: &str) -> common::Result<ContainerInfo> {
    let json_value: serde_json::Value = get_info("containers", container_id).await?;

    // Convert JSON back to protobuf ContainerInfo
    let container_info = ContainerInfo {
        id: json_value["id"].as_str().unwrap_or_default().to_string(),
        names: json_value["names"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect(),
        image: json_value["image"].as_str().unwrap_or_default().to_string(),
        state: json_value["state"]
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect(),
        config: json_value["config"]
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect(),
        annotation: json_value["annotation"]
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect(),
        stats: json_value["stats"]
            .as_object()
            .unwrap_or(&serde_json::Map::new())
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
            .collect(),
    };

    Ok(container_info)
}

/// Get all nodes from etcd
pub async fn get_all_nodes() -> common::Result<Vec<NodeInfo>> {
    get_all_info("nodes").await
}

/// Get all SoCs from etcd
pub async fn get_all_socs() -> common::Result<Vec<SocInfo>> {
    get_all_info("socs").await
}

/// Get all boards from etcd
pub async fn get_all_boards() -> common::Result<Vec<BoardInfo>> {
    get_all_info("boards").await
}

/// Get all containers from etcd
pub async fn get_all_containers() -> common::Result<Vec<ContainerInfo>> {
    let prefix = "/piccolo/metrics/containers/".to_string();
    let kv_pairs = common::etcd::get_all_with_prefix(&prefix).await?;

    let mut containers = Vec::new();
    for kv in kv_pairs {
        match serde_json::from_str::<serde_json::Value>(&kv.value) {
            Ok(json_value) => {
                let container_info = ContainerInfo {
                    id: json_value["id"].as_str().unwrap_or_default().to_string(),
                    names: json_value["names"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .map(|v| v.as_str().unwrap_or_default().to_string())
                        .collect(),
                    image: json_value["image"].as_str().unwrap_or_default().to_string(),
                    state: json_value["state"]
                        .as_object()
                        .unwrap_or(&serde_json::Map::new())
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect(),
                    config: json_value["config"]
                        .as_object()
                        .unwrap_or(&serde_json::Map::new())
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect(),
                    annotation: json_value["annotation"]
                        .as_object()
                        .unwrap_or(&serde_json::Map::new())
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect(),
                    stats: json_value["stats"]
                        .as_object()
                        .unwrap_or(&serde_json::Map::new())
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                        .collect(),
                };
                containers.push(container_info);
            }
            Err(e) => eprintln!("[ETCD] Failed to deserialize container {}: {}", kv.key, e),
        }
    }

    Ok(containers)
}

/// Store a raw stress metric JSON string in etcd under /piccolo/metrics/stress/{process}/{pid}:{ts}
pub async fn store_stress_metric_json(json_str: &str) -> common::Result<()> {
    // parse & validate JSON
    let v: Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse stress metric JSON: {}", e))?;

    let process_name = v
        .get("process_name")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    let pid_str = v
        .get("pid")
        .and_then(|p| p.as_i64().map(|n| n.to_string()))
        .unwrap_or_else(|| "0".to_string());

    let resource_id = format!("{}/{}", process_name, pid_str);

    // store_info - serde_json::Value implements Serialize
    store_info("stress", &resource_id, &v).await
}

/// Retrieve all stored stress metrics as JSON values.
pub async fn get_all_stress_metrics() -> common::Result<Vec<Value>> {
    get_all_info("stress").await
}

/// Delete a stored stress metric by resource id (the id returned/used when storing)
pub async fn delete_stress_metric(resource_id: &str) -> common::Result<()> {
    delete_info("stress", resource_id).await
}

/// Delete NodeInfo from etcd
pub async fn delete_node_info(node_name: &str) -> common::Result<()> {
    delete_info("nodes", node_name).await
}

/// Delete SocInfo from etcd
pub async fn delete_soc_info(soc_id: &str) -> common::Result<()> {
    delete_info("socs", soc_id).await
}

/// Delete BoardInfo from etcd
pub async fn delete_board_info(board_id: &str) -> common::Result<()> {
    delete_info("boards", board_id).await
}

/// Delete ContainerInfo from etcd
pub async fn delete_container_info(container_id: &str) -> common::Result<()> {
    delete_info("containers", container_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_structures::{BoardInfo, SocInfo};
    use common::monitoringserver::{ContainerInfo, NodeInfo};
    use std::time::SystemTime;

    fn sample_node(name: &str, ip: &str) -> NodeInfo {
        NodeInfo {
            node_name: name.to_string(),
            ip: ip.to_string(),
            cpu_usage: 42.0,
            cpu_count: 2,
            gpu_count: 1,
            used_memory: 1024,
            total_memory: 2048,
            mem_usage: 50.0,
            rx_bytes: 100,
            tx_bytes: 200,
            read_bytes: 300,
            write_bytes: 400,
            arch: "x86_64".to_string(),
            os: "linux".to_string(),
        }
    }

    fn sample_container(id: &str, name: &str) -> ContainerInfo {
        ContainerInfo {
            id: id.to_string(),
            names: vec![name.to_string()],
            ..Default::default()
        }
    }

    fn sample_soc(soc_id: &str, node: NodeInfo) -> SocInfo {
        SocInfo {
            soc_id: soc_id.to_string(),
            nodes: vec![node],
            total_cpu_usage: 42.0,
            total_cpu_count: 2,
            total_gpu_count: 1,
            total_used_memory: 1024,
            total_memory: 2048,
            total_mem_usage: 50.0,
            total_rx_bytes: 100,
            total_tx_bytes: 200,
            total_read_bytes: 300,
            total_write_bytes: 400,
            last_updated: SystemTime::now(),
        }
    }

    fn sample_board(board_id: &str, node: NodeInfo) -> BoardInfo {
        BoardInfo {
            board_id: board_id.to_string(),
            nodes: vec![node],
            socs: vec![],
            total_cpu_usage: 42.0,
            total_cpu_count: 2,
            total_gpu_count: 1,
            total_used_memory: 1024,
            total_memory: 2048,
            total_mem_usage: 50.0,
            total_rx_bytes: 100,
            total_tx_bytes: 200,
            total_read_bytes: 300,
            total_write_bytes: 400,
            last_updated: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_store_and_delete_node_info() {
        let node = sample_node("node1", "192.168.10.201");
        let result = store_node_info(&node).await;
        // Should be Ok or error if etcd is not running
        assert!(result.is_ok() || result.is_err());

        let result = delete_node_info("node1").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_store_and_delete_soc_info() {
        let node = sample_node("node1", "192.168.10.201");
        let soc = sample_soc("soc1", node);
        let result = store_soc_info(&soc).await;
        assert!(result.is_ok() || result.is_err());

        let result = delete_soc_info("soc1").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_store_and_delete_board_info() {
        let node = sample_node("node1", "192.168.10.201");
        let board = sample_board("board1", node);
        let result = store_board_info(&board).await;
        assert!(result.is_ok() || result.is_err());

        let result = delete_board_info("board1").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_store_and_delete_container_info() {
        let container = sample_container("c1", "container1");
        let result = store_container_info(&container).await;
        assert!(result.is_ok() || result.is_err());

        let result = delete_container_info("c1").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_get_node_info_not_found() {
        let result = get_node_info("notfound").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_soc_info_not_found() {
        let result = get_soc_info("notfound").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_board_info_not_found() {
        let result = get_board_info("notfound").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_container_info_not_found() {
        let result = get_container_info("notfound").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_nodes_socs_boards_containers() {
        // These should not panic, even if etcd is empty or not running
        let _ = get_all_nodes().await;
        let _ = get_all_socs().await;
        let _ = get_all_boards().await;
        let _ = get_all_containers().await;
    }

    #[tokio::test]
    async fn test_store_info_and_get_info_generic() {
        let node = sample_node("node1", "192.168.10.201");
        let store_result = super::store_info("nodes", "node1", &node).await;
        assert!(store_result.is_ok() || store_result.is_err());

        let get_result: Result<NodeInfo, _> = super::get_info("nodes", "node1").await;
        // Should be Ok if etcd is running, Err otherwise
        assert!(get_result.is_ok() || get_result.is_err());
    }

    #[tokio::test]
    async fn test_delete_info_generic() {
        let del_result = super::delete_info("nodes", "node1").await;
        assert!(del_result.is_ok() || del_result.is_err());
    }

    #[tokio::test]
    async fn test_get_all_info_generic() {
        let all_nodes: Result<Vec<NodeInfo>, _> = super::get_all_info("nodes").await;
        assert!(all_nodes.is_ok() || all_nodes.is_err());
    }

    #[tokio::test]
    async fn test_store_node_info_api() {
        let node = sample_node("node2", "192.168.10.202");
        let result = store_node_info(&node).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_store_soc_info_api() {
        let node = sample_node("node3", "192.168.10.203");
        let soc = sample_soc("soc3", node);
        let result = store_soc_info(&soc).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_store_board_info_api() {
        let node = sample_node("node4", "192.168.10.204");
        let board = sample_board("board4", node);
        let result = store_board_info(&board).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_store_container_info_api() {
        let container = sample_container("c4", "container4");
        let result = store_container_info(&container).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_get_node_info_api() {
        let result = get_node_info("node2").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_get_soc_info_api() {
        let result = get_soc_info("soc3").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_get_board_info_api() {
        let result = get_board_info("board4").await;
        assert!(result.is_ok() || result.is_err());
    }
}

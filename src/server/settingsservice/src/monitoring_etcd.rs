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

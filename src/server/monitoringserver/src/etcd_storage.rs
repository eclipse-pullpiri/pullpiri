/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Store and retrieve monitoring data in etcd

use crate::data_structures::{BoardInfo, SocInfo};
use common::monitoringserver::NodeInfo;
use serde::{de::DeserializeOwned, Serialize};

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

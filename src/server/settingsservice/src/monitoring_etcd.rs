// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration with monitoring server's etcd storage

use crate::monitoring_types::{BoardInfo, NodeInfo, SocInfo};
use etcd_client::Client;

type Result<T> = std::result::Result<T, String>;

/// Create storage client
async fn get_etcd_client() -> Result<Client> {
    Client::connect(["http://localhost:2379"], None)
        .await
        .map_err(|e| format!("Failed to connect to etcd: {}", e))
}

/// Store NodeInfo in etcd
pub async fn store_node_info(node_info: &NodeInfo) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/nodes/{}", node_info.node_name);
    let value =
        serde_json::to_string(node_info).map_err(|e| format!("Serialization error: {}", e))?;

    client
        .put(key, value, None)
        .await
        .map_err(|e| format!("ETCD put error: {}", e))?;

    Ok(())
}

/// Get NodeInfo from etcd
pub async fn get_node_info(node_name: &str) -> Result<NodeInfo> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/nodes/{}", node_name);
    let resp = client
        .get(key, None)
        .await
        .map_err(|e| format!("ETCD get error: {}", e))?;

    if let Some(kv) = resp.kvs().first() {
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error: {}", e))?;
        serde_json::from_str(value).map_err(|e| format!("Deserialization error: {}", e))
    } else {
        Err("NodeInfo not found".to_string())
    }
}

/// Get all nodes from etcd
pub async fn get_all_nodes() -> Result<Vec<NodeInfo>> {
    let mut client = get_etcd_client().await?;

    let resp = client
        .get(
            "/piccolo/metrics/nodes/",
            Some(etcd_client::GetOptions::new().with_prefix()),
        )
        .await
        .map_err(|e| format!("ETCD get error: {}", e))?;

    let mut nodes = Vec::new();
    for kv in resp.kvs() {
        let key = std::str::from_utf8(kv.key())
            .map_err(|e| format!("UTF-8 conversion error for key: {}", e))?;
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error for value: {}", e))?;

        match serde_json::from_str::<NodeInfo>(value) {
            Ok(node) => nodes.push(node),
            Err(e) => eprintln!("Failed to deserialize NodeInfo from {}: {}", key, e),
        }
    }
    Ok(nodes)
}

/// Delete NodeInfo from etcd
pub async fn delete_node_info(node_name: &str) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/nodes/{}", node_name);
    client
        .delete(key, None)
        .await
        .map_err(|e| format!("ETCD delete error: {}", e))?;

    Ok(())
}

/// Store SocInfo in etcd
pub async fn store_soc_info(soc_info: &SocInfo) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/socs/{}", soc_info.soc_id);
    let value =
        serde_json::to_string(soc_info).map_err(|e| format!("Serialization error: {}", e))?;

    client
        .put(key, value, None)
        .await
        .map_err(|e| format!("ETCD put error: {}", e))?;

    Ok(())
}

/// Get SocInfo from etcd
pub async fn get_soc_info(soc_id: &str) -> Result<SocInfo> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/socs/{}", soc_id);
    let resp = client
        .get(key, None)
        .await
        .map_err(|e| format!("ETCD get error: {}", e))?;

    if let Some(kv) = resp.kvs().first() {
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error: {}", e))?;
        serde_json::from_str(value).map_err(|e| format!("Deserialization error: {}", e))
    } else {
        Err("SocInfo not found".to_string())
    }
}

/// Get all SoCs from etcd
pub async fn get_all_socs() -> Result<Vec<SocInfo>> {
    let mut client = get_etcd_client().await?;

    let resp = client
        .get(
            "/piccolo/metrics/socs/",
            Some(etcd_client::GetOptions::new().with_prefix()),
        )
        .await
        .map_err(|e| format!("ETCD get error: {}", e))?;

    let mut socs = Vec::new();
    for kv in resp.kvs() {
        let key = std::str::from_utf8(kv.key())
            .map_err(|e| format!("UTF-8 conversion error for key: {}", e))?;
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error for value: {}", e))?;

        match serde_json::from_str::<SocInfo>(value) {
            Ok(soc) => socs.push(soc),
            Err(e) => eprintln!("Failed to deserialize SocInfo from {}: {}", key, e),
        }
    }
    Ok(socs)
}

/// Delete SocInfo from etcd
pub async fn delete_soc_info(soc_id: &str) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/socs/{}", soc_id);
    client
        .delete(key, None)
        .await
        .map_err(|e| format!("ETCD delete error: {}", e))?;

    Ok(())
}

/// Store BoardInfo in etcd
pub async fn store_board_info(board_info: &BoardInfo) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/boards/{}", board_info.board_id);
    let value =
        serde_json::to_string(board_info).map_err(|e| format!("Serialization error: {}", e))?;

    client
        .put(key, value, None)
        .await
        .map_err(|e| format!("ETCD put error: {}", e))?;

    Ok(())
}

/// Get BoardInfo from etcd
pub async fn get_board_info(board_id: &str) -> Result<BoardInfo> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/boards/{}", board_id);
    let resp = client
        .get(key, None)
        .await
        .map_err(|e| format!("ETCD get error: {}", e))?;

    if let Some(kv) = resp.kvs().first() {
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error: {}", e))?;
        serde_json::from_str(value).map_err(|e| format!("Deserialization error: {}", e))
    } else {
        Err("BoardInfo not found".to_string())
    }
}

/// Get all boards from etcd
pub async fn get_all_boards() -> Result<Vec<BoardInfo>> {
    let mut client = get_etcd_client().await?;

    let resp = client
        .get(
            "/piccolo/metrics/boards/",
            Some(etcd_client::GetOptions::new().with_prefix()),
        )
        .await
        .map_err(|e| format!("ETCD get error: {}", e))?;

    let mut boards = Vec::new();
    for kv in resp.kvs() {
        let key = std::str::from_utf8(kv.key())
            .map_err(|e| format!("UTF-8 conversion error for key: {}", e))?;
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error for value: {}", e))?;

        match serde_json::from_str::<BoardInfo>(value) {
            Ok(board) => boards.push(board),
            Err(e) => eprintln!("Failed to deserialize BoardInfo from {}: {}", key, e),
        }
    }
    Ok(boards)
}

/// Delete BoardInfo from etcd
pub async fn delete_board_info(board_id: &str) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metrics/boards/{}", board_id);
    client
        .delete(key, None)
        .await
        .map_err(|e| format!("ETCD delete error: {}", e))?;

    Ok(())
}

/// Get node logs from etcd
pub async fn get_node_logs(node_name: &str) -> Result<Vec<String>> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/logs/nodes/{}", node_name);
    let resp = client
        .get(key, Some(etcd_client::GetOptions::new().with_prefix()))
        .await
        .map_err(|e| format!("ETCD get logs error: {}", e))?;

    let mut logs = Vec::new();
    for kv in resp.kvs() {
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error: {}", e))?;
        logs.push(value.to_string());
    }

    // If no logs found in dedicated logs path, return empty Vec (not an error)
    Ok(logs)
}

/// Get SoC logs from etcd
pub async fn get_soc_logs(soc_id: &str) -> Result<Vec<String>> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/logs/socs/{}", soc_id);
    let resp = client
        .get(key, Some(etcd_client::GetOptions::new().with_prefix()))
        .await
        .map_err(|e| format!("ETCD get logs error: {}", e))?;

    let mut logs = Vec::new();
    for kv in resp.kvs() {
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error: {}", e))?;
        logs.push(value.to_string());
    }

    Ok(logs)
}

/// Get board logs from etcd
pub async fn get_board_logs(board_id: &str) -> Result<Vec<String>> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/logs/boards/{}", board_id);
    let resp = client
        .get(key, Some(etcd_client::GetOptions::new().with_prefix()))
        .await
        .map_err(|e| format!("ETCD get logs error: {}", e))?;

    let mut logs = Vec::new();
    for kv in resp.kvs() {
        let value = std::str::from_utf8(kv.value())
            .map_err(|e| format!("UTF-8 conversion error: {}", e))?;
        logs.push(value.to_string());
    }

    Ok(logs)
}

/// Store node metadata in etcd
pub async fn store_node_metadata(node_name: &str, metadata: &serde_json::Value) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metadata/nodes/{}", node_name);
    let value =
        serde_json::to_string(metadata).map_err(|e| format!("Serialization error: {}", e))?;

    client
        .put(key, value, None)
        .await
        .map_err(|e| format!("ETCD put metadata error: {}", e))?;

    Ok(())
}

/// Store SoC metadata in etcd
pub async fn store_soc_metadata(soc_id: &str, metadata: &serde_json::Value) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metadata/socs/{}", soc_id);
    let value =
        serde_json::to_string(metadata).map_err(|e| format!("Serialization error: {}", e))?;

    client
        .put(key, value, None)
        .await
        .map_err(|e| format!("ETCD put metadata error: {}", e))?;

    Ok(())
}

/// Store board metadata in etcd
pub async fn store_board_metadata(board_id: &str, metadata: &serde_json::Value) -> Result<()> {
    let mut client = get_etcd_client().await?;

    let key = format!("/piccolo/metadata/boards/{}", board_id);
    let value =
        serde_json::to_string(metadata).map_err(|e| format!("Serialization error: {}", e))?;

    client
        .put(key, value, None)
        .await
        .map_err(|e| format!("ETCD put metadata error: {}", e))?;

    Ok(())
}

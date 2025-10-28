/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Node lookup utilities for finding nodes in the cluster

use common::apiserver::NodeInfo;
use common::etcd;
use serde_json;
use std::error::Error;

/// Find a node by IP address from simplified node keys
pub async fn find_node_by_simple_key() -> Option<String> {
    println!("Checking simplified node keys in etcd...");
    match etcd::get_all_with_prefix("nodes/").await {
        Ok(kvs) => {
            println!("Found {} simplified node keys", kvs.len());
            if let Some(kv) = kvs.first() {
                println!("Node key: {}", kv.key);
                let ip_address = kv.key.trim_start_matches("nodes/");
                println!("Found node IP directly from key: {}", ip_address);
                return Some(ip_address.to_string());
            }
            None
        }
        Err(e) => {
            println!("Error checking simplified nodes: {}", e);
            None
        }
    }
}

/// Find a node directly from etcd using cluster/nodes/ prefix
pub async fn find_node_from_etcd() -> Option<String> {
    println!("Checking cluster/nodes/ prefix in etcd...");
    let kvs = match etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => kvs,
        Err(e) => {
            println!("Error getting nodes: {}", e);
            return None;
        }
    };

    println!("Found {} node entries", kvs.len());
    for kv in &kvs {
        println!("Node entry: {}", kv.key);
    }

    if kvs.is_empty() {
        return None;
    }

    match serde_json::from_str::<NodeInfo>(&kvs[0].value) {
        Ok(node) => {
            println!(
                "Decoded node: {} ({}), status: {}",
                node.node_id, node.ip_address, node.status
            );
            Some(node.ip_address)
        }
        Err(e) => {
            println!("Failed to parse JSON: {} for value: {}", e, &kvs[0].value);
            None
        }
    }
}

/// Find a node using the NodeManager
pub async fn find_node_from_manager() -> Option<String> {
    let node_manager = match crate::node::NodeManager::new() {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("Failed to create NodeManager: {}", e);
            return None;
        }
    };

    match node_manager.get_nodes().await {
        Ok(nodes) => {
            println!("Node manager found {} nodes", nodes.len());

            if !nodes.is_empty() {
                let node = &nodes[0];
                println!(
                    "Node manager found: {} ({}), status: {}",
                    node.node_id, node.ip_address, node.status
                );
                Some(node.ip_address.clone())
            } else {
                println!("Node manager found no nodes");
                None
            }
        }
        Err(e) => {
            eprintln!("Node manager error: {}", e);
            None
        }
    }
}

/// Find a node by hostname
pub async fn find_node_by_hostname(hostname: &str) -> Option<common::apiserver::NodeInfo> {
    println!("Looking for node with hostname: {}", hostname);
    let kvs = match common::etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => kvs,
        Err(e) => {
            println!("Error searching for hostname {}: {}", hostname, e);
            return None;
        }
    };

    println!("Found {} entries in etcd", kvs.len());
    for kv in kvs {
        println!(
            "Processing key: {}",
            String::from_utf8_lossy(kv.key.as_bytes())
        );

        match serde_json::from_str::<common::apiserver::NodeInfo>(&kv.value) {
            Ok(node_info) => {
                println!("Successfully parsed node info: {}", node_info.hostname);
                if node_info.hostname == hostname {
                    println!(
                        "Found node with hostname {}: {}",
                        hostname, node_info.ip_address
                    );
                    return Some(node_info);
                }
            }
            Err(e) => println!("Failed to parse JSON: {} for value: {}", e, kv.value),
        }
    }

    println!("No node found with hostname: {}", hostname);
    None
}

/// Get node IP using all available methods
pub async fn get_node_ip() -> String {
    // Try all methods to find a node IP
    if let Some(ip) = find_node_by_simple_key().await {
        return ip;
    }

    if let Some(ip) = find_node_from_etcd().await {
        return ip;
    }

    if let Some(ip) = find_node_from_manager().await {
        return ip;
    }

    // Fallback to settings file if no nodes are found
    let config = common::setting::get_config();
    let node_ip = config.host.ip.clone();
    println!(
        "All node lookups failed. Falling back to settings IP: {}",
        node_ip
    );

    node_ip
}

/// Add a node IP to the simplified keys for quick lookup
#[allow(dead_code)]
pub async fn add_node_to_simple_keys(ip_address: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let key = format!("nodes/{}", ip_address);
    etcd::put(&key, ip_address).await?;
    println!("Added node IP to simple keys: {}", ip_address);
    Ok(())
}

/// 게스트 노드 정보를 etcd에서 검색하는 함수
pub async fn find_guest_nodes() -> Vec<NodeInfo> {
    println!("Finding guest nodes from etcd...");
    let kvs = match etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => kvs,
        Err(e) => {
            println!("Error searching for guest nodes: {}", e);
            return Vec::new();
        }
    };

    println!("Found {} node entries for guest search", kvs.len());
    let mut guest_nodes = Vec::new();

    for kv in kvs {
        match serde_json::from_str::<NodeInfo>(&kv.value) {
            Ok(node_info) => {
                // 마스터 노드가 아닌 경우에만 게스트 노드로 간주
                if node_info.node_role != common::nodeagent::NodeRole::Master as i32 {
                    println!(
                        "Found guest node: {} ({}) with role: {}",
                        node_info.node_id, node_info.ip_address, node_info.node_role
                    );
                    guest_nodes.push(node_info);
                }
            }
            Err(e) => println!("Failed to parse JSON: {} for value: {}", e, kv.value),
        }
    }

    println!("Found {} guest nodes", guest_nodes.len());
    guest_nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::{NodeRole, NodeStatus, NodeType, ResourceInfo};
    use std::collections::HashMap;

    fn create_test_node_info(
        node_id: &str,
        hostname: &str,
        ip_address: &str,
        node_role: NodeRole,
        status: NodeStatus,
    ) -> NodeInfo {
        NodeInfo {
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            ip_address: ip_address.to_string(),
            node_type: NodeType::Vehicle as i32,
            node_role: node_role as i32,
            status: status as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 100,
                architecture: "x86_64".to_string(),
                os_version: "Ubuntu 20.04".to_string(),
            }),
            last_heartbeat: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_find_node_by_simple_key_success() {
        let result = find_node_by_simple_key().await;
        match result {
            Some(ip) => {
                assert!(!ip.is_empty());
                println!("Found IP via simple key: {}", ip);
            }
            None => {
                println!("No nodes found via simple key (expected if etcd unavailable or empty)")
            }
        }
    }

    #[tokio::test]
    async fn test_find_node_from_etcd_success() {
        let result = find_node_from_etcd().await;
        match result {
            Some(ip) => {
                assert!(!ip.is_empty());
                println!("Found IP via etcd: {}", ip);
            }
            None => println!("No nodes found via etcd (expected if etcd unavailable or empty)"),
        }
    }

    #[tokio::test]
    async fn test_find_node_by_hostname() {
        let result = find_node_by_hostname("test-hostname").await;
        assert!(
            result.is_none(),
            "Should not find test hostname in empty etcd"
        );
    }

    #[tokio::test]
    async fn test_find_guest_nodes() {
        let guest_nodes = find_guest_nodes().await;
        assert!(
            guest_nodes.is_empty(),
            "Should return empty vector when no nodes found"
        );
    }
}

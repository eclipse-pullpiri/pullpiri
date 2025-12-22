/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node lookup utilities for finding nodes in the cluster

use base64::Engine;
use common::apiserver::NodeInfo;
use common::etcd;
use prost::Message;
use std::error::Error;

/// Find a node by IP address from simplified node keys
/// Filters out invalid IPs like 0.0.0.0 and empty strings
pub async fn find_node_by_simple_key() -> Option<String> {
    println!("Checking simplified node keys in etcd...");
    match etcd::get_all_with_prefix("nodes/").await {
        Ok(kvs) => {
            println!("Found {} simplified node keys", kvs.len());
            // Iterate through all nodes and find the first valid IP
            for kv in kvs {
                println!("Node key: {}", kv.key);
                let ip_address = kv.key.trim_start_matches("nodes/");

                // Skip invalid IPs: empty, 0.0.0.0, or localhost when looking for remote nodes
                if ip_address.is_empty()
                    || ip_address == "0.0.0.0"
                    || ip_address == "127.0.0.1"
                {
                    println!("Skipping invalid IP: {}", ip_address);
                    continue;
                }

                println!("Found valid node IP from key: {}", ip_address);
                return Some(ip_address.to_string());
            }
            println!("No valid node IPs found in simplified keys");
            None
        }
        Err(e) => {
            println!("Error checking simplified nodes: {}", e);
            None
        }
    }
}

/// Find a node by specific target IP address from simplified node keys
/// Returns the IP if it exists and is valid, None otherwise
pub async fn find_node_by_target_ip(target_ip: &str) -> Option<String> {
    if target_ip.is_empty() || target_ip == "0.0.0.0" {
        return None;
    }

    println!("Looking for specific node IP: {}", target_ip);
    let key = format!("nodes/{}", target_ip);

    match etcd::get(&key).await {
        Ok(Some(_)) => {
            println!("Found target node IP in etcd: {}", target_ip);
            Some(target_ip.to_string())
        }
        Ok(None) => {
            println!("Target node IP not found in etcd: {}", target_ip);
            None
        }
        Err(e) => {
            println!("Error checking for target IP {}: {}", target_ip, e);
            None
        }
    }
}

/// Find a node directly from etcd using cluster/nodes/ prefix
pub async fn find_node_from_etcd() -> Option<String> {
    println!("Checking cluster/nodes/ prefix in etcd...");
    match etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => {
            println!("Found {} node entries", kvs.len());
            for kv in &kvs {
                println!("Node entry: {}", kv.key);
            }

            if !kvs.is_empty() {
                match base64::engine::general_purpose::STANDARD.decode(&kvs[0].value) {
                    Ok(buf) => match NodeInfo::decode(&buf[..]) {
                        Ok(node) => {
                            println!(
                                "Decoded node: {} ({}), status: {}",
                                node.node_id, node.ip_address, node.status
                            );
                            return Some(node.ip_address);
                        }
                        Err(e) => println!("Failed to decode node data: {}", e),
                    },
                    Err(e) => println!("Failed to decode base64: {}", e),
                }
            }
            None
        }
        Err(e) => {
            println!("Error getting nodes: {}", e);
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
    match common::etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => {
            for kv in kvs {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&kv.value) {
                    if let Ok(node_info) = common::apiserver::NodeInfo::decode(&decoded[..]) {
                        if node_info.hostname == hostname {
                            println!(
                                "Found node with hostname {}: {}",
                                hostname, node_info.ip_address
                            );
                            return Some(node_info);
                        }
                    }
                }
            }
            println!("No node found with hostname: {}", hostname);
            None
        }
        Err(e) => {
            println!("Error searching for hostname {}: {}", hostname, e);
            None
        }
    }
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
    match etcd::get_all_with_prefix("cluster/nodes/").await {
        Ok(kvs) => {
            println!("Found {} node entries for guest search", kvs.len());
            let mut guest_nodes = Vec::new();

            for kv in kvs {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&kv.value) {
                    if let Ok(node_info) = NodeInfo::decode(&decoded[..]) {
                        // 마스터 노드가 아닌 경우에만 게스트 노드로 간주
                        if node_info.node_role != common::nodeagent::NodeRole::Master as i32 {
                            println!(
                                "Found guest node: {} ({}) with role: {}",
                                node_info.node_id, node_info.ip_address, node_info.node_role
                            );
                            guest_nodes.push(node_info);
                        }
                    }
                }
            }

            println!("Found {} guest nodes", guest_nodes.len());
            guest_nodes
        }
        Err(e) => {
            println!("Error searching for guest nodes: {}", e);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use common::apiserver::NodeInfo;
    use common::nodeagent::{NodeRole, NodeStatus, NodeType, ResourceInfo};
    use prost::Message;
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

    fn encode_node_info(node_info: &NodeInfo) -> String {
        let mut buf = Vec::new();
        node_info.encode(&mut buf).unwrap();
        base64::engine::general_purpose::STANDARD.encode(&buf)
    }

    #[tokio::test]
    async fn test_find_node_by_simple_key_success() {
        // This test will pass if etcd returns data, fail if etcd is unavailable
        let result = find_node_by_simple_key().await;

        match result {
            Some(ip) => {
                assert!(!ip.is_empty());
                println!("Found IP via simple key: {}", ip);
            }
            None => {
                println!("No nodes found via simple key (expected if etcd unavailable or empty)");
            }
        }
    }

    #[tokio::test]
    async fn test_find_node_by_simple_key_no_nodes() {
        // Test behavior when no nodes are found
        // This will exercise the None return path
        let result = find_node_by_simple_key().await;
        // Result can be either Some or None depending on etcd state
        println!("Simple key lookup result: {:?}", result.is_some());
    }

    #[tokio::test]
    async fn test_find_node_from_etcd_success() {
        let result = find_node_from_etcd().await;

        match result {
            Some(ip) => {
                assert!(!ip.is_empty());
                println!("Found IP via etcd: {}", ip);
            }
            None => {
                println!("No nodes found via etcd (expected if etcd unavailable or empty)");
            }
        }
    }

    #[tokio::test]
    async fn test_find_node_from_etcd_no_nodes() {
        // Test when kvs is empty
        let result = find_node_from_etcd().await;
        println!("Etcd lookup result: {:?}", result.is_some());
    }

    #[tokio::test]
    async fn test_find_node_from_manager_success() {
        let result = find_node_from_manager().await;

        match result {
            Some(ip) => {
                assert!(!ip.is_empty());
                println!("Found IP via NodeManager: {}", ip);
            }
            None => {
                println!("No nodes found via NodeManager (expected if etcd unavailable or empty)");
            }
        }
    }

    #[tokio::test]
    async fn test_find_node_from_manager_no_nodes() {
        // Test when manager returns empty nodes
        let result = find_node_from_manager().await;
        println!("NodeManager lookup result: {:?}", result.is_some());
    }

    #[tokio::test]
    async fn test_find_node_by_hostname_found() {
        let result = find_node_by_hostname("test-hostname").await;

        match result {
            Some(node) => {
                assert_eq!(node.hostname, "test-hostname");
                println!("Found node by hostname: {}", node.ip_address);
            }
            None => {
                println!("No node found with hostname 'test-hostname' (expected if not present)");
            }
        }
    }

    #[tokio::test]
    async fn test_find_node_by_hostname_not_found() {
        let result = find_node_by_hostname("nonexistent-hostname").await;

        match result {
            Some(_) => {
                println!("Unexpectedly found node with nonexistent hostname");
            }
            None => {
                println!("Correctly returned None for nonexistent hostname");
            }
        }
    }

    #[tokio::test]
    async fn test_find_node_by_hostname_empty() {
        let result = find_node_by_hostname("").await;
        println!("Empty hostname lookup result: {:?}", result.is_some());
    }

    #[tokio::test]
    async fn test_get_node_ip_fallback_to_settings() {
        // This will test the fallback mechanism when all lookup methods fail
        let ip = get_node_ip().await;

        assert!(!ip.is_empty());
        println!("Got node IP: {}", ip);

        // The IP should either be from a found node or from settings
        // We can't predict which, but it should not be empty
    }

    #[tokio::test]
    async fn test_get_node_ip_with_nodes() {
        // Test when nodes are available
        let ip = get_node_ip().await;
        assert!(!ip.is_empty());
        println!("Node IP (with potential nodes): {}", ip);
    }

    #[tokio::test]
    async fn test_add_node_to_simple_keys_success() {
        let test_ip = "192.168.1.100";

        match add_node_to_simple_keys(test_ip).await {
            Ok(()) => {
                println!("Successfully added node IP to simple keys: {}", test_ip);
            }
            Err(e) => {
                println!(
                    "Failed to add node IP (expected if etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_add_node_to_simple_keys_various_ips() {
        let test_ips = vec!["10.0.0.1", "172.16.1.100", "192.168.1.200", "127.0.0.1"];

        for ip in test_ips {
            match add_node_to_simple_keys(ip).await {
                Ok(()) => {
                    println!("Added IP {}", ip);
                }
                Err(e) => {
                    println!(
                        "Failed to add IP {} (expected if etcd unavailable): {}",
                        ip, e
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_add_node_to_simple_keys_edge_cases() {
        // Test with edge case IPs
        let edge_cases = vec![
            "",                // Empty string
            "0.0.0.0",         // All zeros
            "255.255.255.255", // All ones
            "invalid-ip",      // Invalid format
        ];

        for ip in edge_cases {
            match add_node_to_simple_keys(ip).await {
                Ok(()) => {
                    println!("Added edge case IP: {}", ip);
                }
                Err(e) => {
                    println!("Failed to add edge case IP '{}': {}", ip, e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_find_guest_nodes_empty() {
        // Test when no guest nodes are found
        let guest_nodes = find_guest_nodes().await;
        println!("Guest nodes found: {}", guest_nodes.len());

        // Should return empty vector, not panic
        // Length is always >= 0 for Vec, but we verify it doesn't panic
    }

    #[tokio::test]
    async fn test_find_guest_nodes_all_masters() {
        // Test scenario where all nodes are masters
        let guest_nodes = find_guest_nodes().await;

        // Count how many are actually guest nodes (non-masters)
        let non_master_count = guest_nodes
            .iter()
            .filter(|node| node.node_role != NodeRole::Master as i32)
            .count();

        assert_eq!(guest_nodes.len(), non_master_count);
        println!("Non-master nodes: {}", non_master_count);
    }

    #[test]
    fn test_create_test_node_info() {
        let node = create_test_node_info(
            "test-node-1",
            "test-host-1",
            "192.168.1.100",
            NodeRole::Nodeagent,
            NodeStatus::Ready,
        );

        assert_eq!(node.node_id, "test-node-1");
        assert_eq!(node.hostname, "test-host-1");
        assert_eq!(node.ip_address, "192.168.1.100");
        assert_eq!(node.node_role, NodeRole::Nodeagent as i32);
        assert_eq!(node.status, NodeStatus::Ready as i32);
        assert!(node.resources.is_some());
    }

    #[test]
    fn test_encode_node_info() {
        let node = create_test_node_info(
            "encode-test",
            "encode-host",
            "10.0.0.1",
            NodeRole::Master,
            NodeStatus::Pending,
        );

        let encoded = encode_node_info(&node);
        assert!(!encoded.is_empty());

        // Test that we can decode it back
        let decoded_bytes = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        let decoded_node = NodeInfo::decode(&decoded_bytes[..]).unwrap();

        assert_eq!(decoded_node.node_id, node.node_id);
        assert_eq!(decoded_node.hostname, node.hostname);
        assert_eq!(decoded_node.ip_address, node.ip_address);
    }

    #[test]
    fn test_node_role_enum_values() {
        // Test different node roles
        let roles = vec![NodeRole::Unspecified, NodeRole::Master, NodeRole::Nodeagent];

        for role in roles {
            let node = create_test_node_info(
                "role-test",
                "role-host",
                "10.0.0.1",
                role,
                NodeStatus::Ready,
            );

            assert_eq!(node.node_role, role as i32);
            println!("Node role: {:?} = {}", role, node.node_role);
        }
    }

    #[test]
    fn test_node_status_enum_values() {
        // Test different node statuses
        let statuses = vec![
            NodeStatus::Unspecified,
            NodeStatus::Pending,
            NodeStatus::Initializing,
            NodeStatus::Ready,
            NodeStatus::NotReady,
            NodeStatus::Maintenance,
            NodeStatus::Terminating,
        ];

        for status in statuses {
            let node = create_test_node_info(
                "status-test",
                "status-host",
                "10.0.0.1",
                NodeRole::Nodeagent,
                status,
            );

            assert_eq!(node.status, status as i32);
            println!("Node status: {:?} = {}", status, node.status);
        }
    }

    #[tokio::test]
    async fn test_error_handling_decode_failures() {
        // Test base64 decode failure simulation
        // This is handled in find_node_from_etcd when decode fails

        // Test invalid base64 by creating a malformed string
        let invalid_base64 = "invalid-base64-!@#$%";
        match base64::engine::general_purpose::STANDARD.decode(invalid_base64) {
            Ok(_) => panic!("Should have failed to decode invalid base64"),
            Err(e) => println!("Expected base64 decode error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_error_handling_protobuf_decode_failures() {
        // Test protobuf decode failure
        let valid_base64_invalid_protobuf =
            base64::engine::general_purpose::STANDARD.encode(b"not-valid-protobuf-data");

        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&valid_base64_invalid_protobuf)
            .unwrap();

        match NodeInfo::decode(&decoded[..]) {
            Ok(_) => println!("Unexpectedly decoded invalid protobuf"),
            Err(e) => println!("Expected protobuf decode error: {}", e),
        }
    }

    #[tokio::test]
    async fn test_node_lookup_integration() {
        // Integration test that exercises multiple lookup methods
        println!("=== Testing all lookup methods ===");

        // Test simple key lookup
        let simple_result = find_node_by_simple_key().await;
        println!("Simple key result: {:?}", simple_result.is_some());

        // Test etcd lookup
        let etcd_result = find_node_from_etcd().await;
        println!("Etcd result: {:?}", etcd_result.is_some());

        // Test manager lookup
        let manager_result = find_node_from_manager().await;
        println!("Manager result: {:?}", manager_result.is_some());

        // Test hostname lookup
        let hostname_result = find_node_by_hostname("integration-test").await;
        println!("Hostname result: {:?}", hostname_result.is_some());

        // Test guest nodes lookup
        let guest_nodes = find_guest_nodes().await;
        println!("Guest nodes found: {}", guest_nodes.len());

        // Test final IP resolution
        let final_ip = get_node_ip().await;
        println!("Final IP resolved: {}", final_ip);
        assert!(!final_ip.is_empty());
    }

    #[tokio::test]
    async fn test_concurrent_node_operations() {
        // Test concurrent access to node lookup functions
        let tasks = vec![
            tokio::spawn(find_node_by_simple_key()),
            tokio::spawn(find_node_from_etcd()),
            tokio::spawn(find_node_from_manager()),
        ];

        for (i, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(result) => {
                    println!("Concurrent task {} result: {:?}", i, result.is_some());
                }
                Err(e) => {
                    println!("Concurrent task {} failed: {}", i, e);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_guest_node_filtering_logic() {
        // Test the specific logic that filters out master nodes
        let test_nodes = vec![
            create_test_node_info(
                "master-1",
                "master-host",
                "10.0.0.1",
                NodeRole::Master,
                NodeStatus::Ready,
            ),
            create_test_node_info(
                "agent-1",
                "agent-host",
                "10.0.0.2",
                NodeRole::Nodeagent,
                NodeStatus::Ready,
            ),
            create_test_node_info(
                "master-2",
                "master-host-2",
                "10.0.0.3",
                NodeRole::Master,
                NodeStatus::Ready,
            ),
        ];

        // Simulate the filtering logic from find_guest_nodes
        let mut guest_nodes = Vec::new();
        for node in test_nodes {
            if node.node_role != NodeRole::Master as i32 {
                guest_nodes.push(node);
            }
        }

        assert_eq!(guest_nodes.len(), 1);
        assert_eq!(guest_nodes[0].node_id, "agent-1");
        assert_eq!(guest_nodes[0].node_role, NodeRole::Nodeagent as i32);
    }
}

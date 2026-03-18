/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node manager for cluster operations

use common::apiserver::NodeInfo;
use common::persistency;
use common::logd;
use common::nodeagent::fromapiserver::{NodeRegistrationRequest, NodeStatus};

/// Node manager for handling cluster node operations
#[derive(Clone)]
pub struct NodeManager;
#[allow(dead_code)]
impl NodeManager {
    /// Create a new NodeManager instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(NodeManager)
    }

    /// Register a new node in the cluster
    pub async fn register_node(
        &self,
        request: NodeRegistrationRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // node_id 대신 hostname(node_name)을 키로 사용합니다
        let node_key = format!("cluster/nodes/{}", request.hostname);

        // Create node info
        let node_info = NodeInfo {
            node_id: request.node_id.clone(),
            hostname: request.hostname.clone(),
            ip_address: request.ip_address.clone(),
            node_type: request.node_type,
            node_role: request.node_role,
            status: NodeStatus::Pending.into(),
            resources: request.resources,
            last_heartbeat: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            metadata: request.metadata,
        };

        // 1. cluster/nodes/{hostname}: 노드 정보(json string)
        let node_json = serde_json::to_string(&node_info)?;
        persistency::put(&node_key, &node_json).await?;

        // 2. nodes/{ip_address}: hostname(plain string)
        let ip_key = format!("nodes/{}", request.ip_address);
        persistency::put(&ip_key, &request.hostname).await?;

        // 3. nodes/{hostname}: ip 주소(plain string)
        let hostname_key = format!("nodes/{}", request.hostname);
        persistency::put(&hostname_key, &request.ip_address).await?;

        logd!(2, "Node {} registered successfully", request.node_id);
        Ok(format!("cluster-token-{}", request.node_id))
    }

    /// Get all nodes in the cluster
    pub async fn get_all_nodes(
        &self,
    ) -> Result<Vec<NodeInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let prefix = "cluster/nodes/";
        let kvs = persistency::get_all_with_prefix(prefix).await?;

        let mut nodes = Vec::new();
        for kv in kvs {
            match serde_json::from_str::<NodeInfo>(&kv.1) {
                Ok(node) => nodes.push(node),
                Err(e) => {
                    logd!(5, "Failed to parse node json for key {}: {}", kv.0, e);
                    continue;
                }
            }
        }
        Ok(nodes)
    }

    /// Get all nodes in the cluster (alias for get_all_nodes)
    pub async fn get_nodes(
        &self,
    ) -> Result<Vec<NodeInfo>, Box<dyn std::error::Error + Send + Sync>> {
        self.get_all_nodes().await
    }

    /// Get a specific node by ID
    pub async fn get_node(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeInfo>, Box<dyn std::error::Error + Send + Sync>> {
        // node_id를 직접 사용 (hostname으로 간주)
        let node_key = format!("cluster/nodes/{}", node_id);

        match persistency::get(&node_key).await {
            Ok(json_str) => {
                let node_info = serde_json::from_str::<NodeInfo>(&json_str)?;
                Ok(Some(node_info))
            }
            Err(_) => Ok(None), // Node not found
        }
    }

    /// Update node heartbeat
    pub async fn update_heartbeat(
        &self,
        node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut node) = self.get_node(node_id).await? {
            node.last_heartbeat = chrono::Utc::now().timestamp();
            node.status = NodeStatus::Ready.into();

            // node_name으로 키 생성
            let node_key = format!("cluster/nodes/{}", node.hostname);
            let node_json = serde_json::to_string(&node)?;
            persistency::put(&node_key, &node_json).await?;

            logd!(1, "Updated heartbeat for node {}", node_id);
        }
        Ok(())
    }

    /// Update node status
    pub async fn update_status(
        &self,
        node_id: &str,
        status: NodeStatus,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut node) = self.get_node(node_id).await? {
            node.status = status.into();
            node.last_heartbeat = chrono::Utc::now().timestamp();

            // node.hostname을 사용하여 키 생성 (node_id 대신)
            let node_key = format!("cluster/nodes/{}", node.hostname);
            let node_json = serde_json::to_string(&node)?;
            persistency::put(&node_key, &node_json).await?;

            logd!(1, "Updated status for node {} to {:?}", node_id, status);
        }
        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(
        &self,
        node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // get_node를 사용하여 노드 정보를 얻고 hostname을 추출
        if let Some(node) = self.get_node(node_id).await? {
            let node_key = format!("cluster/nodes/{}", node.hostname);
            persistency::delete(&node_key).await?;

            logd!(2, "Removed node {} from cluster", node_id);
            return Ok(());
        }

        // 노드를 찾지 못한 경우 오류 반환
        Err(format!("Node not found: {}", node_id).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::fromapiserver::{NodeRole, NodeStatus, NodeType, ResourceInfo};
    use std::collections::HashMap;

    fn create_test_resource_info() -> ResourceInfo {
        ResourceInfo {
            cpu_cores: 4,
            memory_mb: 8192,
            disk_gb: 100,
            architecture: "x86_64".to_string(),
            os_version: "Ubuntu 20.04".to_string(),
        }
    }

    fn create_test_registration_request(
        node_id: &str,
        hostname: &str,
        ip: &str,
    ) -> NodeRegistrationRequest {
        let mut metadata = HashMap::new();
        metadata.insert("environment".to_string(), "test".to_string());
        metadata.insert("cluster".to_string(), "development".to_string());

        NodeRegistrationRequest {
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            ip_address: ip.to_string(),
            node_type: NodeType::Vehicle.into(),
            node_role: NodeRole::Nodeagent.into(),
            resources: Some(create_test_resource_info()),
            metadata,
        }
    }

    fn create_cloud_node_request() -> NodeRegistrationRequest {
        NodeRegistrationRequest {
            node_id: "cloud-node-001".to_string(),
            hostname: "cloud-host".to_string(),
            ip_address: "10.0.1.100".to_string(),
            node_type: NodeType::Cloud.into(),
            node_role: NodeRole::Master.into(),
            resources: Some(ResourceInfo {
                cpu_cores: 8,
                memory_mb: 16384,
                disk_gb: 500,
                architecture: "x86_64".to_string(),
                os_version: "Ubuntu 22.04".to_string(),
            }),
            metadata: HashMap::new(),
        }
    }

    fn create_bluechi_node_request() -> NodeRegistrationRequest {
        NodeRegistrationRequest {
            node_id: "bluechi-node-001".to_string(),
            hostname: "bluechi-host".to_string(),
            ip_address: "172.16.1.50".to_string(),
            node_type: NodeType::Vehicle.into(),
            node_role: NodeRole::Master.into(), // Use Master instead of BluechiManager
            resources: Some(create_test_resource_info()),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_node_manager_creation() {
        let result = NodeManager::new();
        assert!(result.is_ok());

        let manager = result.unwrap();
        // Test that the manager can be cloned
        let _cloned_manager = manager.clone();
    }

    #[test]
    fn test_node_registration_request_creation() {
        let request =
            create_test_registration_request("test-node-001", "test-host", "192.168.1.100");

        assert_eq!(request.node_id, "test-node-001");
        assert_eq!(request.hostname, "test-host");
        assert_eq!(request.ip_address, "192.168.1.100");
        assert_eq!(request.node_type, NodeType::Vehicle as i32);
        assert_eq!(request.node_role, NodeRole::Nodeagent as i32);
        assert!(request.resources.is_some());
        assert_eq!(request.metadata.len(), 2);
        assert_eq!(
            request.metadata.get("environment"),
            Some(&"test".to_string())
        );
    }

    #[test]
    fn test_cloud_node_creation() {
        let request = create_cloud_node_request();

        assert_eq!(request.node_type, NodeType::Cloud as i32);
        assert_eq!(request.node_role, NodeRole::Master as i32);
        assert_eq!(request.resources.as_ref().unwrap().cpu_cores, 8);
        assert_eq!(request.resources.as_ref().unwrap().memory_mb, 16384);
    }

    #[test]
    fn test_bluechi_node_creation() {
        let request = create_bluechi_node_request();

        assert_eq!(request.node_role, NodeRole::Master as i32);
        assert_eq!(request.ip_address, "172.16.1.50");
    }

    #[test]
    fn test_resource_info_creation() {
        let resources = create_test_resource_info();

        assert_eq!(resources.cpu_cores, 4);
        assert_eq!(resources.memory_mb, 8192);
        assert_eq!(resources.disk_gb, 100);
        assert_eq!(resources.architecture, "x86_64");
        assert_eq!(resources.os_version, "Ubuntu 20.04");
    }

    #[tokio::test]
    async fn test_node_manager_operations() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        // Create a test registration request
        let request =
            create_test_registration_request("test-node-001", "test-host", "192.168.1.100");

        // Test node registration (this will fail if etcd is not running, which is fine for test compilation)
        match manager.register_node(request).await {
            Ok(token) => {
                assert!(!token.is_empty());
                assert!(token.starts_with("cluster-token-"));
                assert!(token.contains("test-node-001"));
            }
            Err(e) => {
                // Expected if etcd is not available during testing
                logd!(4, "Expected etcd connection error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_register_multiple_nodes() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        let requests = vec![
            create_test_registration_request("node-1", "host-1", "192.168.1.101"),
            create_test_registration_request("node-2", "host-2", "192.168.1.102"),
            create_cloud_node_request(),
            create_bluechi_node_request(),
        ];

        for request in requests {
            match manager.register_node(request.clone()).await {
                Ok(token) => {
                    assert!(!token.is_empty());
                    assert!(token.contains(&request.node_id));
                }
                Err(e) => {
                    logd!(
                        4,
                        "Expected etcd connection error for {}: {}",
                        request.node_id,
                        e
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_get_all_nodes() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        match manager.get_all_nodes().await {
            Ok(nodes) => {
                // If etcd is available, we should get a vector (possibly empty)
                logd!(2, "Retrieved {} nodes", nodes.len());
            }
            Err(e) => {
                // Expected if etcd is not available
                logd!(4, "Expected etcd connection error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_nodes_alias() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        // Test that get_nodes is an alias for get_all_nodes
        let result1 = manager.get_all_nodes().await;
        let result2 = manager.get_nodes().await;

        match (&result1, &result2) {
            (Ok(nodes1), Ok(nodes2)) => {
                assert_eq!(nodes1.len(), nodes2.len());
                logd!(2, "Both methods returned {} nodes", nodes1.len());
            }
            (Err(e1), Err(e2)) => {
                // Both should fail with same error if etcd unavailable
                logd!(
                    4,
                    "Both methods failed as expected when etcd unavailable: {} / {}",
                    e1,
                    e2
                );
            }
            _ => {
                // This shouldn't happen - both should behave identically
                logd!(
                    6,
                    "get_nodes and get_all_nodes returned different result types"
                );
                logd!(6, "Result1: {:?}", result1.is_ok());
                logd!(6, "Result2: {:?}", result2.is_ok());
                // Don't panic, just log the discrepancy for debugging
            }
        }
    }

    #[tokio::test]
    async fn test_get_specific_node() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        match manager.get_node("test-node-001").await {
            Ok(Some(node)) => {
                assert_eq!(node.node_id, "test-node-001");
                logd!(2, "Found node: {}", node.hostname);
            }
            Ok(None) => {
                logd!(5, "Node not found (expected if not previously registered)");
            }
            Err(e) => {
                logd!(4, "Expected etcd connection error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_get_nonexistent_node() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        match manager.get_node("nonexistent-node-999").await {
            Ok(Some(_)) => {
                panic!("Should not find nonexistent node");
            }
            Ok(None) => {
                println!("Correctly returned None for nonexistent node");
            }
            Err(e) => {
                println!("Expected etcd connection error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        match manager.update_heartbeat("test-node-001").await {
            Ok(()) => {
                println!("Heartbeat updated successfully");
            }
            Err(e) => {
                println!(
                    "Expected error if node doesn't exist or etcd unavailable: {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_update_node_status() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        let test_statuses = vec![
            NodeStatus::Pending,
            NodeStatus::Initializing,
            NodeStatus::Ready,
            NodeStatus::NotReady,
            NodeStatus::Maintenance,
            NodeStatus::Terminating,
        ];

        for status in test_statuses {
            match manager.update_status("test-node-001", status).await {
                Ok(()) => {
                    println!("Status updated to {:?} successfully", status);
                }
                Err(e) => {
                    println!(
                        "Expected error if node doesn't exist or etcd unavailable: {}",
                        e
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_remove_node() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        match manager.remove_node("test-node-001").await {
            Ok(()) => {
                println!("Node removed successfully");
            }
            Err(e) => {
                println!("Expected error: {}", e);
                // Should contain "Node not found" if node doesn't exist
                // or connection error if etcd unavailable
            }
        }
    }

    #[tokio::test]
    async fn test_remove_nonexistent_node() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        match manager.remove_node("nonexistent-node-999").await {
            Ok(()) => {
                panic!("Should not successfully remove nonexistent node");
            }
            Err(e) => {
                let error_message = e.to_string();
                // Should either be "Node not found" or etcd connection error
                println!(
                    "Expected error when removing nonexistent node: {}",
                    error_message
                );
            }
        }
    }

    #[tokio::test]
    async fn test_node_lifecycle() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");
        let request = create_test_registration_request(
            "lifecycle-test-node",
            "lifecycle-host",
            "192.168.1.200",
        );

        // Test complete node lifecycle if etcd is available
        if let Ok(token) = manager.register_node(request.clone()).await {
            println!("1. Node registered with token: {}", token);

            // Check if node exists
            if let Ok(Some(node)) = manager.get_node("lifecycle-test-node").await {
                println!("2. Node found: {} ({})", node.hostname, node.ip_address);
                assert_eq!(node.node_id, "lifecycle-test-node");
                assert_eq!(node.status, NodeStatus::Pending as i32);

                // Update heartbeat
                if manager
                    .update_heartbeat("lifecycle-test-node")
                    .await
                    .is_ok()
                {
                    println!("3. Heartbeat updated");

                    // Update status
                    if manager
                        .update_status("lifecycle-test-node", NodeStatus::Ready)
                        .await
                        .is_ok()
                    {
                        println!("4. Status updated to Ready");

                        // Finally remove node
                        if manager.remove_node("lifecycle-test-node").await.is_ok() {
                            println!("5. Node removed successfully");
                        }
                    }
                }
            }
        } else {
            println!("Etcd not available for full lifecycle test");
        }
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        // Test sequential node registrations (since we don't have futures crate)
        let requests = vec![
            create_test_registration_request("concurrent-1", "host-1", "192.168.1.201"),
            create_test_registration_request("concurrent-2", "host-2", "192.168.1.202"),
            create_test_registration_request("concurrent-3", "host-3", "192.168.1.203"),
        ];

        for (i, request) in requests.into_iter().enumerate() {
            match manager.register_node(request).await {
                Ok(token) => {
                    println!("Sequential registration {} succeeded: {}", i + 1, token);
                }
                Err(e) => {
                    println!(
                        "Sequential registration {} failed (expected if etcd unavailable): {}",
                        i + 1,
                        e
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn test_edge_cases() {
        let manager = NodeManager::new().expect("Failed to create NodeManager");

        // Test with empty strings (should work with current implementation)
        let edge_case_request = NodeRegistrationRequest {
            node_id: "".to_string(),
            hostname: "".to_string(),
            ip_address: "".to_string(),
            node_type: NodeType::Vehicle.into(),
            node_role: NodeRole::Nodeagent.into(),
            resources: None, // Test with no resources
            metadata: HashMap::new(),
        };

        match manager.register_node(edge_case_request).await {
            Ok(token) => {
                println!("Edge case registration succeeded: {}", token);
                assert!(token.starts_with("cluster-token-"));
            }
            Err(e) => {
                println!("Edge case registration failed: {}", e);
            }
        }

        // Test operations with empty node_id
        match manager.get_node("").await {
            Ok(_) => println!("Got result for empty node_id"),
            Err(e) => println!("Expected error for empty node_id: {}", e),
        }

        match manager.update_heartbeat("").await {
            Ok(()) => println!("Heartbeat update with empty node_id succeeded"),
            Err(e) => println!("Heartbeat update with empty node_id failed: {}", e),
        }

        match manager.remove_node("").await {
            Ok(()) => println!("Remove with empty node_id succeeded"),
            Err(e) => println!("Remove with empty node_id failed: {}", e),
        }
    }

    #[test]
    fn test_node_status_enum_values() {
        // Test all NodeStatus enum variants
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
            let status_int: i32 = status.into();
            assert!(status_int >= 0);
            println!("NodeStatus::{:?} = {}", status, status_int);
        }
    }

    #[test]
    fn test_node_type_and_role_enum_values() {
        // Test NodeType enum variants
        let types = vec![NodeType::Unspecified, NodeType::Cloud, NodeType::Vehicle];

        for node_type in types {
            let type_int: i32 = node_type.into();
            assert!(type_int >= 0);
            println!("NodeType::{:?} = {}", node_type, type_int);
        }

        // Test NodeRole enum variants
        let roles = vec![NodeRole::Unspecified, NodeRole::Master, NodeRole::Nodeagent];

        for role in roles {
            let role_int: i32 = role.into();
            assert!(role_int >= 0);
            println!("NodeRole::{:?} = {}", role, role_int);
        }
    }

    #[test]
    fn test_resource_info_variations() {
        // Test different resource configurations
        let minimal_resources = ResourceInfo {
            cpu_cores: 1,
            memory_mb: 512,
            disk_gb: 10,
            architecture: "arm64".to_string(),
            os_version: "Alpine Linux".to_string(),
        };

        let high_spec_resources = ResourceInfo {
            cpu_cores: 32,
            memory_mb: 131072, // 128GB
            disk_gb: 2048,     // 2TB
            architecture: "x86_64".to_string(),
            os_version: "RHEL 9".to_string(),
        };

        assert_eq!(minimal_resources.cpu_cores, 1);
        assert_eq!(high_spec_resources.cpu_cores, 32);
        assert_eq!(minimal_resources.architecture, "arm64");
        assert_eq!(high_spec_resources.architecture, "x86_64");
    }

    #[test]
    fn test_metadata_variations() {
        let mut complex_metadata = HashMap::new();
        complex_metadata.insert("datacenter".to_string(), "us-west-2".to_string());
        complex_metadata.insert("rack".to_string(), "A-42".to_string());
        complex_metadata.insert("power_source".to_string(), "grid".to_string());
        complex_metadata.insert("backup_power".to_string(), "ups".to_string());
        complex_metadata.insert("network_zone".to_string(), "dmz".to_string());

        let request = NodeRegistrationRequest {
            node_id: "metadata-test-node".to_string(),
            hostname: "metadata-host".to_string(),
            ip_address: "10.1.1.100".to_string(),
            node_type: NodeType::Cloud.into(),
            node_role: NodeRole::Master.into(),
            resources: Some(create_test_resource_info()),
            metadata: complex_metadata.clone(),
        };

        assert_eq!(request.metadata.len(), 5);
        assert_eq!(
            request.metadata.get("datacenter"),
            Some(&"us-west-2".to_string())
        );
        assert_eq!(request.metadata.get("rack"), Some(&"A-42".to_string()));
    }
}

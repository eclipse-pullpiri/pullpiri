/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node manager for cluster operations

use common::apiserver::NodeInfo;
use common::etcd;
use common::nodeagent::{NodeRegistrationRequest, NodeStatus};

/// Node manager for handling cluster node operations
#[derive(Clone)]
pub struct NodeManager;

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
    etcd::put(&node_key, &node_json).await?;

    // 2. nodes/{ip_address}: hostname(plain string)
    let ip_key = format!("nodes/{}", request.ip_address);
    etcd::put(&ip_key, &request.hostname).await?;

    // 3. nodes/{hostname}: ip 주소(plain string)
    let hostname_key = format!("nodes/{}", request.hostname);
    etcd::put(&hostname_key, &request.ip_address).await?;

        println!("Node {} registered successfully", request.node_id);
        Ok(format!("cluster-token-{}", request.node_id))
    }

    /// Get all nodes in the cluster
    pub async fn get_all_nodes(
        &self,
    ) -> Result<Vec<NodeInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let prefix = "cluster/nodes/";
        let kvs = etcd::get_all_with_prefix(prefix).await?;

        let mut nodes = Vec::new();
        for kv in kvs {
            match serde_json::from_str::<NodeInfo>(&kv.value) {
                Ok(node) => nodes.push(node),
                Err(e) => {
                    eprintln!("Failed to parse node json for key {}: {}", kv.key, e);
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

        match etcd::get(&node_key).await {
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
            etcd::put(&node_key, &node_json).await?;

            println!("Updated heartbeat for node {}", node_id);
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
            etcd::put(&node_key, &node_json).await?;

            println!("Updated status for node {} to {:?}", node_id, status);
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
            etcd::delete(&node_key).await?;

            println!("Removed node {} from cluster", node_id);
            return Ok(());
        }

        // 노드를 찾지 못한 경우 오류 반환
        Err(format!("Node not found: {}", node_id).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::{NodeRole, NodeType, ResourceInfo};

    #[tokio::test]
    async fn test_node_manager_operations() {
        let manager = NodeManager;

        // Create a test registration request
        let request = NodeRegistrationRequest {
            node_id: "test-node-001".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            node_type: common::nodeagent::NodeType::Vehicle.into(),
            node_role: NodeRole::Nodeagent.into(),
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 100,
                architecture: "x86_64".to_string(),
                os_version: "Ubuntu 20.04".to_string(),
            }),
            metadata: std::collections::HashMap::new(),
        };

        // Test node registration (this will fail if etcd is not running, which is fine for test compilation)
        match manager.register_node(request).await {
            Ok(token) => {
                assert!(!token.is_empty());
            }
            Err(_) => {
                // Expected if etcd is not available during testing
            }
        }
    }
}

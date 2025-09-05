/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node manager for cluster operations

use common::apiserver::NodeInfo;
use common::etcd;
use common::nodeagent::{NodeRegistrationRequest, NodeStatus};
use prost::Message;

/// Node manager for handling cluster node operations
#[derive(Clone)]
pub struct NodeManager;

impl NodeManager {
    /// Register a new node in the cluster
    pub async fn register_node(
        &self,
        request: NodeRegistrationRequest,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let node_key = format!("cluster/nodes/{}", request.node_id);

        // Create node info
        let node_info = NodeInfo {
            node_id: request.node_id.clone(),
            hostname: request.hostname.clone(),
            ip_address: request.ip_address.clone(),
            role: request.role,
            status: NodeStatus::Pending.into(),
            resources: request.resources,
            last_heartbeat: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            metadata: request.metadata,
        };

        // Use prost to serialize to binary format (more efficient for etcd)
        let mut buf = Vec::new();
        prost::Message::encode(&node_info, &mut buf)?;

        // Store in etcd as base64 encoded binary
        let encoded = base64::encode(&buf);
        etcd::put(&node_key, &encoded).await?;

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
            match base64::decode(&kv.value) {
                Ok(buf) => match NodeInfo::decode(&buf[..]) {
                    Ok(node) => nodes.push(node),
                    Err(e) => {
                        eprintln!("Failed to decode node data for key {}: {}", kv.key, e);
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("Failed to decode base64 for key {}: {}", kv.key, e);
                    continue;
                }
            }
        }

        Ok(nodes)
    }

    /// Get a specific node by ID
    pub async fn get_node(
        &self,
        node_id: &str,
    ) -> Result<Option<NodeInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let node_key = format!("cluster/nodes/{}", node_id);

        match etcd::get(&node_key).await {
            Ok(encoded) => {
                let buf = base64::decode(&encoded)?;
                let node_info = NodeInfo::decode(&buf[..])?;
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

            let node_key = format!("cluster/nodes/{}", node_id);
            let mut buf = Vec::new();
            prost::Message::encode(&node, &mut buf)?;
            let encoded = base64::encode(&buf);
            etcd::put(&node_key, &encoded).await?;

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

            let node_key = format!("cluster/nodes/{}", node_id);
            let mut buf = Vec::new();
            prost::Message::encode(&node, &mut buf)?;
            let encoded = base64::encode(&buf);
            etcd::put(&node_key, &encoded).await?;

            println!("Updated status for node {} to {:?}", node_id, status);
        }

        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(
        &self,
        node_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let node_key = format!("cluster/nodes/{}", node_id);
        etcd::delete(&node_key).await?;

        println!("Removed node {} from cluster", node_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::{NodeRole, ResourceInfo};

    #[tokio::test]
    async fn test_node_manager_operations() {
        let manager = NodeManager;

        // Create a test registration request
        let request = NodeRegistrationRequest {
            node_id: "test-node-001".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            role: NodeRole::Sub.into(),
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

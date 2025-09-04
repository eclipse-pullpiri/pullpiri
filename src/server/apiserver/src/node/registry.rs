/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node Registry Module
//! 
//! Handles node registration, authentication, and basic node information management.
//! This module provides a lightweight registry suitable for embedded environments.

use common::nodeagent::{
    NodeInfo, NodeRegistrationRequest, NodeRegistrationResponse, NodeStatus, 
    ClusterInfo, Credentials
};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, anyhow};

/// Node registry for managing cluster nodes
#[derive(Clone)]
pub struct NodeRegistry {
    nodes: Arc<RwLock<HashMap<String, NodeInfo>>>,
    cluster_info: Arc<RwLock<ClusterInfo>>,
}

impl NodeRegistry {
    /// Create a new node registry
    pub fn new() -> Self {
        let cluster_info = ClusterInfo {
            cluster_id: "piccolo-cluster".to_string(),
            cluster_name: "PICCOLO Embedded Cluster".to_string(),
            master_endpoint: "localhost:47100".to_string(), // API server gRPC port
            nodes: vec![],
        };

        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            cluster_info: Arc::new(RwLock::new(cluster_info)),
        }
    }

    /// Register a new node in the cluster
    pub async fn register_node(&self, request: NodeRegistrationRequest) -> Result<NodeRegistrationResponse> {
        // Basic validation
        if request.hostname.is_empty() || request.ip_address.is_empty() {
            return Err(anyhow!("Invalid node information: hostname and IP address are required"));
        }

        // Simple authentication check (in production this would be more sophisticated)
        if !self.authenticate_node(&request.credentials).await? {
            return Err(anyhow!("Authentication failed"));
        }

        // Generate node ID if not provided
        let node_id = if request.node_id.is_empty() {
            format!("node-{}-{}", request.hostname, chrono::Utc::now().timestamp())
        } else {
            request.node_id
        };

        // Create node info
        let node_info = NodeInfo {
            node_id: node_id.clone(),
            hostname: request.hostname,
            ip_address: request.ip_address,
            role: request.role.into(),
            status: NodeStatus::Initializing.into(),
            last_seen: chrono::Utc::now().timestamp(),
        };

        // Store node information
        {
            let mut nodes = self.nodes.write().await;
            nodes.insert(node_id.clone(), node_info.clone());
        }

        // Update cluster info
        {
            let mut cluster_info = self.cluster_info.write().await;
            cluster_info.nodes.push(node_info.clone());
        }

        log::info!("Node registered: {} ({})", node_id, node_info.hostname);

        // Return registration response
        let cluster_info = self.cluster_info.read().await.clone();
        Ok(NodeRegistrationResponse {
            node_id,
            cluster_info: Some(cluster_info),
            status: NodeStatus::Initializing.into(),
            message: "Node registered successfully".to_string(),
        })
    }

    /// Get all nodes in the cluster
    pub async fn get_nodes(&self) -> Result<Vec<NodeInfo>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.values().cloned().collect())
    }

    /// Get specific node by ID
    pub async fn get_node(&self, node_id: &str) -> Result<Option<NodeInfo>> {
        let nodes = self.nodes.read().await;
        Ok(nodes.get(node_id).cloned())
    }

    /// Update node status
    pub async fn update_node_status(&self, node_id: &str, status: NodeStatus) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(node_id) {
            node.status = status.into();
            node.last_seen = chrono::Utc::now().timestamp();
            log::debug!("Updated node {} status to {:?}", node_id, status);
            Ok(())
        } else {
            Err(anyhow!("Node {} not found", node_id))
        }
    }

    /// Record node heartbeat
    pub async fn record_heartbeat(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(node_id) {
            node.last_seen = chrono::Utc::now().timestamp();
            
            // Update status to ready if it was initializing
            if node.status == NodeStatus::Initializing as i32 {
                node.status = NodeStatus::Ready.into();
                log::info!("Node {} is now ready", node_id);
            }
            Ok(())
        } else {
            Err(anyhow!("Node {} not found", node_id))
        }
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        
        if nodes.remove(node_id).is_some() {
            // Update cluster info
            let mut cluster_info = self.cluster_info.write().await;
            cluster_info.nodes.retain(|n| n.node_id != node_id);
            
            log::info!("Node removed: {}", node_id);
            Ok(())
        } else {
            Err(anyhow!("Node {} not found", node_id))
        }
    }

    /// Get cluster information
    pub async fn get_cluster_info(&self) -> Result<ClusterInfo> {
        Ok(self.cluster_info.read().await.clone())
    }

    /// Check for inactive nodes and mark them as not ready
    pub async fn check_node_health(&self, timeout_seconds: i64) -> Result<Vec<String>> {
        let mut unhealthy_nodes = Vec::new();
        let current_time = chrono::Utc::now().timestamp();
        let mut nodes = self.nodes.write().await;

        for (node_id, node) in nodes.iter_mut() {
            if current_time - node.last_seen > timeout_seconds {
                if node.status == NodeStatus::Ready as i32 {
                    node.status = NodeStatus::NotReady.into();
                    unhealthy_nodes.push(node_id.clone());
                    log::warn!("Node {} marked as unhealthy (last seen: {})", node_id, node.last_seen);
                }
            }
        }

        Ok(unhealthy_nodes)
    }

    /// Simple authentication check
    async fn authenticate_node(&self, credentials: &Option<Credentials>) -> Result<bool> {
        // In a production environment, this would validate certificates, tokens, etc.
        // For now, we'll accept any credentials or no credentials for simplicity
        match credentials {
            Some(creds) => {
                // Basic token validation
                if creds.token.is_empty() {
                    log::warn!("Empty authentication token provided");
                    return Ok(false);
                }
                // In real implementation, validate token against known values
                Ok(true)
            },
            None => {
                log::warn!("No credentials provided, allowing for development");
                Ok(true) // Allow for development/testing
            }
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::ResourceInfo;

    #[tokio::test]
    async fn test_node_registration() {
        let registry = NodeRegistry::new();
        
        let request = NodeRegistrationRequest {
            node_id: "test-node".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            role: NodeRole::Sub.into(),
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 2048,
                disk_gb: 100,
                architecture: "x86_64".to_string(),
                platform: "Linux".to_string(),
            }),
            credentials: Some(Credentials {
                token: "test-token".to_string(),
                certificate: "".to_string(),
                expires_at: 0,
            }),
        };

        let response = registry.register_node(request).await.unwrap();
        assert_eq!(response.node_id, "test-node");
        assert!(response.cluster_info.is_some());

        // Verify node is in registry
        let node = registry.get_node("test-node").await.unwrap();
        assert!(node.is_some());
        assert_eq!(node.unwrap().hostname, "test-host");
    }

    #[tokio::test]
    async fn test_heartbeat_updates_status() {
        let registry = NodeRegistry::new();
        
        let request = NodeRegistrationRequest {
            node_id: "heartbeat-test".to_string(),
            hostname: "heartbeat-host".to_string(),
            ip_address: "192.168.1.101".to_string(),
            role: NodeRole::Sub.into(),
            resources: None,
            credentials: None,
        };

        registry.register_node(request).await.unwrap();
        
        // Initially should be initializing
        let node = registry.get_node("heartbeat-test").await.unwrap().unwrap();
        assert_eq!(node.status, NodeStatus::Initializing as i32);

        // Send heartbeat should make it ready
        registry.record_heartbeat("heartbeat-test").await.unwrap();
        
        let node = registry.get_node("heartbeat-test").await.unwrap().unwrap();
        assert_eq!(node.status, NodeStatus::Ready as i32);
    }
}
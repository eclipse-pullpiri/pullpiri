/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node registry for cluster management using etcd

use super::{NodeInfo, NodeRole, NodeStatus, ClusterTopology, TopologyType};
use common::{etcd, Result};
use serde_json;
use std::collections::HashMap;

const NODES_PREFIX: &str = "/piccolo/cluster/nodes";
const TOPOLOGY_PREFIX: &str = "/piccolo/cluster/topology";
const HEARTBEAT_TIMEOUT_SECONDS: i64 = 90; // 90 seconds timeout for heartbeats

/// Node registry for managing cluster nodes
pub struct NodeRegistry {
    // Etcd operations are handled through the common::etcd module
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self {}
    }

    /// Initialize the node registry with etcd connection
    pub async fn initialize(&self) -> Result<()> {
        // We'll establish connection when needed since etcd module handles connection internally
        println!("Node registry initialized");
        Ok(())
    }

    /// Register a new node in the cluster
    pub async fn register_node(&self, mut node_info: NodeInfo) -> Result<String> {
        // Generate cluster ID if it's the first master node
        let cluster_id = self.get_or_create_cluster_id().await?;

        // Set node status to initializing during registration
        node_info.status = NodeStatus::Initializing;
        node_info.update_heartbeat();

        let key = format!("{}/{}", NODES_PREFIX, node_info.node_id);
        let value = serde_json::to_string(&node_info)?;

        etcd::put(&key, &value).await
            .map_err(|e| format!("Failed to register node: {}", e))?;

        println!("Registered node: {} with role: {:?}", node_info.node_name, node_info.role);
        Ok(cluster_id)
    }

    /// Update node status (typically for heartbeats)
    pub async fn update_node_status(&self, node_id: &str, status: NodeStatus, metrics: Option<HashMap<String, String>>) -> Result<()> {
        let mut node_info = self.get_node(node_id).await?;
        node_info.status = status;
        node_info.update_heartbeat();

        // Update resource metrics if provided
        if let Some(metrics) = metrics {
            if let Ok(cpu_usage) = metrics.get("cpu_usage").unwrap_or(&"0.0".to_string()).parse::<f64>() {
                node_info.resources.cpu_usage = cpu_usage;
            }
            if let Ok(memory_usage) = metrics.get("memory_usage").unwrap_or(&"0.0".to_string()).parse::<f64>() {
                node_info.resources.memory_usage = memory_usage;
            }
        }

        let key = format!("{}/{}", NODES_PREFIX, node_id);
        let value = serde_json::to_string(&node_info)?;

        etcd::put(&key, &value).await
            .map_err(|e| format!("Failed to update node status: {}", e))?;

        Ok(())
    }

    /// Get a specific node by ID
    pub async fn get_node(&self, node_id: &str) -> Result<NodeInfo> {
        let key = format!("{}/{}", NODES_PREFIX, node_id);
        
        match etcd::get(&key).await {
            Ok(value) => {
                let node_info: NodeInfo = serde_json::from_str(&value)?;
                Ok(node_info)
            },
            Err(e) => Err(format!("Failed to get node: {}", e).into()),
        }
    }

    /// Get all nodes in the cluster
    pub async fn get_all_nodes(&self) -> Result<Vec<NodeInfo>> {
        match etcd::get_all_with_prefix(NODES_PREFIX).await {
            Ok(kvs) => {
                let mut nodes = Vec::new();
                for kv in kvs {
                    if let Ok(node_info) = serde_json::from_str::<NodeInfo>(&kv.value) {
                        nodes.push(node_info);
                    }
                }
                Ok(nodes)
            },
            Err(e) => Err(format!("Failed to get nodes: {}", e).into()),
        }
    }

    /// Get nodes filtered by status
    pub async fn get_nodes_by_status(&self, status_filter: NodeStatus) -> Result<Vec<NodeInfo>> {
        let all_nodes = self.get_all_nodes().await?;
        Ok(all_nodes.into_iter().filter(|node| node.status == status_filter).collect())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        let key = format!("{}/{}", NODES_PREFIX, node_id);
        etcd::delete(&key).await
            .map_err(|e| format!("Failed to remove node: {}", e))?;

        println!("Removed node: {}", node_id);
        Ok(())
    }

    /// Get cluster topology
    pub async fn get_cluster_topology(&self, cluster_id: &str) -> Result<ClusterTopology> {
        let all_nodes = self.get_all_nodes().await?;
        
        let mut master_nodes = Vec::new();
        let mut sub_nodes = Vec::new();

        for node in all_nodes {
            match node.role {
                NodeRole::Master => master_nodes.push(node),
                NodeRole::Sub => sub_nodes.push(node),
            }
        }

        Ok(ClusterTopology {
            cluster_id: cluster_id.to_string(),
            cluster_name: "piccolo-cluster".to_string(),
            topology_type: TopologyType::Simple,
            master_nodes,
            sub_nodes,
            config: HashMap::new(),
        })
    }

    /// Check for stale nodes and mark them as offline
    pub async fn check_stale_nodes(&self) -> Result<Vec<String>> {
        let mut stale_nodes = Vec::new();
        let all_nodes = self.get_all_nodes().await?;
        let current_time = chrono::Utc::now().timestamp();

        for node in all_nodes {
            let heartbeat_age = current_time - node.last_heartbeat;
            if heartbeat_age > HEARTBEAT_TIMEOUT_SECONDS && node.is_online() {
                // Mark node as offline
                self.update_node_status(&node.node_id, NodeStatus::Offline, None).await?;
                stale_nodes.push(node.node_id);
            }
        }

        if !stale_nodes.is_empty() {
            println!("Marked {} nodes as offline due to stale heartbeats", stale_nodes.len());
        }

        Ok(stale_nodes)
    }

    /// Get or create cluster ID
    async fn get_or_create_cluster_id(&self) -> Result<String> {
        let key = format!("{}/default", TOPOLOGY_PREFIX);
        
        match etcd::get(&key).await {
            Ok(value) => Ok(value),
            Err(_) => {
                // Create new cluster ID
                let cluster_id = format!("piccolo-cluster-{}", chrono::Utc::now().timestamp());
                etcd::put(&key, &cluster_id).await
                    .map_err(|e| format!("Failed to create cluster ID: {}", e))?;
                Ok(cluster_id)
            }
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
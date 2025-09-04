/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::apiserver::{ClusterInfo, ClusterTopology, Node, NodeRole, NodeStatus, TopologyType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Node registry for cluster management
#[derive(Clone)]
pub struct NodeRegistry {
    nodes: Arc<RwLock<HashMap<String, Node>>>,
    cluster_info: Arc<RwLock<ClusterInfo>>,
    topology: Arc<RwLock<ClusterTopology>>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        let cluster_info = ClusterInfo {
            cluster_id: "piccolo-cluster-001".to_string(),
            cluster_name: "PICCOLO Embedded Cluster".to_string(),
            master_endpoint: "localhost:47098".to_string(),
        };

        let topology = ClusterTopology {
            id: "topology-001".to_string(),
            name: "Basic Embedded Topology".to_string(),
            r#type: TopologyType::BasicEmbedded as i32,
            master_nodes: Vec::new(),
            sub_nodes: Vec::new(),
            parent_cluster: None,
            config: HashMap::new(),
        };

        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            cluster_info: Arc::new(RwLock::new(cluster_info)),
            topology: Arc::new(RwLock::new(topology)),
        }
    }

    /// Register a new node in the cluster
    pub async fn register_node(&self, mut node: Node) -> Result<String, String> {
        let mut nodes = self.nodes.write().await;

        // Validate node information
        if node.id.is_empty() || node.name.is_empty() || node.ip.is_empty() {
            return Err("Invalid node information: id, name, and ip are required".to_string());
        }

        // Check if node already exists
        if nodes.contains_key(&node.id) {
            return Err(format!("Node with id {} already exists", node.id));
        }

        // Set registration timestamp and initial status
        node.created_at = chrono::Utc::now().timestamp();
        node.last_heartbeat = node.created_at;
        node.status = NodeStatus::Initializing as i32;

        let node_id = node.id.clone();
        nodes.insert(node_id.clone(), node.clone());

        // Update topology
        let mut topology = self.topology.write().await;
        match NodeRole::try_from(node.role) {
            Ok(NodeRole::Master) => {
                if !topology.master_nodes.iter().any(|n| n.id == node.id) {
                    topology.master_nodes.push(node.clone());
                }
            }
            Ok(NodeRole::Sub) => {
                if !topology.sub_nodes.iter().any(|n| n.id == node.id) {
                    topology.sub_nodes.push(node.clone());
                }
            }
            Err(_) => {
                return Err("Invalid node role".to_string());
            }
        }

        println!("Node {} ({}) registered successfully", node_id, node.name);
        Ok(node_id)
    }

    /// Get a node by ID
    pub async fn get_node(&self, node_id: &str) -> Option<Node> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).cloned()
    }

    /// Get all nodes with optional filtering
    pub async fn get_nodes(&self, filter: Option<&str>) -> Vec<Node> {
        let nodes = self.nodes.read().await;
        let mut result: Vec<Node> = nodes.values().cloned().collect();

        if let Some(filter_str) = filter {
            result.retain(|node| {
                node.name.contains(filter_str)
                    || node.ip.contains(filter_str)
                    || format!(
                        "{:?}",
                        NodeRole::try_from(node.role).unwrap_or(NodeRole::Sub)
                    )
                    .contains(filter_str)
            });
        }

        result
    }

    /// Update node heartbeat
    pub async fn update_heartbeat(&self, node_id: &str) -> Result<(), String> {
        let mut nodes = self.nodes.write().await;

        if let Some(node) = nodes.get_mut(node_id) {
            node.last_heartbeat = chrono::Utc::now().timestamp();

            // Update status to Ready if it was Initializing
            if node.status == NodeStatus::Initializing as i32 {
                node.status = NodeStatus::Ready as i32;
                println!("Node {} is now Ready", node_id);
            }

            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    /// Update node status
    pub async fn update_node_status(
        &self,
        node_id: &str,
        status: NodeStatus,
    ) -> Result<(), String> {
        let mut nodes = self.nodes.write().await;

        if let Some(node) = nodes.get_mut(node_id) {
            node.status = status as i32;
            println!("Node {} status updated to {:?}", node_id, status);
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    /// Get cluster information
    pub async fn get_cluster_info(&self) -> ClusterInfo {
        let cluster_info = self.cluster_info.read().await;
        cluster_info.clone()
    }

    /// Get cluster topology
    pub async fn get_topology(&self) -> ClusterTopology {
        let topology = self.topology.read().await;
        topology.clone()
    }

    /// Update cluster topology
    pub async fn update_topology(
        &self,
        new_topology: ClusterTopology,
    ) -> Result<ClusterTopology, String> {
        let mut topology = self.topology.write().await;
        *topology = new_topology.clone();
        println!("Cluster topology updated");
        Ok(new_topology)
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &str) -> Result<(), String> {
        let mut nodes = self.nodes.write().await;

        if nodes.remove(node_id).is_some() {
            // Update topology
            let mut topology = self.topology.write().await;
            topology.master_nodes.retain(|n| n.id != node_id);
            topology.sub_nodes.retain(|n| n.id != node_id);

            println!("Node {} removed from cluster", node_id);
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    /// Check for unhealthy nodes (no heartbeat for > 30 seconds)
    pub async fn check_node_health(&self) -> Vec<String> {
        let mut nodes = self.nodes.write().await;
        let current_time = chrono::Utc::now().timestamp();
        let timeout = 30; // 30 seconds timeout
        let mut unhealthy_nodes = Vec::new();

        for (node_id, node) in nodes.iter_mut() {
            if current_time - node.last_heartbeat > timeout {
                if node.status != NodeStatus::NotReady as i32 {
                    node.status = NodeStatus::NotReady as i32;
                    unhealthy_nodes.push(node_id.clone());
                    println!(
                        "Node {} marked as NotReady due to missing heartbeat",
                        node_id
                    );
                }
            }
        }

        unhealthy_nodes
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
    use common::apiserver::ResourceInfo;

    #[tokio::test]
    async fn test_node_registration() {
        let registry = NodeRegistry::new();

        let node = Node {
            id: "test-node-001".to_string(),
            name: "Test Node".to_string(),
            ip: "192.168.1.100".to_string(),
            role: NodeRole::Sub as i32,
            status: NodeStatus::Pending as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 256,
            }),
            created_at: 0,
            last_heartbeat: 0,
        };

        let result = registry.register_node(node).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-node-001");

        let retrieved_node = registry.get_node("test-node-001").await;
        assert!(retrieved_node.is_some());
        assert_eq!(retrieved_node.unwrap().name, "Test Node");
    }

    #[tokio::test]
    async fn test_heartbeat_update() {
        let registry = NodeRegistry::new();

        let node = Node {
            id: "test-node-002".to_string(),
            name: "Test Node 2".to_string(),
            ip: "192.168.1.101".to_string(),
            role: NodeRole::Sub as i32,
            status: NodeStatus::Pending as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 2,
                memory_mb: 4096,
                disk_gb: 128,
            }),
            created_at: 0,
            last_heartbeat: 0,
        };

        registry.register_node(node).await.unwrap();

        let result = registry.update_heartbeat("test-node-002").await;
        assert!(result.is_ok());

        let updated_node = registry.get_node("test-node-002").await.unwrap();
        assert!(updated_node.last_heartbeat > 0);
        assert_eq!(updated_node.status, NodeStatus::Ready as i32);
    }
}

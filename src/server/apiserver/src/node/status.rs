/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::node::NodeRegistry;
use common::apiserver::NodeStatus;
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Node status manager for monitoring and health checks
#[derive(Clone)]
pub struct NodeStatusManager {
    registry: Arc<NodeRegistry>,
}

impl NodeStatusManager {
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        Self { registry }
    }

    /// Start background health monitoring
    pub async fn start_health_monitoring(&self) {
        let registry = self.registry.clone();
        let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds

        loop {
            interval.tick().await;

            let unhealthy_nodes = registry.check_node_health().await;
            if !unhealthy_nodes.is_empty() {
                println!(
                    "Health check found {} unhealthy nodes: {:?}",
                    unhealthy_nodes.len(),
                    unhealthy_nodes
                );

                // Here you could send alerts or take recovery actions
                self.handle_unhealthy_nodes(unhealthy_nodes).await;
            }
        }
    }

    /// Handle unhealthy nodes
    async fn handle_unhealthy_nodes(&self, unhealthy_nodes: Vec<String>) {
        for node_id in unhealthy_nodes {
            println!("Attempting to recover unhealthy node: {}", node_id);

            // Mark node as maintenance for investigation
            if let Err(e) = self
                .registry
                .update_node_status(&node_id, NodeStatus::Maintenance)
                .await
            {
                eprintln!("Failed to update status for node {}: {}", node_id, e);
            }

            // Here you could implement recovery strategies:
            // - Send restart command to NodeAgent
            // - Attempt to re-establish connection
            // - Remove node if permanently unreachable
        }
    }

    /// Update node heartbeat
    pub async fn process_heartbeat(&self, node_id: &str) -> Result<(), String> {
        self.registry.update_heartbeat(node_id).await
    }

    /// Update node status with additional metrics
    pub async fn process_status_report(
        &self,
        node_id: &str,
        status_data: &std::collections::HashMap<String, String>,
    ) -> Result<(), String> {
        // Process metrics and update node status accordingly
        let mut should_mark_ready = true;

        // Check CPU usage
        if let Some(cpu_str) = status_data.get("cpu_usage") {
            if let Ok(cpu_usage) = cpu_str.parse::<f64>() {
                if cpu_usage > 95.0 {
                    should_mark_ready = false;
                    println!("Node {} has high CPU usage: {}%", node_id, cpu_usage);
                }
            }
        }

        // Check memory usage
        if let Some(mem_str) = status_data.get("memory_usage") {
            if let Ok(memory_usage) = mem_str.parse::<f64>() {
                if memory_usage > 90.0 {
                    should_mark_ready = false;
                    println!("Node {} has high memory usage: {}%", node_id, memory_usage);
                }
            }
        }

        // Update heartbeat
        self.registry.update_heartbeat(node_id).await?;

        // Update status based on metrics
        if !should_mark_ready {
            if let Some(node) = self.registry.get_node(node_id).await {
                if node.status == NodeStatus::Ready as i32 {
                    self.registry
                        .update_node_status(node_id, NodeStatus::NotReady)
                        .await?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apiserver::{Node, NodeRole, ResourceInfo};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_heartbeat_processing() {
        let registry = Arc::new(NodeRegistry::new());
        let status_manager = NodeStatusManager::new(registry.clone());

        // Register a test node
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

        registry.register_node(node).await.unwrap();

        // Process heartbeat
        let result = status_manager.process_heartbeat("test-node-001").await;
        assert!(result.is_ok());

        // Check that the node is now ready
        let updated_node = registry.get_node("test-node-001").await.unwrap();
        assert_eq!(updated_node.status, NodeStatus::Ready as i32);
    }

    #[tokio::test]
    async fn test_status_report_high_cpu() {
        let registry = Arc::new(NodeRegistry::new());
        let status_manager = NodeStatusManager::new(registry.clone());

        // Register a test node and make it ready
        let node = Node {
            id: "test-node-002".to_string(),
            name: "Test Node 2".to_string(),
            ip: "192.168.1.101".to_string(),
            role: NodeRole::Sub as i32,
            status: NodeStatus::Ready as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 2,
                memory_mb: 4096,
                disk_gb: 128,
            }),
            created_at: 0,
            last_heartbeat: 0,
        };

        registry.register_node(node).await.unwrap();
        registry
            .update_node_status("test-node-002", NodeStatus::Ready)
            .await
            .unwrap();

        // Process status report with high CPU
        let mut status_data = HashMap::new();
        status_data.insert("cpu_usage".to_string(), "98.5".to_string());
        status_data.insert("memory_usage".to_string(), "45.0".to_string());

        let result = status_manager
            .process_status_report("test-node-002", &status_data)
            .await;
        assert!(result.is_ok());

        // Check that the node is marked as not ready due to high CPU
        let updated_node = registry.get_node("test-node-002").await.unwrap();
        assert_eq!(updated_node.status, NodeStatus::NotReady as i32);
    }
}

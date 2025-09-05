/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node status monitoring and management

use common::apiserver::NodeInfo;
use common::nodeagent::NodeStatus;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Node status manager for monitoring cluster health
pub struct NodeStatusManager;

impl NodeStatusManager {
    /// Check if a node is healthy based on last heartbeat
    pub fn is_node_healthy(&self, node: &NodeInfo, heartbeat_timeout_seconds: u64) -> bool {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as i64;

        let time_since_heartbeat = current_time - node.last_heartbeat;
        time_since_heartbeat < heartbeat_timeout_seconds as i64
    }

    /// Get unhealthy nodes in the cluster
    pub fn get_unhealthy_nodes(
        &self,
        nodes: &[NodeInfo],
        heartbeat_timeout_seconds: u64,
    ) -> Vec<String> {
        nodes
            .iter()
            .filter(|node| !self.is_node_healthy(node, heartbeat_timeout_seconds))
            .map(|node| node.node_id.clone())
            .collect()
    }

    /// Get cluster health summary
    pub fn get_cluster_health_summary(
        &self,
        nodes: &[NodeInfo],
        heartbeat_timeout_seconds: u64,
    ) -> ClusterHealthSummary {
        let total_nodes = nodes.len();
        let healthy_nodes = nodes
            .iter()
            .filter(|node| self.is_node_healthy(node, heartbeat_timeout_seconds))
            .count();
        let unhealthy_nodes = total_nodes - healthy_nodes;

        let master_nodes = nodes
            .iter()
            .filter(|node| node.role == common::nodeagent::NodeRole::Master as i32)
            .count();

        let sub_nodes = nodes
            .iter()
            .filter(|node| node.role == common::nodeagent::NodeRole::Sub as i32)
            .count();

        let ready_nodes = nodes
            .iter()
            .filter(|node| node.status == NodeStatus::Ready as i32)
            .count();

        ClusterHealthSummary {
            total_nodes,
            healthy_nodes,
            unhealthy_nodes,
            master_nodes,
            sub_nodes,
            ready_nodes,
            cluster_status: if unhealthy_nodes == 0 {
                ClusterStatus::Healthy
            } else if healthy_nodes > 0 {
                ClusterStatus::Degraded
            } else {
                ClusterStatus::Critical
            },
        }
    }

    /// Convert status string to NodeStatus enum
    pub fn parse_node_status(&self, status: &str) -> NodeStatus {
        match status.to_lowercase().as_str() {
            "pending" => NodeStatus::Pending,
            "initializing" => NodeStatus::Initializing,
            "ready" => NodeStatus::Ready,
            "not_ready" | "notready" => NodeStatus::NotReady,
            "maintenance" => NodeStatus::Maintenance,
            "terminating" => NodeStatus::Terminating,
            _ => NodeStatus::Unspecified,
        }
    }
}

/// Cluster health summary
#[derive(Debug, Clone)]
pub struct ClusterHealthSummary {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub unhealthy_nodes: usize,
    pub master_nodes: usize,
    pub sub_nodes: usize,
    pub ready_nodes: usize,
    pub cluster_status: ClusterStatus,
}

/// Overall cluster status
#[derive(Debug, Clone, PartialEq)]
pub enum ClusterStatus {
    Healthy,
    Degraded,
    Critical,
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::{NodeRole, ResourceInfo};

    fn create_test_node(node_id: &str, last_heartbeat: i64, status: NodeStatus) -> NodeInfo {
        NodeInfo {
            node_id: node_id.to_string(),
            hostname: format!("host-{}", node_id),
            ip_address: "192.168.1.100".to_string(),
            role: NodeRole::Sub.into(),
            status: status.into(),
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 100,
                architecture: "x86_64".to_string(),
                os_version: "Ubuntu 20.04".to_string(),
            }),
            last_heartbeat,
            created_at: 1234567890,
            metadata: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_node_health_check() {
        let status_manager = NodeStatusManager;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Healthy node (recent heartbeat)
        let healthy_node = create_test_node("node1", current_time - 10, NodeStatus::Ready);
        assert!(status_manager.is_node_healthy(&healthy_node, 60));

        // Unhealthy node (old heartbeat)
        let unhealthy_node = create_test_node("node2", current_time - 120, NodeStatus::Ready);
        assert!(!status_manager.is_node_healthy(&unhealthy_node, 60));
    }

    #[test]
    fn test_cluster_health_summary() {
        let status_manager = NodeStatusManager;
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let nodes = vec![
            create_test_node("node1", current_time - 10, NodeStatus::Ready),
            create_test_node("node2", current_time - 120, NodeStatus::NotReady),
            create_test_node("node3", current_time - 5, NodeStatus::Ready),
        ];

        let summary = status_manager.get_cluster_health_summary(&nodes, 60);

        assert_eq!(summary.total_nodes, 3);
        assert_eq!(summary.healthy_nodes, 2);
        assert_eq!(summary.unhealthy_nodes, 1);
        assert_eq!(summary.cluster_status, ClusterStatus::Degraded);
    }

    #[test]
    fn test_status_parsing() {
        let status_manager = NodeStatusManager;

        assert_eq!(status_manager.parse_node_status("ready"), NodeStatus::Ready);
        assert_eq!(
            status_manager.parse_node_status("PENDING"),
            NodeStatus::Pending
        );
        assert_eq!(
            status_manager.parse_node_status("not_ready"),
            NodeStatus::NotReady
        );
        assert_eq!(
            status_manager.parse_node_status("unknown"),
            NodeStatus::Unspecified
        );
    }
}

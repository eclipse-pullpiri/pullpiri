/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node Status Module
//!
//! Handles node status monitoring, heartbeat processing, and health checks.
//! Optimized for embedded environments with minimal resource usage.

use anyhow::{anyhow, Result};
use common::nodeagent::{
    AlertInfo, HeartbeatRequest, HeartbeatResponse, NodeStatus, StatusAck, StatusReport,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Node status manager for tracking node health and metrics
#[derive(Clone)]
pub struct NodeStatusManager {
    node_metrics: Arc<RwLock<HashMap<String, NodeMetrics>>>,
    heartbeat_timeout: i64,
}

/// Metrics tracked for each node
#[derive(Clone, Debug)]
pub struct NodeMetrics {
    pub node_id: String,
    pub last_heartbeat: i64,
    pub status: NodeStatus,
    pub container_count: usize,
    pub alerts: Vec<AlertInfo>,
    pub metrics: HashMap<String, String>,
    pub consecutive_failures: u32,
}

impl NodeStatusManager {
    /// Create a new node status manager
    pub fn new(heartbeat_timeout_seconds: i64) -> Self {
        Self {
            node_metrics: Arc::new(RwLock::new(HashMap::new())),
            heartbeat_timeout: heartbeat_timeout_seconds,
        }
    }

    /// Process heartbeat from a node
    pub async fn process_heartbeat(&self, request: HeartbeatRequest) -> Result<HeartbeatResponse> {
        let current_time = chrono::Utc::now().timestamp();

        let mut metrics = self.node_metrics.write().await;
        let node_metrics = metrics
            .entry(request.node_id.clone())
            .or_insert_with(|| NodeMetrics {
                node_id: request.node_id.clone(),
                last_heartbeat: 0,
                status: NodeStatus::Pending,
                container_count: 0,
                alerts: Vec::new(),
                metrics: HashMap::new(),
                consecutive_failures: 0,
            });

        // Update heartbeat info
        node_metrics.last_heartbeat = current_time;
        node_metrics.status = request.status();
        node_metrics.consecutive_failures = 0; // Reset failure count on successful heartbeat

        log::debug!("Heartbeat received from node: {}", request.node_id);

        Ok(HeartbeatResponse {
            success: true,
            message: "Heartbeat acknowledged".to_string(),
            server_timestamp: current_time,
        })
    }

    /// Process status report from a node
    pub async fn process_status_report(&self, report: StatusReport) -> Result<StatusAck> {
        let mut metrics = self.node_metrics.write().await;
        let node_metrics = metrics
            .entry(report.node_id.clone())
            .or_insert_with(|| NodeMetrics {
                node_id: report.node_id.clone(),
                last_heartbeat: 0,
                status: NodeStatus::Pending,
                container_count: 0,
                alerts: Vec::new(),
                metrics: HashMap::new(),
                consecutive_failures: 0,
            });

        // Update node metrics
        node_metrics.last_heartbeat = report.timestamp;
        node_metrics.status = report.status();
        node_metrics.container_count = report.containers.len();
        node_metrics.alerts = report.alerts.clone();
        node_metrics.metrics = report.metrics.clone();

        // Log container status if any
        if !report.containers.is_empty() {
            log::debug!(
                "Node {} reported {} containers",
                report.node_id,
                report.containers.len()
            );
            for container in &report.containers {
                log::trace!(
                    "Container {}: {} ({})",
                    container.name,
                    container.status,
                    container.image
                );
            }
        }

        // Log alerts if any
        if !report.alerts.is_empty() {
            log::warn!(
                "Node {} reported {} alerts",
                report.node_id,
                report.alerts.len()
            );
            for alert in &report.alerts {
                log::warn!("Alert: {} - {}", alert.severity, alert.message);
            }
        }

        Ok(StatusAck {
            received: true,
            message: "Status report processed successfully".to_string(),
        })
    }

    /// Get current status of a specific node
    pub async fn get_node_status(&self, node_id: &str) -> Result<Option<NodeMetrics>> {
        let metrics = self.node_metrics.read().await;
        Ok(metrics.get(node_id).cloned())
    }

    /// Get status of all nodes
    pub async fn get_all_node_status(&self) -> Result<Vec<NodeMetrics>> {
        let metrics = self.node_metrics.read().await;
        Ok(metrics.values().cloned().collect())
    }

    /// Check for unhealthy nodes based on heartbeat timeout
    pub async fn check_node_health(&self) -> Result<Vec<String>> {
        let current_time = chrono::Utc::now().timestamp();
        let mut unhealthy_nodes = Vec::new();

        let mut metrics = self.node_metrics.write().await;

        for (node_id, node_metrics) in metrics.iter_mut() {
            let time_since_heartbeat = current_time - node_metrics.last_heartbeat;

            if time_since_heartbeat > self.heartbeat_timeout {
                // Node is considered unhealthy
                if node_metrics.status == NodeStatus::Ready {
                    node_metrics.status = NodeStatus::NotReady;
                    node_metrics.consecutive_failures += 1;
                    unhealthy_nodes.push(node_id.clone());

                    log::warn!(
                        "Node {} marked as unhealthy: {} seconds since last heartbeat (consecutive failures: {})",
                        node_id,
                        time_since_heartbeat,
                        node_metrics.consecutive_failures
                    );
                }
            }
        }

        Ok(unhealthy_nodes)
    }

    /// Get nodes that have been unhealthy for an extended period and should be removed
    pub async fn get_failed_nodes(&self, max_consecutive_failures: u32) -> Result<Vec<String>> {
        let metrics = self.node_metrics.read().await;
        let failed_nodes: Vec<String> = metrics
            .iter()
            .filter(|(_, node_metrics)| {
                node_metrics.consecutive_failures >= max_consecutive_failures
                    && node_metrics.status == NodeStatus::NotReady
            })
            .map(|(node_id, _)| node_id.clone())
            .collect();

        if !failed_nodes.is_empty() {
            log::warn!(
                "Found {} failed nodes that may need removal",
                failed_nodes.len()
            );
        }

        Ok(failed_nodes)
    }

    /// Remove node metrics (when node is removed from cluster)
    pub async fn remove_node_metrics(&self, node_id: &str) -> Result<()> {
        let mut metrics = self.node_metrics.write().await;
        if metrics.remove(node_id).is_some() {
            log::info!("Removed metrics for node: {}", node_id);
            Ok(())
        } else {
            Err(anyhow!("Node metrics not found: {}", node_id))
        }
    }

    /// Get cluster-wide health summary
    pub async fn get_cluster_health_summary(&self) -> Result<ClusterHealthSummary> {
        let metrics = self.node_metrics.read().await;
        let mut summary = ClusterHealthSummary::default();

        for node_metrics in metrics.values() {
            summary.total_nodes += 1;

            match node_metrics.status {
                NodeStatus::Ready => summary.ready_nodes += 1,
                NodeStatus::NotReady => summary.not_ready_nodes += 1,
                NodeStatus::Initializing => summary.initializing_nodes += 1,
                NodeStatus::Maintenance => summary.maintenance_nodes += 1,
                _ => summary.other_nodes += 1,
            }

            summary.total_containers += node_metrics.container_count;
            summary.total_alerts += node_metrics.alerts.len();
        }

        summary.healthy = summary.not_ready_nodes == 0 && summary.total_nodes > 0;

        Ok(summary)
    }

    /// Start background health monitoring task
    pub async fn start_health_monitoring(&self, check_interval_seconds: u64) {
        let status_manager = self.clone();

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(check_interval_seconds));

            loop {
                interval.tick().await;

                match status_manager.check_node_health().await {
                    Ok(unhealthy_nodes) => {
                        if !unhealthy_nodes.is_empty() {
                            log::warn!(
                                "Health check found {} unhealthy nodes",
                                unhealthy_nodes.len()
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("Error during health check: {}", e);
                    }
                }

                // Check for nodes that should be removed
                match status_manager.get_failed_nodes(3).await {
                    // 3 consecutive failures
                    Ok(failed_nodes) => {
                        for node_id in failed_nodes {
                            log::error!(
                                "Node {} has failed consistently and may need manual intervention",
                                node_id
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("Error checking failed nodes: {}", e);
                    }
                }
            }
        });
    }
}

/// Cluster health summary
#[derive(Default, Debug)]
pub struct ClusterHealthSummary {
    pub healthy: bool,
    pub total_nodes: usize,
    pub ready_nodes: usize,
    pub not_ready_nodes: usize,
    pub initializing_nodes: usize,
    pub maintenance_nodes: usize,
    pub other_nodes: usize,
    pub total_containers: usize,
    pub total_alerts: usize,
}

impl ClusterHealthSummary {
    pub fn ready_percentage(&self) -> f64 {
        if self.total_nodes == 0 {
            0.0
        } else {
            (self.ready_nodes as f64 / self.total_nodes as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::{AlertInfo, ContainerStatus};

    #[tokio::test]
    async fn test_heartbeat_processing() {
        let status_manager = NodeStatusManager::new(30);

        let heartbeat = HeartbeatRequest {
            node_id: "test-node".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            status: NodeStatus::Ready.into(),
        };

        let response = status_manager.process_heartbeat(heartbeat).await.unwrap();
        assert!(response.success);

        let metrics = status_manager.get_node_status("test-node").await.unwrap();
        assert!(metrics.is_some());
        let metrics = metrics.unwrap();
        assert_eq!(metrics.status, NodeStatus::Ready);
        assert_eq!(metrics.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_status_report_processing() {
        let status_manager = NodeStatusManager::new(30);

        let containers = vec![ContainerStatus {
            container_id: "container1".to_string(),
            name: "test-app".to_string(),
            image: "test-image:latest".to_string(),
            status: "running".to_string(),
            cpu_percent: 50,
            memory_usage: 512,
        }];

        let alerts = vec![AlertInfo {
            alert_id: "alert1".to_string(),
            severity: "warning".to_string(),
            message: "High CPU usage".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }];

        let mut metrics_map = HashMap::new();
        metrics_map.insert("cpu_usage".to_string(), "75%".to_string());
        metrics_map.insert("memory_usage".to_string(), "60%".to_string());

        let report = StatusReport {
            node_id: "test-node".to_string(),
            status: NodeStatus::Ready.into(),
            metrics: metrics_map,
            containers,
            alerts,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let ack = status_manager.process_status_report(report).await.unwrap();
        assert!(ack.received);

        let metrics = status_manager.get_node_status("test-node").await.unwrap();
        assert!(metrics.is_some());
        let metrics = metrics.unwrap();
        assert_eq!(metrics.container_count, 1);
        assert_eq!(metrics.alerts.len(), 1);
        assert!(metrics.metrics.contains_key("cpu_usage"));
    }

    #[tokio::test]
    async fn test_health_check() {
        let status_manager = NodeStatusManager::new(1); // 1 second timeout for testing

        // Add a node with old heartbeat
        let old_heartbeat = HeartbeatRequest {
            node_id: "old-node".to_string(),
            timestamp: chrono::Utc::now().timestamp() - 10, // 10 seconds ago
            status: NodeStatus::Ready.into(),
        };

        status_manager
            .process_heartbeat(old_heartbeat)
            .await
            .unwrap();

        // Manually set old timestamp to simulate timeout
        {
            let mut metrics = status_manager.node_metrics.write().await;
            if let Some(node_metrics) = metrics.get_mut("old-node") {
                node_metrics.last_heartbeat = chrono::Utc::now().timestamp() - 10;
            }
        }

        // Wait a bit and check health
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let unhealthy = status_manager.check_node_health().await.unwrap();
        assert!(unhealthy.contains(&"old-node".to_string()));
    }
}

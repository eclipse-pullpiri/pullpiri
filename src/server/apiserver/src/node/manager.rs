/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node Manager Module
//! 
//! Main coordinator for node management, combining registry and status management.
//! Provides the primary interface for node clustering operations.

use crate::node::{registry::NodeRegistry, status::NodeStatusManager};
use common::nodeagent::{
    NodeRegistrationRequest, NodeRegistrationResponse, NodeStatus, 
    StatusReport, StatusAck, HeartbeatRequest, HeartbeatResponse,
};
use common::apiserver::{
    GetNodesRequest, GetNodesResponse, GetNodeRequest, GetNodeResponse,
    UpdateNodeStatusRequest, UpdateNodeStatusResponse, GetTopologyRequest,
    GetTopologyResponse, ClusterTopology, TopologyType, HealthCheckRequest, HealthCheckResponse
};
use std::collections::HashMap;
use anyhow::Result;

/// Main node manager that coordinates all node operations
#[derive(Clone)]
pub struct NodeManager {
    registry: NodeRegistry,
    status_manager: NodeStatusManager,
}

impl NodeManager {
    /// Create a new node manager
    pub fn new() -> Self {
        Self {
            registry: NodeRegistry::new(),
            status_manager: NodeStatusManager::new(30), // 30 second heartbeat timeout
        }
    }

    /// Initialize the node manager and start background tasks
    pub async fn initialize(&self) -> Result<()> {
        log::info!("Initializing Node Manager");
        
        // Start health monitoring background task
        self.status_manager.start_health_monitoring(10).await; // Check every 10 seconds
        
        log::info!("Node Manager initialized successfully");
        Ok(())
    }

    /// Register a new node in the cluster
    pub async fn register_node(&self, request: NodeRegistrationRequest) -> Result<NodeRegistrationResponse> {
        log::info!("Registering node: {} ({})", request.hostname, request.ip_address);
        
        // Register with the registry
        let response = self.registry.register_node(request).await?;
        
        log::info!("Node registration completed: {}", response.node_id);
        Ok(response)
    }

    /// Process heartbeat from a node
    pub async fn process_heartbeat(&self, request: HeartbeatRequest) -> Result<HeartbeatResponse> {
        log::debug!("Processing heartbeat from node: {}", request.node_id);
        
        // Process in status manager
        let response = self.status_manager.process_heartbeat(request.clone()).await?;
        
        // Update registry with heartbeat info
        self.registry.record_heartbeat(&request.node_id).await?;
        
        Ok(response)
    }

    /// Process status report from a node
    pub async fn process_status_report(&self, report: StatusReport) -> Result<StatusAck> {
        log::debug!("Processing status report from node: {}", report.node_id);
        
        // Process in status manager
        let ack = self.status_manager.process_status_report(report.clone()).await?;
        
        // Update registry status if needed
        if let Some(node) = self.registry.get_node(&report.node_id).await? {
            if node.status != report.status {
                self.registry.update_node_status(&report.node_id, report.status()).await?;
            }
        }
        
        Ok(ack)
    }

    /// Get all nodes with optional filtering
    pub async fn get_nodes(&self, request: GetNodesRequest) -> Result<GetNodesResponse> {
        let mut nodes = self.registry.get_nodes().await?;
        
        // Apply filters if provided
        if let Some(status_filter) = request.status_filter {
            nodes.retain(|node| node.status == status_filter);
        }
        
        if let Some(role_filter) = request.role_filter {
            nodes.retain(|node| node.role == role_filter);
        }
        
        if let Some(filter) = &request.filter {
            if !filter.is_empty() {
                nodes.retain(|node| {
                    node.hostname.contains(filter) || 
                    node.ip_address.contains(filter) ||
                    node.node_id.contains(filter)
                });
            }
        }
        
        Ok(GetNodesResponse {
            nodes: nodes.clone(),
            success: true,
            message: format!("Found {} nodes", nodes.len()),
        })
    }

    /// Get specific node information
    pub async fn get_node(&self, request: GetNodeRequest) -> Result<GetNodeResponse> {
        match self.registry.get_node(&request.node_id).await? {
            Some(node) => Ok(GetNodeResponse {
                node: Some(node),
                success: true,
                message: "Node found".to_string(),
            }),
            None => Ok(GetNodeResponse {
                node: None,
                success: false,
                message: format!("Node {} not found", request.node_id),
            })
        }
    }

    /// Update node status
    pub async fn update_node_status(&self, request: UpdateNodeStatusRequest) -> Result<UpdateNodeStatusResponse> {
        match self.registry.update_node_status(&request.node_id, request.status()).await {
            Ok(_) => {
                log::info!("Updated node {} status to {:?}: {}", 
                          request.node_id, request.status(), request.reason);
                Ok(UpdateNodeStatusResponse {
                    success: true,
                    message: "Node status updated successfully".to_string(),
                })
            },
            Err(e) => Ok(UpdateNodeStatusResponse {
                success: false,
                message: format!("Failed to update node status: {}", e),
            })
        }
    }

    /// Get cluster topology
    pub async fn get_topology(&self, _request: GetTopologyRequest) -> Result<GetTopologyResponse> {
        let all_nodes = self.registry.get_nodes().await?;
        
        let mut master_nodes = Vec::new();
        let mut sub_nodes = Vec::new();
        
        for node in all_nodes {
            match node.role() {
                common::nodeagent::NodeRole::Master => master_nodes.push(node),
                common::nodeagent::NodeRole::Sub => sub_nodes.push(node),
                _ => sub_nodes.push(node), // Default to sub node
            }
        }

        let topology = ClusterTopology {
            cluster_id: "piccolo-cluster".to_string(),
            cluster_name: "PICCOLO Embedded Cluster".to_string(),
            r#type: TopologyType::BasicEmbedded.into(),
            master_nodes,
            sub_nodes,
            parent_cluster: None,
            config: HashMap::new(),
        };

        Ok(GetTopologyResponse {
            topology: Some(topology),
            success: true,
            message: "Topology retrieved successfully".to_string(),
        })
    }

    /// Health check for the clustering service
    pub async fn health_check(&self, _request: HealthCheckRequest) -> Result<HealthCheckResponse> {
        let health_summary = self.status_manager.get_cluster_health_summary().await?;
        let cluster_info = self.registry.get_cluster_info().await?;
        
        let mut details = HashMap::new();
        details.insert("total_nodes".to_string(), health_summary.total_nodes.to_string());
        details.insert("ready_nodes".to_string(), health_summary.ready_nodes.to_string());
        details.insert("ready_percentage".to_string(), 
                      format!("{:.1}%", health_summary.ready_percentage()));
        details.insert("total_containers".to_string(), health_summary.total_containers.to_string());
        details.insert("total_alerts".to_string(), health_summary.total_alerts.to_string());
        details.insert("cluster_id".to_string(), cluster_info.cluster_id);

        let status = if health_summary.healthy {
            "healthy".to_string()
        } else {
            format!("unhealthy: {}/{} nodes ready", 
                   health_summary.ready_nodes, health_summary.total_nodes)
        };

        Ok(HealthCheckResponse {
            healthy: health_summary.healthy,
            status,
            details,
        })
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &str) -> Result<()> {
        log::info!("Removing node from cluster: {}", node_id);
        
        // Remove from registry
        self.registry.remove_node(node_id).await?;
        
        // Remove metrics
        self.status_manager.remove_node_metrics(node_id).await?;
        
        log::info!("Node {} removed successfully", node_id);
        Ok(())
    }

    /// Start background maintenance tasks
    pub async fn start_maintenance_tasks(&self) {
        let manager = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60)); // Every minute
            
            loop {
                interval.tick().await;
                
                // Check for unhealthy nodes
                match manager.registry.check_node_health(120).await { // 2 minute timeout
                    Ok(unhealthy_nodes) => {
                        for node_id in unhealthy_nodes {
                            log::warn!("Node {} is unhealthy", node_id);
                        }
                    },
                    Err(e) => {
                        log::error!("Error checking node health: {}", e);
                    }
                }

                // Check for failed nodes that might need removal
                match manager.status_manager.get_failed_nodes(5).await { // 5 consecutive failures
                    Ok(failed_nodes) => {
                        for node_id in failed_nodes {
                            log::error!("Node {} has failed consistently - manual intervention may be required", node_id);
                            // In a production system, this might trigger automatic node removal
                            // or send alerts to administrators
                        }
                    },
                    Err(e) => {
                        log::error!("Error checking failed nodes: {}", e);
                    }
                }
            }
        });
    }

    /// Get registry reference (for internal use)
    pub fn registry(&self) -> &NodeRegistry {
        &self.registry
    }

    /// Get status manager reference (for internal use)
    pub fn status_manager(&self) -> &NodeStatusManager {
        &self.status_manager
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::{NodeRole, ResourceInfo, Credentials};

    #[tokio::test]
    async fn test_node_registration_flow() {
        let manager = NodeManager::new();
        manager.initialize().await.unwrap();

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

        let response = manager.register_node(request).await.unwrap();
        assert_eq!(response.node_id, "test-node");
        assert!(response.cluster_info.is_some());

        // Test getting the node
        let get_request = GetNodeRequest {
            node_id: "test-node".to_string(),
        };
        let get_response = manager.get_node(get_request).await.unwrap();
        assert!(get_response.success);
        assert!(get_response.node.is_some());
    }

    #[tokio::test]
    async fn test_heartbeat_flow() {
        let manager = NodeManager::new();
        manager.initialize().await.unwrap();

        // First register a node
        let reg_request = NodeRegistrationRequest {
            node_id: "heartbeat-test".to_string(),
            hostname: "heartbeat-host".to_string(),
            ip_address: "192.168.1.101".to_string(),
            role: NodeRole::Sub.into(),
            resources: None,
            credentials: None,
        };

        manager.register_node(reg_request).await.unwrap();

        // Send heartbeat
        let heartbeat = HeartbeatRequest {
            node_id: "heartbeat-test".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            status: NodeStatus::Ready.into(),
        };

        let response = manager.process_heartbeat(heartbeat).await.unwrap();
        assert!(response.success);

        // Check that node status was updated
        let get_request = GetNodeRequest {
            node_id: "heartbeat-test".to_string(),
        };
        let get_response = manager.get_node(get_request).await.unwrap();
        let node = get_response.node.unwrap();
        assert_eq!(node.status, NodeStatus::Ready as i32);
    }

    #[tokio::test]
    async fn test_health_check() {
        let manager = NodeManager::new();
        manager.initialize().await.unwrap();

        let health_response = manager.health_check(HealthCheckRequest {}).await.unwrap();
        
        // Should be healthy even with no nodes (empty cluster)
        assert!(health_response.details.contains_key("total_nodes"));
        assert!(health_response.details.contains_key("cluster_id"));
    }
}
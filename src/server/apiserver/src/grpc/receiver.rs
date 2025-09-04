/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::node::{NodeManager, NodeRegistry};
use common::apiserver::api_server_service_server::ApiServerService;
use common::apiserver::{
    GetNodeRequest, GetNodeResponse, GetNodesRequest, GetNodesResponse, GetTopologyRequest,
    GetTopologyResponse, NodeRegistrationRequest, NodeRegistrationResponse, UpdateTopologyRequest,
    UpdateTopologyResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

/// API Server gRPC service handler for clustering
#[derive(Clone)]
pub struct ApiServerReceiver {
    node_manager: Arc<NodeManager>,
    node_registry: Arc<NodeRegistry>,
}

impl ApiServerReceiver {
    pub fn new(node_registry: Arc<NodeRegistry>) -> Self {
        let node_manager = Arc::new(NodeManager::new(node_registry.clone()));
        Self {
            node_manager,
            node_registry,
        }
    }
}

#[tonic::async_trait]
impl ApiServerService for ApiServerReceiver {
    /// Get all nodes with optional filtering
    async fn get_nodes(
        &self,
        request: Request<GetNodesRequest>,
    ) -> Result<Response<GetNodesResponse>, Status> {
        println!("Received GetNodes request");
        let req = request.into_inner();

        let filter = req.filter.as_deref();
        let nodes = self.node_registry.get_nodes(filter).await;

        Ok(Response::new(GetNodesResponse { nodes }))
    }

    /// Get a specific node by ID
    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeResponse>, Status> {
        println!(
            "Received GetNode request for node: {}",
            request.get_ref().node_id
        );
        let req = request.into_inner();

        if let Some(node) = self.node_registry.get_node(&req.node_id).await {
            Ok(Response::new(GetNodeResponse { node: Some(node) }))
        } else {
            Err(Status::not_found(format!("Node {} not found", req.node_id)))
        }
    }

    /// Register a new node in the cluster
    async fn register_node(
        &self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        println!(
            "Received node registration request for: {}",
            request.get_ref().hostname
        );
        let req = request.into_inner();

        match self.node_manager.register_node(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                eprintln!("Node registration failed: {}", e);
                Err(Status::invalid_argument(e))
            }
        }
    }

    /// Get cluster topology
    async fn get_topology(
        &self,
        _request: Request<GetTopologyRequest>,
    ) -> Result<Response<GetTopologyResponse>, Status> {
        println!("Received GetTopology request");

        let topology = self.node_registry.get_topology().await;
        Ok(Response::new(GetTopologyResponse {
            topology: Some(topology),
        }))
    }

    /// Update cluster topology
    async fn update_topology(
        &self,
        request: Request<UpdateTopologyRequest>,
    ) -> Result<Response<UpdateTopologyResponse>, Status> {
        println!("Received UpdateTopology request");
        let req = request.into_inner();

        if let Some(topology) = req.topology {
            match self.node_registry.update_topology(topology).await {
                Ok(updated_topology) => Ok(Response::new(UpdateTopologyResponse {
                    updated_topology: Some(updated_topology),
                    success: true,
                    error_message: None,
                })),
                Err(e) => {
                    eprintln!("Failed to update topology: {}", e);
                    Err(Status::internal(e))
                }
            }
        } else {
            Err(Status::invalid_argument("Topology data is required"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apiserver::{Credentials, Node, NodeRole, NodeStatus, ResourceInfo};

    #[tokio::test]
    async fn test_get_nodes() {
        let registry = Arc::new(NodeRegistry::new());
        let receiver = ApiServerReceiver::new(registry.clone());

        // Register a test node first
        let node = Node {
            id: "test-node-001".to_string(),
            name: "Test Node".to_string(),
            ip: "192.168.1.100".to_string(),
            role: NodeRole::Sub as i32,
            status: NodeStatus::Ready as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 256,
            }),
            created_at: 0,
            last_heartbeat: 0,
        };
        registry.register_node(node).await.unwrap();

        // Test GetNodes
        let request = Request::new(GetNodesRequest { filter: None });
        let response = receiver.get_nodes(request).await.unwrap();

        assert_eq!(response.into_inner().nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_node_registration() {
        let registry = Arc::new(NodeRegistry::new());
        let receiver = ApiServerReceiver::new(registry);

        let request = Request::new(NodeRegistrationRequest {
            node_id: "test-node-002".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.101".to_string(),
            role: NodeRole::Sub as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 2,
                memory_mb: 4096,
                disk_gb: 128,
            }),
            credentials: Some(Credentials {
                token: "piccolo-cluster-token".to_string(),
                certificate: "".to_string(),
            }),
        });

        let response = receiver.register_node(request).await;
        assert!(response.is_ok());

        let resp_inner = response.unwrap().into_inner();
        assert!(resp_inner.success);
        assert_eq!(resp_inner.node_id, "test-node-002");
    }
}

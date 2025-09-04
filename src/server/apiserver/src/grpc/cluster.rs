/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC Clustering Server
//! 
//! Implements the gRPC services for node clustering functionality.

use crate::node::NodeManager;
use common::apiserver::{
    api_server_cluster_server::{ApiServerCluster, ApiServerClusterServer},
    GetNodesRequest, GetNodesResponse, GetNodeRequest, GetNodeResponse,
    UpdateNodeStatusRequest, UpdateNodeStatusResponse, GetTopologyRequest,
    GetTopologyResponse, UpdateTopologyRequest, UpdateTopologyResponse,
    HealthCheckRequest, HealthCheckResponse,
};
use common::nodeagent::{
    NodeRegistrationRequest, NodeRegistrationResponse,
};
use tonic::{Request, Response, Status, transport::Server};
use std::sync::Arc;
use anyhow::Result;

/// gRPC server implementation for clustering services
pub struct ClusteringServer {
    node_manager: Arc<NodeManager>,
}

impl ClusteringServer {
    /// Create a new clustering server
    pub fn new(node_manager: Arc<NodeManager>) -> Self {
        Self { node_manager }
    }

    /// Start the gRPC server
    pub async fn start(node_manager: Arc<NodeManager>, addr: std::net::SocketAddr) -> Result<()> {
        let clustering_server = ClusteringServer::new(node_manager);
        
        log::info!("Starting gRPC clustering server on {}", addr);
        
        Server::builder()
            .add_service(ApiServerClusterServer::new(clustering_server))
            .serve(addr)
            .await
            .map_err(|e| anyhow::anyhow!("gRPC server error: {}", e))?;
            
        Ok(())
    }
}

#[tonic::async_trait]
impl ApiServerCluster for ClusteringServer {
    /// Register a new node in the cluster
    async fn register_node(
        &self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        let req = request.into_inner();
        log::info!("Node registration request from: {}", req.hostname);
        
        match self.node_manager.register_node(req).await {
            Ok(response) => {
                log::info!("Node registration successful: {}", response.node_id);
                Ok(Response::new(response))
            },
            Err(e) => {
                log::error!("Node registration failed: {}", e);
                Err(Status::internal(format!("Registration failed: {}", e)))
            }
        }
    }

    /// Get all nodes with optional filtering
    async fn get_nodes(
        &self,
        request: Request<GetNodesRequest>,
    ) -> Result<Response<GetNodesResponse>, Status> {
        let req = request.into_inner();
        
        match self.node_manager.get_nodes(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                log::error!("Failed to get nodes: {}", e);
                Err(Status::internal(format!("Failed to get nodes: {}", e)))
            }
        }
    }

    /// Get specific node information
    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeResponse>, Status> {
        let req = request.into_inner();
        
        match self.node_manager.get_node(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                log::error!("Failed to get node: {}", e);
                Err(Status::internal(format!("Failed to get node: {}", e)))
            }
        }
    }

    /// Update node status
    async fn update_node_status(
        &self,
        request: Request<UpdateNodeStatusRequest>,
    ) -> Result<Response<UpdateNodeStatusResponse>, Status> {
        let req = request.into_inner();
        
        match self.node_manager.update_node_status(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                log::error!("Failed to update node status: {}", e);
                Err(Status::internal(format!("Failed to update node status: {}", e)))
            }
        }
    }

    /// Get cluster topology
    async fn get_topology(
        &self,
        request: Request<GetTopologyRequest>,
    ) -> Result<Response<GetTopologyResponse>, Status> {
        let req = request.into_inner();
        
        match self.node_manager.get_topology(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                log::error!("Failed to get topology: {}", e);
                Err(Status::internal(format!("Failed to get topology: {}", e)))
            }
        }
    }

    /// Update cluster topology (placeholder implementation)
    async fn update_topology(
        &self,
        _request: Request<UpdateTopologyRequest>,
    ) -> Result<Response<UpdateTopologyResponse>, Status> {
        // For now, return not implemented
        // In a full implementation, this would update the cluster topology
        Err(Status::unimplemented("Topology updates not yet implemented"))
    }

    /// Health check for the clustering service
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let req = request.into_inner();
        
        match self.node_manager.health_check(req).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => {
                log::error!("Health check failed: {}", e);
                Err(Status::internal(format!("Health check failed: {}", e)))
            }
        }
    }
}
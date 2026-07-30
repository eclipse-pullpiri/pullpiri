/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub mod actioncontroller;
pub mod apiserver;

use crate::desired_state::DesiredState;
use common::nodeagent::node_agent_connection_server::NodeAgentConnection;
use common::nodeagent::{
    fromactioncontroller::{HandleWorkloadRequest, HandleWorkloadResponse},
    fromapiserver::{
        ConfigRequest, ConfigResponse, HandleYamlRequest, HandleYamlResponse, HeartbeatRequest,
        HeartbeatResponse, NodeRegistrationRequest, NodeRegistrationResponse, StatusAck,
        StatusReport,
    },
    GetResourceStatusRequest, GetResourceStatusResponse, UpdateResourcesRequest,
    UpdateResourcesResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tonic::{Request, Response, Status};

/// NodeAgent gRPC service handler
#[derive(Clone)]
pub struct NodeAgentReceiver {
    pub tx: mpsc::Sender<HandleYamlRequest>,
    /// Node information for clustering
    pub node_id: String,
    pub hostname: String,
    pub ip_address: String,
    /// In-memory cache of desired states for self-healing
    pub desired_states_cache: Arc<Mutex<HashMap<String, DesiredState>>>,
}

impl NodeAgentReceiver {
    pub fn new(
        tx: mpsc::Sender<HandleYamlRequest>,
        node_id: String,
        hostname: String,
        ip_address: String,
        desired_states_cache: Arc<Mutex<HashMap<String, DesiredState>>>,
    ) -> Self {
        Self {
            tx,
            node_id,
            hostname,
            ip_address,
            desired_states_cache,
        }
    }
}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentReceiver {
    /// Handle a yaml request from API-Server
    ///
    /// Receives a yaml from API-Server and forwards it to the NodeAgent manager for processing.
    async fn handle_yaml(
        &self,
        request: Request<HandleYamlRequest>,
    ) -> Result<Response<HandleYamlResponse>, Status> {
        apiserver::handle_yaml(self.tx.clone(), request).await
    }

    /// Register this node with the API server
    async fn register_node(
        &self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        apiserver::register_node(request).await
    }

    /// Report status to the API server
    async fn report_status(
        &self,
        request: Request<StatusReport>,
    ) -> Result<Response<StatusAck>, Status> {
        apiserver::report_status(request).await
    }

    /// Process heartbeat from API server
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        apiserver::heartbeat(request).await
    }

    /// Receive configuration updates from API server
    async fn receive_config(
        &self,
        request: Request<ConfigRequest>,
    ) -> Result<Response<ConfigResponse>, Status> {
        apiserver::receive_config(request).await
    }

    /// Handle a workload request from ActionController
    ///
    /// Stores desired state in the in-memory cache on START and removes it on STOP/REMOVE,
    /// then delegates to the Podman runtime.
    async fn handle_workload(
        &self,
        request: Request<HandleWorkloadRequest>,
    ) -> Result<Response<HandleWorkloadResponse>, Status> {
        actioncontroller::handle_workload(request, Arc::clone(&self.desired_states_cache)).await
    }

    /// Update the CPU / memory limits of a running container at runtime.
    ///
    /// Dynamic Resource Scaling (#514 / #526): applies the change through the
    /// Podman REST API and returns the actual runtime state read back after the
    /// update for desired/actual synchronization.
    async fn update_resources(
        &self,
        request: Request<UpdateResourcesRequest>,
    ) -> Result<Response<UpdateResourcesResponse>, Status> {
        actioncontroller::update_resources(request).await
    }

    /// Query the actual runtime CPU / memory limits of a container.
    async fn get_resource_status(
        &self,
        request: Request<GetResourceStatusRequest>,
    ) -> Result<Response<GetResourceStatusResponse>, Status> {
        actioncontroller::get_resource_status(request).await
    }
}

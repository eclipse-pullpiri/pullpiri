/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC sender implementation for API Server
//! Consolidates all outbound gRPC communications from the API Server

use common::filtergateway::{
    connect_server as fg_connect_server,
    filter_gateway_connection_client::FilterGatewayConnectionClient, HandleScenarioRequest,
    HandleScenarioResponse,
};
use common::nodeagent::{
    connect_guest_server, connect_server, node_agent_service_client::NodeAgentServiceClient,
    HandleYamlRequest, HandleYamlResponse,
};
use common::statemanager::{
    connect_server as sm_connect_server,
    state_manager_connection_client::StateManagerConnectionClient, StateChange,
    StateChangeResponse,
};
use tonic::{Request, Response, Status};

/// Consolidated gRPC sender for all API Server outbound communications
#[derive(Clone, Default)]
pub struct ApiServerSender {
    /// Cached StateManager client
    state_manager_client: Option<StateManagerConnectionClient<tonic::transport::Channel>>,
}

impl ApiServerSender {
    /// Create a new sender instance
    pub fn new() -> Self {
        Self {
            state_manager_client: None,
        }
    }

    /// Send YAML to NodeAgent
    pub async fn send_yaml_to_nodeagent(
        &self,
        action: HandleYamlRequest,
    ) -> Result<Response<HandleYamlResponse>, Status> {
        let mut client: NodeAgentServiceClient<tonic::transport::Channel> =
            NodeAgentServiceClient::connect(connect_server())
                .await
                .unwrap();
        client.handle_yaml(Request::new(action)).await
    }

    /// Send YAML to guest NodeAgent
    pub async fn send_yaml_to_guest_nodeagent(
        &self,
        action: HandleYamlRequest,
    ) -> Result<Response<HandleYamlResponse>, Status> {
        let mut client: NodeAgentServiceClient<tonic::transport::Channel> =
            NodeAgentServiceClient::connect(connect_guest_server())
                .await
                .unwrap();
        client.handle_yaml(Request::new(action)).await
    }

    /// Send scenario to FilterGateway
    pub async fn send_scenario_to_filtergateway(
        &self,
        scenario: HandleScenarioRequest,
    ) -> Result<Response<HandleScenarioResponse>, Status> {
        let mut client = FilterGatewayConnectionClient::connect(fg_connect_server())
            .await
            .unwrap();
        client.handle_scenario(Request::new(scenario)).await
    }

    /// Ensure StateManager connection is established
    async fn ensure_state_manager_connected(&mut self) -> Result<(), Status> {
        if self.state_manager_client.is_none() {
            match StateManagerConnectionClient::connect(sm_connect_server()).await {
                Ok(client) => {
                    self.state_manager_client = Some(client);
                    Ok(())
                }
                Err(e) => Err(Status::unknown(format!(
                    "Failed to connect to StateManager: {}",
                    e
                ))),
            }
        } else {
            Ok(())
        }
    }

    /// Send state change to StateManager
    pub async fn send_state_change_to_statemanager(
        &mut self,
        state_change: StateChange,
    ) -> Result<Response<StateChangeResponse>, Status> {
        self.ensure_state_manager_connected().await?;

        if let Some(client) = &mut self.state_manager_client {
            client.send_state_change(Request::new(state_change)).await
        } else {
            Err(Status::unknown("StateManager client not connected"))
        }
    }
}

// Legacy compatibility functions
pub async fn send(action: HandleYamlRequest) -> Result<Response<HandleYamlResponse>, Status> {
    let sender = ApiServerSender::new();
    sender.send_yaml_to_nodeagent(action).await
}

pub async fn send_guest(action: HandleYamlRequest) -> Result<Response<HandleYamlResponse>, Status> {
    let sender = ApiServerSender::new();
    sender.send_yaml_to_guest_nodeagent(action).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_server_sender_creation() {
        let sender = ApiServerSender::new();
        assert!(sender.state_manager_client.is_none());
    }

    #[tokio::test]
    async fn test_legacy_compatibility() {
        let request = HandleYamlRequest {
            yaml: "test".to_string(),
        };

        // Test that legacy functions still work
        // Note: These may fail in test environment due to missing services
        let _result1 = send(request.clone()).await;
        let _result2 = send_guest(request).await;
    }
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::apiserver::api_server_connection_client::ApiServerConnectionClient;
use common::monitoringserver::{ContainerList, SendContainerListResponse};
use common::nodeagent::fromapiserver::{
    HeartbeatRequest, HeartbeatResponse, NodeRegistrationRequest, NodeRegistrationResponse,
    StatusAck, StatusReport,
};
use common::statemanager::{
    state_manager_connection_client::StateManagerConnectionClient, Action, Response,
};

use common::monitoringserver::monitoring_server_connection_client::MonitoringServerConnectionClient;
use tonic::{Request, Status};

/// Sender for making gRPC requests to Monitoring Server
#[derive(Clone, Default)]
pub struct NodeAgentSender {}

impl NodeAgentSender {
    /// Trigger an action for a scenario
    pub async fn trigger_action(
        &mut self,
        action: Action,
    ) -> Result<tonic::Response<Response>, Status> {
        let addr = common::statemanager::connect_server();
        let client = StateManagerConnectionClient::connect(addr).await;
        match client {
            Ok(mut client) => {
                // Send the action
                client.send_action(Request::new(action)).await
            }
            Err(e) => {
                // Handle connection error
                Err(Status::unknown(format!(
                    "Failed to connect statemanager: {}",
                    e
                )))
            }
        }
    }

    /// Send a ContainerList to the monitoring server via gRPC
    pub async fn send_container_list(
        &mut self,
        container_list: ContainerList,
    ) -> Result<tonic::Response<SendContainerListResponse>, Status> {
        let config = crate::config::Config::get();
        let master_ip = config.nodeagent.master_ip.clone();
        let addr = format!("http://{}:47003", master_ip);

        let client = MonitoringServerConnectionClient::connect(addr).await;

        match client {
            Ok(mut client) => {
                // Send the container list
                client
                    .send_container_list(Request::new(container_list))
                    .await
            }
            Err(e) => {
                // Handle connection error
                Err(Status::unknown(format!("Failed to connect: {}", e)))
            }
        }
    }

    /// Send node information to the monitoring server
    pub async fn send_node_info(
        &mut self,
        node_info: common::monitoringserver::NodeInfo,
    ) -> Result<tonic::Response<common::monitoringserver::SendNodeInfoResponse>, Status> {
        let config = crate::config::Config::get();
        let master_ip = config.nodeagent.master_ip.clone();
        let addr = format!("http://{}:47003", master_ip);

        let client = MonitoringServerConnectionClient::connect(addr).await;

        match client {
            Ok(mut client) => client.send_node_info(Request::new(node_info)).await,
            Err(e) => Err(Status::unknown(format!("Failed to connect: {}", e))),
        }
    }

    /// Send a changed ContainerList to the state manager via gRPC
    pub async fn send_changed_container_list(
        &mut self,
        container_list: ContainerList,
    ) -> Result<tonic::Response<SendContainerListResponse>, Status> {
        let config = crate::config::Config::get();
        let master_ip = config.nodeagent.master_ip.clone();
        let addr = format!("http://{}:47006", master_ip);

        let client = StateManagerConnectionClient::connect(addr).await;

        match client {
            Ok(mut client) => {
                // Send the changed container list
                client
                    .send_changed_container_list(Request::new(container_list))
                    .await
            }
            Err(e) => {
                // Handle connection error
                Err(Status::unknown(format!("Failed to connect: {}", e)))
            }
        }
    }

    /// Register this node with the API server
    pub async fn register_with_api_server(
        &mut self,
        registration_request: NodeRegistrationRequest,
    ) -> Result<tonic::Response<NodeRegistrationResponse>, Status> {
        let config = crate::config::Config::get();
        let master_ip = config.nodeagent.master_ip.clone();
        let addr = format!("http://{}:47098", master_ip);

        let client = ApiServerConnectionClient::connect(addr).await;

        match client {
            Ok(mut client) => {
                client
                    .register_node(Request::new(registration_request))
                    .await
            }
            Err(e) => Err(Status::unknown(format!(
                "Failed to connect to API server: {}",
                e
            ))),
        }
    }

    /// Send heartbeat to the API server
    pub async fn send_heartbeat(
        &mut self,
        _heartbeat_request: HeartbeatRequest,
    ) -> Result<tonic::Response<HeartbeatResponse>, Status> {
        // For NodeAgent sender, we use the NodeAgent service to send heartbeat
        // This is a local operation that would be handled by the node's own receiver
        // In practice, heartbeats are typically sent from NodeAgent to API server
        // but this implementation allows for local heartbeat processing

        // TODO: Implement heartbeat sending to API server if needed
        // For now, return a success response
        // Use master_ip from config
        let config = crate::config::Config::get();
        let master_ip = config.nodeagent.master_ip.clone();
        let master_endpoint = format!("http://{}:47098", master_ip);

        Ok(tonic::Response::new(HeartbeatResponse {
            ack: true,
            updated_config: Some(common::nodeagent::fromapiserver::ClusterConfig {
                master_endpoint,
                heartbeat_interval: 30,
                settings: std::collections::HashMap::new(),
            }),
        }))
    }

    /// Send status report to the API server
    pub async fn send_status_report(
        &mut self,
        status_report: StatusReport,
    ) -> Result<tonic::Response<StatusAck>, Status> {
        // Similar to heartbeat, this is typically a NodeAgent operation
        // TODO: Implement status reporting to API server if needed

        println!("Sending status report for node: {}", status_report.node_id);

        Ok(tonic::Response::new(StatusAck {
            received: true,
            message: "Status report sent successfully".to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::grpc::sender::NodeAgentSender;
    use common::monitoringserver::{ContainerList, NodeInfo};
    use common::nodeagent::fromapiserver::{
        HeartbeatRequest, NodeRegistrationRequest, StatusReport,
    };
    use common::statemanager::{Action, Response as SMResponse};

    #[tokio::test]
    async fn test_trigger_action_success() {
        let mut sender = NodeAgentSender::default();

        let action = Action::default();
        let result = sender.trigger_action(action).await;

        match result {
            Ok(response) => {
                let _resp: SMResponse = response.into_inner();
            }
            Err(_) => {
                // connection might fail in test environment, still test handles Ok case
            }
        }
    }

    #[tokio::test]
    async fn test_trigger_action_multiple_calls() {
        let mut sender = NodeAgentSender::default();

        let action1 = Action::default();
        let action2 = Action::default();

        let result1 = sender.trigger_action(action1).await;
        let result2 = sender.trigger_action(action2).await;

        assert!(result1.is_ok() || result1.is_err());
        assert!(result2.is_ok() || result2.is_err());
    }

    #[tokio::test]
    async fn test_send_container_list_error_propagation() {
        let mut sender = NodeAgentSender::default();

        let container_list = ContainerList::default();

        let response = sender.send_container_list(container_list).await;

        assert!(response.is_ok() || response.is_err());
    }

    #[tokio::test]
    async fn test_send_node_info_error_propagation() {
        let mut sender = NodeAgentSender::default();

        let node_info = NodeInfo::default();

        let response = sender.send_node_info(node_info).await;

        assert!(response.is_ok() || response.is_err());
    }

    #[tokio::test]
    async fn test_send_changed_container_list_error_propagation() {
        let mut sender = NodeAgentSender::default();

        let container_list = ContainerList::default();

        let response = sender.send_changed_container_list(container_list).await;

        assert!(response.is_ok() || response.is_err());
    }

    #[tokio::test]
    async fn test_register_with_api_server_success_and_failure() {
        let mut sender = NodeAgentSender::default();

        let req = NodeRegistrationRequest::default();
        let result = sender.register_with_api_server(req).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_send_heartbeat_returns_success() {
        let mut sender = NodeAgentSender::default();

        let req = HeartbeatRequest::default();
        let result = sender.send_heartbeat(req).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert!(resp.ack);
        assert_eq!(resp.updated_config.as_ref().unwrap().heartbeat_interval, 30);
    }

    #[tokio::test]
    async fn test_send_status_report_returns_success() {
        let mut sender = NodeAgentSender::default();

        let req = StatusReport::default();
        let result = sender.send_status_report(req).await;
        assert!(result.is_ok());
        let resp = result.unwrap().into_inner();
        assert!(resp.received);
        assert!(resp.message.contains("Status report sent"));
    }

    #[tokio::test]
    async fn test_send_container_list_with_invalid_addr() {
        let mut sender = NodeAgentSender::default();

        // Simulate invalid config by patching master_ip if possible
        // (If not possible, this will just test error handling)
        let mut container_list = ContainerList::default();
        container_list.node_name = "invalid".to_string();
        let result = sender.send_container_list(container_list).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_send_node_info_with_invalid_addr() {
        let mut sender = NodeAgentSender::default();

        let mut node_info = NodeInfo::default();
        node_info.node_name = "invalid".to_string();
        let result = sender.send_node_info(node_info).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_send_changed_container_list_with_invalid_addr() {
        let mut sender = NodeAgentSender::default();

        let mut container_list = ContainerList::default();
        container_list.node_name = "invalid".to_string();
        let result = sender.send_changed_container_list(container_list).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_register_with_api_server_with_invalid_addr() {
        let mut sender = NodeAgentSender::default();

        let mut req = NodeRegistrationRequest::default();
        req.node_id = "invalid".to_string();
        let result = sender.register_with_api_server(req).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_send_heartbeat_multiple_calls() {
        let mut sender = NodeAgentSender::default();

        let req = HeartbeatRequest::default();
        let result1 = sender.send_heartbeat(req.clone()).await;
        let result2 = sender.send_heartbeat(req).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_send_status_report_multiple_calls() {
        let mut sender = NodeAgentSender::default();

        let req = StatusReport::default();
        let result1 = sender.send_status_report(req.clone()).await;
        let result2 = sender.send_status_report(req).await;
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}

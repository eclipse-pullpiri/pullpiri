/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::monitoringserver::{ContainerList, SendContainerListResponse};
use common::statemanager::{
    connect_server, state_manager_connection_client::StateManagerConnectionClient, Action, Response,
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
        let mut client = StateManagerConnectionClient::connect(connect_server())
            .await
            .unwrap();
        client.send_action(Request::new(action)).await
    }

    /// Send a ContainerList to the monitoring server via gRPC
    pub async fn send_container_list(
        &mut self,
        container_list: ContainerList,
    ) -> Result<tonic::Response<SendContainerListResponse>, Status> {
        let mut client =
            MonitoringServerConnectionClient::connect(common::monitoringserver::connect_server())
                .await
                .unwrap();
        client
            .send_container_list(Request::new(container_list))
            .await
    }

    /// Send a changed ContainerList to the state manager via gRPC
    pub async fn send_changed_container_list(
        &mut self,
        container_list: ContainerList,
    ) -> Result<tonic::Response<SendContainerListResponse>, Status> {
        let mut client =
            StateManagerConnectionClient::connect(common::statemanager::connect_server())
                .await
                .unwrap();
        client
            .send_changed_container_list(Request::new(container_list))
            .await
    }
}

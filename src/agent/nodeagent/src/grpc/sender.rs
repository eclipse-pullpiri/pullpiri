/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::statemanager::{
    connect_server, state_manager_connection_client::StateManagerConnectionClient, Action, Response,
};
use tonic::{Request, Status};

/// Sender for making gRPC requests to Monitoring Server
#[derive(Clone)]
pub struct NodeAgentSender {}

impl NodeAgentSender {
    /// Create a new NodeAgentSender
    pub fn new() -> Self {
        Self {}
    }

    /// Trigger an action for a scenario
    pub async fn trigger_action(&mut self, action: Action) -> Result<tonic::Response<Response>, Status> {
        let mut client = StateManagerConnectionClient::connect(connect_server())
            .await
            .unwrap();
        client.send_action(Request::new(action)).await
    }
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager gRPC client for sending state change messages from PolicyManager.

use common::statemanager::{
    connect_server, state_manager_connection_client::StateManagerConnectionClient, ResourceType,
    StateChange, StateChangeResponse,
};
use tonic::{Request, Status};

/// StateManager gRPC client for PolicyManager component.
#[derive(Clone)]
pub struct StateManagerSender {
    /// Cached gRPC client connection to the StateManager service.
    client: Option<StateManagerConnectionClient<tonic::transport::Channel>>,
}

impl Default for StateManagerSender {
    fn default() -> Self {
        Self::new()
    }
}

impl StateManagerSender {
    /// Creates a new StateManagerSender instance.
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Ensures a gRPC connection to the StateManager exists and is ready for use.
    async fn ensure_connected(&mut self) -> Result<(), Status> {
        if self.client.is_none() {
            match StateManagerConnectionClient::connect(connect_server()).await {
                Ok(client) => {
                    self.client = Some(client);
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

    /// Sends a state change message to the StateManager service.
    pub async fn send_state_change(
        &mut self,
        state_change: StateChange,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            client.send_state_change(Request::new(state_change)).await
        } else {
            Err(Status::unknown("Client not connected"))
        }
    }
}

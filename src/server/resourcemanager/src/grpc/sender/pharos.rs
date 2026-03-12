/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Pharos gRPC client for sending network resource messages from ResourceManager.

use common::external::pharos::{
    pharos_network_manager_client::PharosNetworkManagerClient, NetworkRemoveRequest,
    NetworkRemoveResponse, NetworkSetupRequest, NetworkSetupResponse,
};
use tonic::{Request, Status};

/// Pharos gRPC client for ResourceManager component.
#[derive(Clone)]
pub struct PharosSender {
    /// Cached gRPC client connection to the Pharos service.
    client: Option<PharosNetworkManagerClient<tonic::transport::Channel>>,
}

impl Default for PharosSender {
    fn default() -> Self {
        Self::new()
    }
}

impl PharosSender {
    /// Creates a new PharosSender instance.
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Ensures a gRPC connection to the Pharos exists and is ready for use.
    async fn ensure_connected(&mut self) -> Result<(), Status> {
        if self.client.is_none() {
            match PharosNetworkManagerClient::connect(
                common::external::pharos::connect_pharos_server(),
            )
            .await
            {
                Ok(client) => {
                    self.client = Some(client);
                    Ok(())
                }
                Err(e) => Err(Status::unknown(format!(
                    "Failed to connect to Pharos: {}",
                    e
                ))),
            }
        } else {
            Ok(())
        }
    }

    /// Sends a network setup request to the Pharos service.
    pub async fn setup_network(
        &mut self,
        req: NetworkSetupRequest,
    ) -> Result<tonic::Response<NetworkSetupResponse>, Status> {
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            client.setup_network(Request::new(req)).await
        } else {
            Err(Status::unknown("Client not connected"))
        }
    }

    /// Sends a network remove request to the Pharos service.
    pub async fn remove_network(
        &mut self,
        req: NetworkRemoveRequest,
    ) -> Result<tonic::Response<NetworkRemoveResponse>, Status> {
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            client.remove_network(Request::new(req)).await
        } else {
            Err(Status::unknown("Client not connected"))
        }
    }
}

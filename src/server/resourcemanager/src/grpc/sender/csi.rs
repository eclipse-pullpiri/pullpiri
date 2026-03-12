/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! CSI gRPC client for sending volume resource messages from ResourceManager.

use common::external::csi::{
    csi_volume_manager_client::CsiVolumeManagerClient, VolumeCreateRequest, VolumeCreateResponse,
    VolumeDeleteRequest, VolumeDeleteResponse,
};
use tonic::{Request, Status};

/// CSI gRPC client for ResourceManager component.
#[derive(Clone)]
pub struct CsiSender {
    /// Cached gRPC client connection to the CSI service.
    client: Option<CsiVolumeManagerClient<tonic::transport::Channel>>,
}

impl Default for CsiSender {
    fn default() -> Self {
        Self::new()
    }
}

impl CsiSender {
    /// Creates a new CsiSender instance.
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Ensures a gRPC connection to the CSI exists and is ready for use.
    async fn ensure_connected(&mut self) -> Result<(), Status> {
        if self.client.is_none() {
            match CsiVolumeManagerClient::connect(
                common::external::csi::connect_csi_server(),
            )
            .await
            {
                Ok(client) => {
                    self.client = Some(client);
                    Ok(())
                }
                Err(e) => Err(Status::unknown(format!(
                    "Failed to connect to CSI: {}",
                    e
                ))),
            }
        } else {
            Ok(())
        }
    }

    /// Sends a volume create request to the CSI service.
    pub async fn create_volume(
        &mut self,
        req: VolumeCreateRequest,
    ) -> Result<tonic::Response<VolumeCreateResponse>, Status> {
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            client.create_volume(Request::new(req)).await
        } else {
            Err(Status::unknown("Client not connected"))
        }
    }

    /// Sends a volume delete request to the CSI service.
    pub async fn delete_volume(
        &mut self,
        req: VolumeDeleteRequest,
    ) -> Result<tonic::Response<VolumeDeleteResponse>, Status> {
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            client.delete_volume(Request::new(req)).await
        } else {
            Err(Status::unknown("Client not connected"))
        }
    }
}

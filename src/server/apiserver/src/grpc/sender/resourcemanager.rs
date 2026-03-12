/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! ResourceManager gRPC client for sending resource YAML from ApiServer.
//!
//! This module provides a client interface for the ApiServer to communicate with
//! the ResourceManager service via gRPC. It sends raw YAML artifacts (Network/Volume)
//! to ResourceManager for parsing and processing.

use common::resourcemanager::{
    connect_server, resource_manager_service_client::ResourceManagerServiceClient,
    Action, HandleResourceRequest, HandleResourceResponse,
};
use tonic::{Request, Response, Status};

/// ResourceManager gRPC client for ApiServer component.
///
/// This client manages the gRPC connection to the ResourceManager service and provides
/// methods for sending raw YAML artifacts for processing.
#[derive(Clone)]
pub struct ResourceManagerSender {
    /// Cached gRPC client connection to the ResourceManager service.
    client: Option<ResourceManagerServiceClient<tonic::transport::Channel>>,
}

impl Default for ResourceManagerSender {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceManagerSender {
    /// Creates a new ResourceManagerSender instance.
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Ensures a gRPC connection to the ResourceManager exists and is ready for use.
    async fn ensure_connected(&mut self) -> Result<(), Status> {
        if self.client.is_none() {
            match ResourceManagerServiceClient::connect(connect_server()).await {
                Ok(client) => {
                    self.client = Some(client);
                    Ok(())
                }
                Err(e) => Err(Status::unknown(format!(
                    "Failed to connect to ResourceManager: {}",
                    e
                ))),
            }
        } else {
            Ok(())
        }
    }

    /// Sends a resource YAML to ResourceManager for processing.
    ///
    /// # Arguments
    /// * `resource_yaml` - Raw YAML string (Network or Volume artifact)
    /// * `action` - Action type (APPLY or WITHDRAW)
    ///
    /// # Returns
    /// * `Result<Response<HandleResourceResponse>, Status>` - Response from ResourceManager
    pub async fn send(
        &mut self,
        resource_yaml: String,
        action: Action,
    ) -> Result<Response<HandleResourceResponse>, Status> {
        self.ensure_connected().await?;

        let request = HandleResourceRequest {
            resource_yaml,
            action: action as i32,
        };

        if let Some(client) = &mut self.client {
            client.handle_resource(Request::new(request)).await
        } else {
            Err(Status::unknown("ResourceManager client not connected"))
        }
    }
}

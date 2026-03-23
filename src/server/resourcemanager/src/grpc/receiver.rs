/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC Receiver module
//!
//! This module is responsible for:
//! - Receiving gRPC requests
//! - Delegating message processing to ResourceManager

use std::sync::Arc;

use crate::manager::ResourceManager;
use common::logd;
use common::resourcemanager::resource_manager_service_server::{
    ResourceManagerService, ResourceManagerServiceServer,
};
use common::resourcemanager::{Action, HandleResourceRequest, HandleResourceResponse};
use tokio::sync::RwLock;
use tonic::Response;

/// Receiver for handling incoming gRPC requests for ResourceManager
///
/// Implements the ResourceManagerService gRPC service defined in
/// the protobuf specification.
pub struct ResourceManagerReceiver {
    /// Reference to the ResourceManager
    manager: Arc<RwLock<ResourceManager>>,
}

impl ResourceManagerReceiver {
    /// Create a new ResourceManagerReceiver instance
    ///
    /// # Arguments
    ///
    /// * `manager` - Shared reference to the ResourceManager
    ///
    /// # Returns
    ///
    /// A new ResourceManagerReceiver instance
    pub fn new(manager: Arc<RwLock<ResourceManager>>) -> Self {
        Self { manager }
    }

    /// Get a gRPC server for this receiver
    ///
    /// # Returns
    ///
    /// A configured ResourceManagerServiceServer
    pub fn into_service(self) -> ResourceManagerServiceServer<Self> {
        ResourceManagerServiceServer::new(self)
    }
}

#[tonic::async_trait]
impl ResourceManagerService for ResourceManagerReceiver {
    async fn handle_resource(
        &self,
        request: tonic::Request<HandleResourceRequest>,
    ) -> Result<tonic::Response<HandleResourceResponse>, tonic::Status> {
        let req = request.into_inner();
        let yaml_str = &req.resource_yaml;
        let action = Action::try_from(req.action).unwrap_or(Action::Apply);

        logd!(1, "GRPC RECEIVER: Received resource request");
        logd!(2, "  Action: {:?}", action);

        // Delegate YAML parsing, resource kind determination, and routing to ResourceManager
        let mut manager = self.manager.write().await;
        let response = manager.handle_resource(yaml_str, action).await;

        Ok(Response::new(response))
    }
}

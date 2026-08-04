/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! ResourceManager service.
//!
//! Owns the desired resource state for scaled workloads, validates whether a
//! requested CPU / memory update fits into the available node capacity, and
//! reconciles the desired state against the actual runtime state reported by
//! the NodeAgent (via the ActionController).
//!
//! Part of the Dynamic Resource Scaling feature (#514 / #526).

pub mod grpc;
pub mod manager;

use common::resourcemanager::resource_manager_connection_server::ResourceManagerConnectionServer;
use grpc::receiver::ResourceManagerReceiver;
use std::sync::Arc;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ResourceManager starting...");

    let addr = common::resourcemanager::open_server().parse()?;
    let manager = Arc::new(manager::ResourceManagerManager::new());
    let receiver = ResourceManagerReceiver::new(manager);

    println!("ResourceManager gRPC server listening on {}", addr);

    Server::builder()
        .add_service(ResourceManagerConnectionServer::new(receiver))
        .serve(addr)
        .await?;

    Ok(())
}

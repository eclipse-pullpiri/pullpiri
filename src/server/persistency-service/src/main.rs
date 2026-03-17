/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Persistency Service Main
//!
//! A standalone gRPC service that provides centralized persistency for all Pullpiri components.

use common::persistency_proto::persistency_service_server::PersistencyServiceServer;
use persistency_service::PersistencyServiceImpl;
use tonic::transport::Server;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Pullpiri Persistency Service");

    // Create the persistency service
    let service = match PersistencyServiceImpl::new() {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to initialize persistency service: {:?}", e);
            std::process::exit(1);
        }
    };

    // Get the server address
    let addr = common::persistency::open_server().parse()?;
    info!("Persistency service listening on {}", addr);

    // Start the gRPC server
    Server::builder()
        .add_service(PersistencyServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
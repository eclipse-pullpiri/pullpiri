/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
pub mod grpc;
pub mod manager;

use std::sync::Arc;

use common::logd;
use common::logd::logger;
use grpc::receiver::ResourceManagerReceiver;
use manager::ResourceManager;
use tokio::sync::RwLock;
use tonic::transport::Server;

#[tokio::main]
async fn main() {
    // Initialize async logger for logd! macro
    let _ = logger::init_async_logger("resourcemanager").await;

    logd!(1, "Starting Resource Manager...");

    // Create ResourceManager and wrap in Arc<RwLock> for shared mutable ownership
    let manager = Arc::new(RwLock::new(ResourceManager::new()));

    // Create receiver with injected manager
    let receiver = ResourceManagerReceiver::new(manager);

    let addr = common::resourcemanager::open_server()
        .parse()
        .expect("resourcemanager address parsing error");
    logd!(1, "ResourceManager gRPC server listening on {}", addr);

    if let Err(e) = Server::builder()
        .add_service(receiver.into_service())
        .serve(addr)
        .await
    {
        logd!(5, "gRPC server error: {}", e);
    }
}

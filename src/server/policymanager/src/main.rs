/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
pub mod grpc;

use common::policymanager::policy_manager_connection_server::PolicyManagerConnectionServer;
use grpc::receiver::PolicyManagerGrpcServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 PolicyManager starting...");

    let addr = common::policymanager::open_server().parse()?;
    let server = PolicyManagerGrpcServer::new();

    println!("📡 PolicyManager gRPC server listening on {}", addr);

    Server::builder()
        .add_service(PolicyManagerConnectionServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
pub mod grpc;

use common::resourcemanager::resource_manager_service_server::ResourceManagerServiceServer;
use grpc::receiver::ResourceManagerGrpcServer;
use tonic::transport::Server;

#[tokio::main]
async fn main() {
    println!("Starting Resource Manager...");

    let server = ResourceManagerGrpcServer::new();

    let addr = common::resourcemanager::open_server()
        .parse()
        .expect("resourcemanager address parsing error");
    println!("ResourceManager gRPC server listening on {}", addr);

    if let Err(e) = Server::builder()
        .add_service(ResourceManagerServiceServer::new(server))
        .serve(addr)
        .await
    {
        println!("gRPC server error: {}", e);
    }
}

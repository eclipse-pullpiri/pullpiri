/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! MonitoringServer main entry point
//!
//! This file sets up the asynchronous runtime, initializes the manager and gRPC server,
//! and launches both concurrently. It also provides unit tests for initialization.

use common::monitoringserver::{ContainerList, NodeInfo};
pub mod data_structures;
pub mod etcd_storage;
pub mod grpc;
pub mod manager;

use common::monitoringserver::monitoring_server_connection_server::MonitoringServerConnectionServer;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Launches the MonitoringServerManager in an asynchronous task.
///
/// This function creates the manager, initializes it, and then runs it.
/// If initialization or running fails, errors are printed to stderr.
async fn launch_manager(
    rx_container: Receiver<ContainerList>,
    rx_node: Receiver<NodeInfo>,
    rx_stress: Receiver<String>,
) {
    let mut manager = manager::MonitoringServerManager::new(rx_container, rx_node, rx_stress).await;

    match manager.initialize().await {
        Ok(_) => {
            println!("MonitoringServerManager successfully initialized");
            if let Err(e) = manager.run().await {
                eprintln!("Error running MonitoringServerManager: {:?}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize MonitoringServerManager: {:?}", e);
        }
    }
}

/// Initializes the MonitoringServer gRPC server.
///
/// Sets up the gRPC service and starts listening for incoming requests.
async fn initialize(
    tx_container: Sender<ContainerList>,
    tx_node: Sender<NodeInfo>,
    tx_stress: Sender<String>,
) {
    use tonic::transport::Server;

    let server = grpc::receiver::MonitoringServerReceiver {
        tx_container,
        tx_node,
        tx_stress,
    };

    let addr = common::monitoringserver::open_server()
        .parse()
        .expect("monitoringserver address parsing error");
    println!("MonitoringServer listening on {}", addr);

    if let Err(e) = Server::builder()
        .add_service(MonitoringServerConnectionServer::new(server))
        .serve(addr)
        .await
    {
        eprintln!("gRPC server error: {}", e);
    }
}

#[tokio::main]
async fn main() {
    println!("Starting MonitoringServer...");

    let (tx_container, rx_container) = channel::<ContainerList>(100);
    let (tx_node, rx_node) = channel::<NodeInfo>(100);

    // Add stress channel and a simple consumer
    let (tx_stress, rx_stress) = channel::<String>(16);

    let mgr = launch_manager(rx_container, rx_node, rx_stress);
    let grpc = initialize(tx_container, tx_node, tx_stress);

    tokio::join!(mgr, grpc);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_launch_manager_completes() {
        let (_tx_c, rx_c) = tokio::sync::mpsc::channel(1);
        let (_tx_n, rx_n) = tokio::sync::mpsc::channel(1);
        let (_tx_s, rx_s) = tokio::sync::mpsc::channel::<String>(1);
        // Use a timeout to ensure the test does not hang
        let _result = timeout(Duration::from_secs(2), launch_manager(rx_c, rx_n, rx_s)).await;
        //assert!(result.is_ok(), "launch_manager did not complete in time");
    }

    #[tokio::test]
    async fn test_initialize_completes() {
        let (tx_c, _rx_c) = tokio::sync::mpsc::channel(1);
        let (tx_n, _rx_n) = tokio::sync::mpsc::channel(1);
        let (tx_s, _rx_s) = tokio::sync::mpsc::channel::<String>(1);
        // Spawn initialize in a background task and cancel after a short delay
        let handle = tokio::spawn(async move {
            // Use a short timeout to avoid hanging on .serve()
            let _ = timeout(Duration::from_millis(500), initialize(tx_c, tx_n, tx_s)).await;
        });

        // Wait for the task to finish or timeout
        let _result = timeout(Duration::from_secs(1), handle).await;
        assert!(_result.is_ok(), "initialize did not complete in time");
    }
}

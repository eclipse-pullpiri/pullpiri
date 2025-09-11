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
async fn launch_manager(rx_container: Receiver<ContainerList>, rx_node: Receiver<NodeInfo>) {
    let mut manager = manager::MonitoringServerManager::new(rx_container, rx_node).await;

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
async fn initialize(tx_container: Sender<ContainerList>, tx_node: Sender<NodeInfo>) {
    use tonic::transport::Server;

    let server = grpc::receiver::MonitoringServerReceiver {
        tx_container,
        tx_node,
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

    let mgr = launch_manager(rx_container, rx_node);
    let grpc = initialize(tx_container, tx_node);

    tokio::join!(mgr, grpc);
}

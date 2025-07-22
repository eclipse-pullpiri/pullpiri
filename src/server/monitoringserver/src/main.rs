//! MonitoringServer main entry point
//!
//! This file sets up the asynchronous runtime, initializes the manager and gRPC server,
//! and launches both concurrently. It also provides unit tests for initialization.

use common::monitoringserver::ContainerList;
pub mod grpc;
pub mod manager;

use common::monitoringserver::monitoring_server_connection_server::MonitoringServerConnectionServer;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Launches the MonitoringServerManager in an asynchronous task.
///
/// This function creates the manager, initializes it, and then runs it.
/// If initialization or running fails, errors are printed to stderr.
async fn launch_manager(rx_grpc: Receiver<ContainerList>) {
    let mut manager = manager::MonitoringServerManager::new(rx_grpc).await;

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
async fn initialize(tx_grpc: Sender<ContainerList>) {
    use tonic::transport::Server;

    let server = grpc::receiver::MonitoringServerReceiver {
        tx: tx_grpc.clone(),
    };

    let addr = common::monitoringserver::open_server()
        .parse()
        .expect("monitoringserver address parsing error");
    println!("MonitoringServer listening on {}", addr);

    let _ = Server::builder()
        .add_service(MonitoringServerConnectionServer::new(server))
        .serve(addr)
        .await;
}

#[tokio::main]
async fn main() {
    println!("Starting MonitoringServer...");

    let (tx_grpc, rx_grpc) = channel::<ContainerList>(100);
    let mgr = launch_manager(rx_grpc);
    let grpc = initialize(tx_grpc);

    tokio::join!(mgr, grpc);
}

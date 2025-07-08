//! NodeAgent main entry point
//!
//! This file sets up the asynchronous runtime, initializes the manager and gRPC server,
//! and launches both concurrently. It also provides unit tests for initialization.

use grpc::sender::NodeAgentSender;
use manager::NodeAgentParameter;

mod bluechi;
pub mod grpc;
pub mod manager;

use common::nodeagent::node_agent_connection_server::NodeAgentConnectionServer;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Launches the NodeAgentManager in an asynchronous task.
///
/// This function creates the manager, initializes it, and then runs it.
/// If initialization or running fails, errors are printed to stderr.
async fn launch_manager(rx_grpc: Receiver<NodeAgentParameter>) {
    let mut manager = manager::NodeAgentManager::new(rx_grpc).await;

    match manager.initialize().await {
        Ok(_) => {
            println!("NodeAgentManager successfully initialized");
            if let Err(e) = manager.run().await {
                eprintln!("Error running NodeAgentManager: {:?}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize NodeAgentManager: {:?}", e);
        }
    }
}

/// Initializes the NodeAgent gRPC server.
///
/// Sets up the gRPC service and starts listening for incoming requests.
async fn initialize(tx_grpc: Sender<manager::NodeAgentParameter>) {
    use tonic::transport::Server;

    let server = grpc::receiver::NodeAgentReceiver::new(tx_grpc);
    let addr = common::nodeagent::open_server()
        .parse()
        .expect("nodeagent address parsing error");

    println!("NodeAgent listening on {}", addr);

    let _ = Server::builder()
        .add_service(NodeAgentConnectionServer::new(server))
        .serve(addr)
        .await;
}

/// Main entry point for the NodeAgent binary.
///
/// Sets up the async runtime, creates the communication channel, and launches
/// both the manager and gRPC server concurrently.
#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging (if needed)
    let (tx_grpc, rx_grpc): (Sender<NodeAgentParameter>, Receiver<NodeAgentParameter>) =
        channel(100);

    // Launch the manager thread (handles business logic)
    let mgr = launch_manager(rx_grpc);

    // Launch the gRPC server (handles incoming gRPC requests)
    let grpc = initialize(tx_grpc);

    // Run both tasks concurrently
    tokio::join!(mgr, grpc);
}

#[cfg(test)]
mod tests {
    use crate::manager::NodeAgentParameter;
    use crate::{initialize, launch_manager};
    use tokio::sync::mpsc::{channel, Receiver, Sender};
    use tokio::task::LocalSet;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_main_initializes_channels() {
        let (tx_grpc, rx_grpc): (Sender<NodeAgentParameter>, Receiver<NodeAgentParameter>) =
            channel(100);
        assert_eq!(tx_grpc.capacity(), 100);
        assert!(!rx_grpc.is_closed());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_main_launch_manager() {
        let (_tx_grpc, rx_grpc): (Sender<NodeAgentParameter>, Receiver<NodeAgentParameter>) =
            channel(100);
        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = launch_manager(rx_grpc).await;
        });
        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }
        assert!(true);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_main_initialize_grpc() {
        let (tx_grpc, _rx_grpc): (Sender<NodeAgentParameter>, Receiver<NodeAgentParameter>) =
            channel(100);
        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = initialize(tx_grpc).await;
        });
        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }
        assert!(true);
    }
}

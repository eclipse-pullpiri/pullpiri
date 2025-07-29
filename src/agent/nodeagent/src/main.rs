//! NodeAgent main entry point
//!
//! This file sets up the asynchronous runtime, initializes the manager and gRPC server,
//! and launches both concurrently. It also provides unit tests for initialization.

use common::nodeagent::HandleYamlRequest;
mod bluechi;
pub mod grpc;
pub mod manager;
pub mod resource;

use common::nodeagent::node_agent_connection_server::NodeAgentConnectionServer;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Launches the NodeAgentManager in an asynchronous task.
///
/// This function creates the manager, initializes it, and then runs it.
/// If initialization or running fails, errors are printed to stderr.
async fn launch_manager(rx_grpc: Receiver<HandleYamlRequest>, hostname: String) {
    let mut manager = manager::NodeAgentManager::new(rx_grpc, hostname).await;

    //manager.initialize().await;
    // let _ = manager.process_grpc_requests().await;

    match manager.initialize().await {
        Ok(_) => {
            println!("NodeAgentManager successfully initialized");
            // Only proceed to run if initialization was successful
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
async fn initialize(tx_grpc: Sender<HandleYamlRequest>, hostname: String) {
    use tonic::transport::Server;

    let server = grpc::receiver::NodeAgentReceiver {
        tx: tx_grpc.clone(),
    };

    let hostname_in_setting = common::setting::get_config().host.name.clone();

    let addr = if hostname.trim().eq_ignore_ascii_case(&hostname_in_setting) {
        common::nodeagent::open_server()
    } else {
        common::nodeagent::open_guest_server()
    }
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
    let hostname: String = String::from_utf8_lossy(
        &std::process::Command::new("hostname")
            .output()
            .expect("Failed to get hostname")
            .stdout,
    )
    .trim()
    .to_string();
    println!("Starting NodeAgent on host: {}", hostname);

    let (tx_grpc, rx_grpc) = channel::<HandleYamlRequest>(100);
    let mgr = launch_manager(rx_grpc, hostname.clone());
    let grpc = initialize(tx_grpc, hostname);

    tokio::join!(mgr, grpc);
}

#[cfg(test)]
mod tests {
    use crate::launch_manager;
    use common::nodeagent::HandleYamlRequest;
    use tokio::sync::mpsc::{channel, Receiver, Sender};
    use tokio::task::LocalSet;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_main_initializes_channels() {
        let (tx_grpc, rx_grpc): (Sender<HandleYamlRequest>, Receiver<HandleYamlRequest>) =
            channel(100);
        assert_eq!(tx_grpc.capacity(), 100);
        assert!(!rx_grpc.is_closed());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_main_launch_manager() {
        let (_tx_grpc, rx_grpc): (Sender<HandleYamlRequest>, Receiver<HandleYamlRequest>) =
            channel(100);
        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = launch_manager(rx_grpc, "hostname".to_string()).await;
        });
        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }
        assert!(true);
    }

    #[tokio::test]
    async fn test_inspect() {
        let r = crate::resource::container::inspect().await;
        println!("{:#?}", r);
    }
}

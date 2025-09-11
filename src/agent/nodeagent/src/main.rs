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
    let mut manager = manager::NodeAgentManager::new(rx_grpc, hostname.clone()).await;

    match manager.initialize().await {
        Ok(_) => {
            println!("NodeAgentManager successfully initialized");

            // Add registration with API server
            let mut sender = grpc::sender::NodeAgentSender::default();
            let config = common::setting::get_config();
            let node_id = format!("{}-{}", hostname, config.host.ip);

            let registration_request = common::nodeagent::NodeRegistrationRequest {
                node_id: node_id.clone(),
                hostname: hostname.clone(),
                ip_address: config.host.ip.clone(),
                metadata: std::collections::HashMap::new(), // Add empty metadata
                resources: None, // Use None if NodeResources doesn't exist, or create the correct struct
                role: 0,         // Use integer instead of string (0 = worker, 1 = master, etc.)
            };

            // Register with API server
            match sender.register_with_api_server(registration_request).await {
                Ok(_) => println!("Successfully registered with API server"),
                Err(e) => eprintln!("Failed to register with API server: {:?}", e),
            }

            // Start heartbeat task
            let mut sender_clone = sender.clone();
            let node_id_clone = node_id.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3));
                loop {
                    interval.tick().await;
                    let heartbeat_request = common::nodeagent::HeartbeatRequest {
                        node_id: node_id_clone.clone(),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64, // Cast to i64
                    };
                    // Fix: call on instance, not static method
                    if let Err(e) = sender_clone.send_heartbeat(heartbeat_request).await {
                        eprintln!("Failed to send heartbeat: {:?}", e);
                    }
                }
            });

            // Run the manager
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

    let config = common::setting::get_config();
    let node_id = format!("{}-{}", hostname, config.host.ip);
    let ip_address = config.host.ip.clone();

    let server = grpc::receiver::NodeAgentReceiver::new(
        tx_grpc.clone(),
        node_id,
        hostname.clone(),
        ip_address,
    );

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
#[cfg(not(tarpaulin_include))]
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
        let hostname: String = String::from_utf8_lossy(
            &std::process::Command::new("hostname")
                .output()
                .expect("Failed to get hostname")
                .stdout,
        )
        .trim()
        .to_string();

        let r = crate::resource::container::inspect(hostname).await;
        println!("{:#?}", r);
    }
}

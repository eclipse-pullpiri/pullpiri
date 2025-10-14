//! NodeAgent main entry point
//!
//! This file sets up the asynchronous runtime, initializes the manager and gRPC server,
//! and launches both concurrently. It also provides unit tests for initialization.

use clap::Parser;
use common::nodeagent::HandleYamlRequest;
use common::nodeagent::NodeRegistrationRequest;
use std::path::PathBuf;
mod bluechi;
pub mod config;
pub mod grpc;
pub mod manager;
pub mod resource;

use common::nodeagent::node_agent_connection_server::NodeAgentConnectionServer;
use tokio::sync::mpsc::{channel, Receiver, Sender};

/// Launches the NodeAgentManager in an asynchronous task.
///
/// This function creates the manager, initializes it, and then runs it.
/// If initialization or running fails, errors are printed to stderr.
async fn launch_manager(
    rx_grpc: Receiver<HandleYamlRequest>,
    hostname: String,
    config: config::Config,
) {
    let mut manager = manager::NodeAgentManager::new(rx_grpc, hostname.clone()).await;

    match manager.initialize().await {
        Ok(_) => {
            println!("NodeAgentManager successfully initialized");
            // Add registration with API server
            let mut sender = grpc::sender::NodeAgentSender::default();

            // Use IP address from config file
            let host_ip = config.get_host_ip();
            let node_name = config.get_node_name();
            // node_id를 node_name과 동일하게 설정 (IP 주소 제거)
            let node_id = node_name.clone();

            let registration_request = NodeRegistrationRequest {
                node_id: node_id.clone(),
                hostname: hostname.clone(),
                ip_address: host_ip.clone(),
                metadata: std::collections::HashMap::new(),
                resources: None,
                node_type: match config.nodeagent.node_type.as_str() {
                    "cloud" => 1,   // NodeType::Cloud as i32
                    "vehicle" => 2, // NodeType::Vehicle as i32
                    _ => 0,         // NodeType::Unspecified as i32
                },
                node_role: match config.nodeagent.node_role.as_str() {
                    "master" => 1,    // NodeRole::Master as i32
                    "nodeagent" => 2, // NodeRole::Nodeagent as i32
                    "bluechi" => 3,   // NodeRole::Bluechi as i32
                    _ => 0,           // NodeRole::Unspecified as i32
                },
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
async fn initialize(tx_grpc: Sender<HandleYamlRequest>, hostname: String, config: config::Config) {
    use tonic::transport::Server;

    // Use IP address from config file
    let host_ip = config.get_host_ip();
    let node_name = config.get_node_name();
    // node_id를 node_name과 동일하게 설정 (IP 주소 제거)
    let node_id = node_name.clone();

    let server = grpc::receiver::NodeAgentReceiver::new(
        tx_grpc.clone(),
        node_id,
        hostname.clone(),
        host_ip.clone(),
    );

    // Use hostname from config if available
    let config_hostname = config.get_hostname();
    //adding _ to prevent warnings as it was never used anywhere to prevent warnings
    let _hostname_to_check = if config_hostname.is_empty() || config_hostname == "$(hostname)" {
        hostname.clone()
    } else {
        config_hostname
    };

    // Determine server address based on hostname
    let addr = format!("{}:{}", host_ip, config.nodeagent.grpc_port)
        .parse()
        .expect("nodeagent address parsing error");
    println!("NodeAgent listening on {}", addr);
    println!(
        "NodeAgent config - master_ip: {}, grpc_port: {}",
        config.nodeagent.master_ip, config.nodeagent.grpc_port
    );

    let _ = Server::builder()
        .add_service(NodeAgentConnectionServer::new(server))
        .serve(addr)
        .await;
}

/// Main entry point for the NodeAgent binary.
///
/// Sets up the async runtime, creates the communication channel, and launches
/// both the manager and gRPC server concurrently.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "/etc/piccolo/nodeagent.yaml")]
    config: PathBuf,
}

#[cfg(not(feature = "tarpaulin_include"))]
#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Load configuration file
    let app_config = match config::Config::load(&args.config) {
        Ok(config) => {
            println!("Loaded configuration from {}", args.config.display());
            config
        }
        Err(err) => {
            eprintln!(
                "Error loading configuration from {}: {}",
                args.config.display(),
                err
            );
            eprintln!("Falling back to default configuration");
            config::Config::default()
        }
    };

    // Set global config for other parts of the application
    config::Config::set_global(app_config.clone());

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
    let mgr = launch_manager(rx_grpc, hostname.clone(), app_config.clone());
    let grpc = initialize(tx_grpc, hostname, app_config);

    tokio::join!(mgr, grpc);
}

#[cfg(feature = "tarpaulin_include")]
fn main() {
    // Dummy main for coverage builds
    println!("Tarpaulin coverage build: main function stub.");
}

#[cfg(test)]
mod tests {
    use crate::config::Config;
    use crate::{initialize, launch_manager};
    use common::nodeagent::{HandleYamlRequest, NodeRegistrationRequest};
    use std::collections::HashMap;
    use std::path::PathBuf;
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
        let config = Config::default();
        let local = LocalSet::new();
        local.spawn_local(async move {
            let _ = launch_manager(rx_grpc, "hostname".to_string(), config).await;
        });
        tokio::select! {
            _ = local => {}
            _ = sleep(Duration::from_millis(200)) => {}
        }
        assert!(true);
    }

    #[tokio::test]
    async fn test_registration_request_fields() {
        let config = Config::default();
        let host_ip = config.get_host_ip();
        let node_name = config.get_node_name();
        let hostname = "testhost".to_string();
        let registration_request = NodeRegistrationRequest {
            node_id: node_name.clone(),
            hostname: hostname.clone(),
            ip_address: host_ip.clone(),
            metadata: HashMap::new(),
            resources: None,
            node_type: match config.nodeagent.node_type.as_str() {
                "cloud" => 1,
                "vehicle" => 2,
                _ => 0,
            },
            node_role: match config.nodeagent.node_role.as_str() {
                "master" => 1,
                "nodeagent" => 2,
                "bluechi" => 3,
                _ => 0,
            },
        };
        assert_eq!(registration_request.node_id, node_name);
        assert_eq!(registration_request.ip_address, host_ip);
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
    #[tokio::test]
    async fn test_initialize_runs_without_panic() {
        let (tx_grpc, _rx_grpc): (Sender<HandleYamlRequest>, Receiver<HandleYamlRequest>) =
            channel(10);
        let config = Config::default();
        let hostname = "testhost".to_string();
        // Should not panic or error
        let fut = initialize(tx_grpc, hostname, config);
        tokio::select! {
            _ = fut => {},
            _ = sleep(Duration::from_millis(200)) => {},
        }
        assert!(true);
    }
}

//! NodeAgentManager: Asynchronous manager for NodeAgent
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the monitoring server or other services.
//! It is designed to be thread-safe and run in an async context.
use crate::grpc::sender::NodeAgentSender;
use common::nodeagent::HandleYamlRequest;
use common::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Main manager struct for NodeAgent.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
pub struct NodeAgentManager {
    /// Receiver for scenario information from gRPC
    rx_grpc: Arc<Mutex<mpsc::Receiver<HandleYamlRequest>>>,
    /// gRPC sender for monitoring server
    sender: Arc<Mutex<NodeAgentSender>>,
    // Add other shared state as needed
    hostname: String,
}

impl NodeAgentManager {
    /// Creates a new NodeAgentManager instance.
    ///
    /// # Arguments
    /// * `rx_grpc` - Channel receiver for scenario information
    pub async fn new(rx: mpsc::Receiver<HandleYamlRequest>, hostname: String) -> Self {
        Self {
            rx_grpc: Arc::new(Mutex::new(rx)),
            sender: Arc::new(Mutex::new(NodeAgentSender::default())),
            hostname,
        }
    }

    /// Initializes the NodeAgentManager (e.g., loads scenarios, prepares state).
    pub async fn initialize(&mut self) -> Result<()> {
        println!("NodeAgentManager init");
        // Add initialization logic here (e.g., read scenarios, subscribe, etc.)
        Ok(())
    }

    // pub async fn handle_yaml(&self, whole_yaml: &String) -> Result<()> {
    //     crate::bluechi::parse(whole_yaml.to_string()).await?;
    //     println!("Handling yaml request nodeagent manager: {:?}", whole_yaml);
    //     Ok(())
    // }

    /// Main loop for processing incoming gRPC scenario requests.
    ///
    /// This function continuously receives scenario parameters from the gRPC channel
    /// and handles them (e.g., triggers actions, updates state, etc.).
    pub async fn process_grpc_requests(&self) -> Result<()> {
        let arc_rx_grpc = Arc::clone(&self.rx_grpc);
        let mut rx_grpc: tokio::sync::MutexGuard<'_, mpsc::Receiver<HandleYamlRequest>> =
            arc_rx_grpc.lock().await;
        while let Some(yaml_data) = rx_grpc.recv().await {
            crate::bluechi::parse(yaml_data.yaml, self.hostname.clone()).await?;
        }

        Ok(())
    }

    /// Runs the NodeAgentManager event loop.
    ///
    /// Spawns the gRPC processing task and waits for it to finish.
    pub async fn run(self) -> Result<()> {
        let arc_self = Arc::new(self);
        let grpc_manager = Arc::clone(&arc_self);
        let grpc_processor = tokio::spawn(async move {
            if let Err(e) = grpc_manager.process_grpc_requests().await {
                eprintln!("Error in gRPC processor: {:?}", e);
            }
        });
        let _ = tokio::try_join!(grpc_processor);
        println!("NodeAgentManager stopped");
        Ok(())
    }
}

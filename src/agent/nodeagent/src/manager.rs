//! NodeAgentManager: Asynchronous manager for NodeAgent
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the monitoring server or other services.
//! It is designed to be thread-safe and run in an async context.

use common::{
    spec::artifact::{Package, Scenario},
    Result,
};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::grpc::sender::NodeAgentSender;

/// Parameter struct for scenario actions received via gRPC.
#[derive(Debug)]
pub struct NodeAgentParameter {
    pub action: i32,           // Action code (e.g., allow, withdraw, etc.)
    pub scenario: Scenario,    // Scenario details
}

/// Main manager struct for NodeAgent.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
pub struct NodeAgentManager {
    /// Receiver for scenario information from gRPC
    rx_grpc: Arc<Mutex<mpsc::Receiver<NodeAgentParameter>>>,
    /// gRPC sender for monitoring server
    sender: Arc<Mutex<NodeAgentSender>>,
    // Add other shared state as needed
}

impl NodeAgentManager {
    /// Creates a new NodeAgentManager instance.
    ///
    /// # Arguments
    /// * `rx_grpc` - Channel receiver for scenario information
    pub async fn new(rx_grpc: mpsc::Receiver<NodeAgentParameter>) -> Self {
        Self {
            rx_grpc: Arc::new(Mutex::new(rx_grpc)),
            sender: Arc::new(Mutex::new(NodeAgentSender::new())),
        }
    }

    /// Initializes the NodeAgentManager (e.g., loads scenarios, prepares state).
    pub async fn initialize(&mut self) -> Result<()> {
        println!("NodeAgentManager init");
        // Add initialization logic here (e.g., read scenarios, subscribe, etc.)
        Ok(())
    }

    // pub async fn handle_workload(&self, workload_name: &String) -> Result<()> {
    //     crate::bluechi::parse(workload_name.to_string()).await?;
    //     // Handle the workload request
    //     println!("Handling workload request: {:?}", workload_name);
    //     //ToDo : Implement the logic  1. extart etcd Network. 2. using extracted data to control the node
    //     Ok(())
    // }

    /// Main loop for processing incoming gRPC scenario requests.
    ///
    /// This function continuously receives scenario parameters from the gRPC channel
    /// and handles them (e.g., triggers actions, updates state, etc.).
    async fn process_grpc_requests(&self) -> Result<()> {
        loop {
            let scenario_parameter = {
                let mut rx_grpc = self.rx_grpc.lock().await;
                rx_grpc.recv().await
            };
            match scenario_parameter {
                Some(param) => {
                    println!("Received scenario: {:?}", param);
                    // Example usage of sender:
                    // let mut sender = self.sender.lock().await;
                    // sender.trigger_action(...).await?;
                }
                None => {
                    println!("gRPC channel closed");
                    break;
                }
            }
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


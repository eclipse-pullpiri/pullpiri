//! StateManagerManager: Asynchronous manager for StateManager
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the nodeagent or other services.
//! It is designed to be thread-safe and run in an async context.
use common::monitoringserver::ContainerList;
use common::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Main manager struct for StateManager.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
pub struct StateManagerManager {
    /// Receiver for scenario information from gRPC
    rx_grpc: Arc<Mutex<mpsc::Receiver<ContainerList>>>,
}

impl StateManagerManager {
    /// Creates a new StateManagerManager instance.
    ///
    /// # Arguments
    /// * `rx_grpc` - Channel receiver for scenario information
    pub async fn new(rx: mpsc::Receiver<ContainerList>) -> Self {
        Self {
            rx_grpc: Arc::new(Mutex::new(rx)),
        }
    }

    /// Initializes the StateManagerManager (e.g., loads scenarios, prepares state).
    pub async fn initialize(&mut self) -> Result<()> {
        println!("StateManagerManager init");
        // Add initialization logic here (e.g., read scenarios, subscribe, etc.)
        Ok(())
    }

    /// Main loop for processing incoming gRPC ContainerList messages.
    ///
    /// This function continuously receives ContainerList from the gRPC channel
    /// and handles them (e.g., triggers actions, updates state, etc.).
    pub async fn process_grpc_requests(&self) -> Result<()> {
        loop {
            let container_list_opt = {
                let mut rx_grpc = self.rx_grpc.lock().await;
                rx_grpc.recv().await
            };
            if let Some(container_list) = container_list_opt {
                // Handle the received ContainerList
                println!(
                    "Received ContainerList from nodeagent: node_name={}, containers={:?}",
                    container_list.node_name, container_list.containers
                );
                // TODO: Add your processing logic here
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Runs the StateManagerManager event loop.
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
        let _ = grpc_processor.await;
        println!("StateManagerManager stopped");
        Ok(())
    }
}

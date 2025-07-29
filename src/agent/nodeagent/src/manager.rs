//! NodeAgentManager: Asynchronous manager for NodeAgent
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the monitoring server or other services.
//! It is designed to be thread-safe and run in an async context.
use crate::grpc::sender::NodeAgentSender;
use common::monitoringserver::ContainerList;
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

    /// Background task: Periodically gathers container info using inspect().
    ///
    /// This runs in an infinite loop and logs or processes container info as needed.
    async fn gather_container_info_loop(&self) {
        use crate::resource::container::inspect;
        use tokio::time::{sleep, Duration};

        // This is the previous container list for comparison
        let mut previous_container_list = Vec::new();

        loop {
            let container_list = inspect().await.unwrap_or_default();
            let node = self.hostname.clone();

            // Send the container info to the monitoring server
            {
                let mut sender = self.sender.lock().await;
                if let Err(e) = sender
                    .send_container_list(ContainerList {
                        node_name: node.clone(),
                        containers: container_list.clone(),
                    })
                    .await
                {
                    eprintln!("[NodeAgent] Error sending container info: {}", e);
                }
            }

            // Check if the container list is changed from the previous one
            if previous_container_list != container_list {
                println!(
                    "Container list changed for node: {}. Previous: {:?}, Current: {:?}",
                    node, previous_container_list, container_list
                );

                // Save the previous container list for comparison
                previous_container_list = container_list.clone();

                // Send the changed container list to the state manager
                let mut sender = self.sender.lock().await;
                if let Err(e) = sender
                    .send_changed_container_list(ContainerList {
                        node_name: node.clone(),
                        containers: container_list,
                    })
                    .await
                {
                    eprintln!("[NodeAgent] Error sending changed container list: {}", e);
                }
            }

            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Runs the NodeAgentManager event loop.
    ///
    /// Spawns the gRPC processing task and the container info gatherer, and waits for them to finish.
    pub async fn run(self) -> Result<()> {
        let arc_self = Arc::new(self);
        let grpc_manager = Arc::clone(&arc_self);
        let grpc_processor = tokio::spawn(async move {
            if let Err(e) = grpc_manager.process_grpc_requests().await {
                eprintln!("Error in gRPC processor: {:?}", e);
            }
        });
        let container_manager = Arc::clone(&arc_self);
        let container_gatherer = tokio::spawn(async move {
            container_manager.gather_container_info_loop().await;
        });
        let _ = tokio::try_join!(grpc_processor, container_gatherer);
        println!("NodeAgentManager stopped");
        Ok(())
    }
}

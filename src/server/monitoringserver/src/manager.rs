//! MonitoringServerManager: Asynchronous manager for MonitoringServer
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the nodeagent or other services.
//! It is designed to be thread-safe and run in an async context.
use common::monitoringserver::{ContainerList, NodeInfo};
use common::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Main manager struct for MonitoringServer.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
pub struct MonitoringServerManager {
    /// Receiver for container information from gRPC
    rx_container: Arc<Mutex<mpsc::Receiver<ContainerList>>>,
    /// Receiver for node information from gRPC
    rx_node: Arc<Mutex<mpsc::Receiver<NodeInfo>>>,
}

impl MonitoringServerManager {
    /// Creates a new MonitoringServerManager instance.
    ///
    /// # Arguments
    /// * `rx_container` - Channel receiver for container information
    /// * `rx_node` - Channel receiver for node information
    pub async fn new(
        rx_container: mpsc::Receiver<ContainerList>,
        rx_node: mpsc::Receiver<NodeInfo>,
    ) -> Self {
        Self {
            rx_container: Arc::new(Mutex::new(rx_container)),
            rx_node: Arc::new(Mutex::new(rx_node)),
        }
    }

    /// Initializes the MonitoringServerManager (e.g., loads scenarios, prepares state).
    pub async fn initialize(&mut self) -> Result<()> {
        println!("MonitoringServerManager init");
        // Add initialization logic here (e.g., read scenarios, subscribe, etc.)
        Ok(())
    }

    /// Processes ContainerList messages from nodeagent.
    ///
    /// This function handles the received ContainerList and processes it accordingly.
    async fn handle_container_list(&self, container_list: ContainerList) {
        println!(
            "[MonitoringServer] Received ContainerList from {}: containers count={}",
            container_list.node_name,
            container_list.containers.len()
        );

        // Print container details
        for container in &container_list.containers {
            println!(
                "  Container: ID={}, Names={:?}, Image={}",
                container.id, container.names, container.image
            );
        }

        // TODO: Add your container processing logic here
    }

    /// Processes NodeInfo messages from nodeagent.
    ///
    /// This function handles the received NodeInfo and processes it accordingly.
    async fn handle_node_info(&self, node_info: NodeInfo) {
        println!(
            "[MonitoringServer] Received NodeInfo from {}: \
            CPU: {:.2}% ({} cores), GPU: {} units, \
            Memory: {:.2}% ({}/{} KB), \
            Network: RX {} B / TX {} B, \
            Disk: Read {} B / Write {} B, \
            OS: {}, Arch: {}, IP: {}",
            node_info.node_name,
            node_info.cpu_usage,
            node_info.cpu_count,
            node_info.gpu_count,
            node_info.mem_usage,
            node_info.used_memory,
            node_info.total_memory,
            node_info.rx_bytes,
            node_info.tx_bytes,
            node_info.read_bytes,
            node_info.write_bytes,
            node_info.os,
            node_info.arch,
            node_info.ip
        );

        // TODO: Add your node info processing logic here
    }

    /// Main loop for processing incoming gRPC ContainerList messages.
    ///
    /// This function continuously receives ContainerList from the gRPC channel
    /// and handles them using the handle_container_list method.
    pub async fn process_container_requests(&self) -> Result<()> {
        loop {
            let container_list_opt = {
                let mut rx_container = self.rx_container.lock().await;
                rx_container.recv().await
            };
            if let Some(container_list) = container_list_opt {
                self.handle_container_list(container_list).await;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Main loop for processing incoming gRPC NodeInfo messages.
    ///
    /// This function continuously receives NodeInfo from the gRPC channel
    /// and handles them using the handle_node_info method.
    pub async fn process_node_info_requests(&self) -> Result<()> {
        loop {
            let node_info_opt = {
                let mut rx_node = self.rx_node.lock().await;
                rx_node.recv().await
            };
            if let Some(node_info) = node_info_opt {
                self.handle_node_info(node_info).await;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Runs the MonitoringServerManager event loop.
    ///
    /// Spawns both container and node info processing tasks and waits for them to finish.
    pub async fn run(self) -> Result<()> {
        let arc_self = Arc::new(self);

        // Container processor task
        let container_manager = Arc::clone(&arc_self);
        let container_processor = tokio::spawn(async move {
            if let Err(e) = container_manager.process_container_requests().await {
                eprintln!("Container processor error: {:?}", e);
            }
        });

        // NodeInfo processor task
        let node_manager = Arc::clone(&arc_self);
        let node_processor = tokio::spawn(async move {
            if let Err(e) = node_manager.process_node_info_requests().await {
                eprintln!("Node processor error: {:?}", e);
            }
        });

        let _ = tokio::try_join!(container_processor, node_processor);
        println!("MonitoringServerManager stopped");
        Ok(())
    }
}

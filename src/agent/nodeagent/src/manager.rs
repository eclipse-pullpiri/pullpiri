//! NodeAgentManager: Asynchronous manager for NodeAgent
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the monitoring server or other services.
//! It is designed to be thread-safe and run in an async context.
use crate::cluster::{ClusterClient, ClusterConfig};
use crate::grpc::sender::NodeAgentSender;
use common::monitoringserver::{ContainerInfo, ContainerList};
use common::nodeagent::{HandleYamlRequest, NodeRole};
use common::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Main manager struct for NodeAgent.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
/// Now includes clustering functionality for master node communication.
pub struct NodeAgentManager {
    /// Receiver for scenario information from gRPC
    rx_grpc: Arc<Mutex<mpsc::Receiver<HandleYamlRequest>>>,
    /// gRPC sender for monitoring server
    sender: Arc<Mutex<NodeAgentSender>>,
    /// Clustering client for master node communication
    cluster_client: Option<ClusterClient>,
    /// Hostname of this node
    hostname: String,
}

impl NodeAgentManager {
    /// Creates a new NodeAgentManager instance.
    ///
    /// # Arguments
    /// * `rx_grpc` - Channel receiver for scenario information
    /// * `hostname` - Hostname of this node
    pub async fn new(rx: mpsc::Receiver<HandleYamlRequest>, hostname: String) -> Self {
        Self {
            rx_grpc: Arc::new(Mutex::new(rx)),
            sender: Arc::new(Mutex::new(NodeAgentSender::default())),
            cluster_client: None,
            hostname,
        }
    }

    /// Creates a new NodeAgentManager with clustering enabled.
    ///
    /// # Arguments
    /// * `rx_grpc` - Channel receiver for scenario information
    /// * `hostname` - Hostname of this node
    /// * `master_endpoint` - Endpoint of the master node (e.g., "http://192.168.1.1:47100")
    pub async fn new_with_clustering(
        rx: mpsc::Receiver<HandleYamlRequest>,
        hostname: String,
        master_endpoint: String,
    ) -> Self {
        let cluster_config = ClusterConfig {
            master_endpoint,
            hostname: hostname.clone(),
            node_role: NodeRole::Sub,
            ..Default::default()
        };

        let cluster_client = ClusterClient::new(cluster_config);

        Self {
            rx_grpc: Arc::new(Mutex::new(rx)),
            sender: Arc::new(Mutex::new(NodeAgentSender::default())),
            cluster_client: Some(cluster_client),
            hostname,
        }
    }

    /// Initializes the NodeAgentManager (e.g., loads scenarios, prepares state).
    /// Now includes clustering initialization if enabled.
    pub async fn initialize(&mut self) -> Result<()> {
        println!("NodeAgentManager init");

        // Initialize clustering if enabled
        if let Some(cluster_client) = &self.cluster_client {
            println!("Initializing clustering...");
            match cluster_client.initialize().await {
                Ok(_) => {
                    println!("Clustering initialized successfully");
                    // Start background tasks for heartbeat and status reporting
                    cluster_client.start_background_tasks().await;
                }
                Err(e) => {
                    eprintln!(
                        "Failed to initialize clustering: {}. Running in standalone mode.",
                        e
                    );
                    // Continue running in standalone mode
                }
            }
        } else {
            println!("Running in standalone mode (no clustering)");
        }

        Ok(())
    }

    /// Check if clustering is enabled and connected
    pub fn is_clustering_enabled(&self) -> bool {
        self.cluster_client.is_some()
    }

    /// Check if connected to master node (only meaningful if clustering is enabled)
    pub fn is_connected_to_master(&self) -> bool {
        self.cluster_client
            .as_ref()
            .map(|client| client.is_connected())
            .unwrap_or(false)
    }

    /// Get the node ID assigned by the master (if any)
    pub async fn get_node_id(&self) -> Option<String> {
        if let Some(cluster_client) = &self.cluster_client {
            cluster_client.get_node_id().await
        } else {
            None
        }
    }

    /// Get clustering status information
    pub async fn get_clustering_status(&self) -> String {
        if let Some(cluster_client) = &self.cluster_client {
            let connected = cluster_client.is_connected();
            let node_id = cluster_client.get_node_id().await;

            match (connected, node_id) {
                (true, Some(id)) => format!("Connected to cluster as node: {}", id),
                (true, None) => "Connected but not registered".to_string(),
                (false, Some(id)) => format!("Disconnected (last known node ID: {})", id),
                (false, None) => "Disconnected and not registered".to_string(),
            }
        } else {
            "Clustering disabled".to_string()
        }
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
            let container_list = inspect(self.hostname.clone()).await.unwrap_or_default();
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

            // Check if the container list is changed from the previous one except for ContainerList.stats
            // (which is not included in the comparison)
            if !containers_equal_except_stats(&previous_container_list, &container_list) {
                // println!(
                //     "Container list changed for node: {}. Previous: {:?}, Current: {:?}",
                //     node, previous_container_list, container_list
                // );

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

    /// Background task: Periodically gathers system info using extract_system_info().
    ///
    /// This runs in an infinite loop and logs or processes system info as needed.
    async fn gather_node_info_loop(&self) {
        use crate::resource::nodeinfo::extract_node_info_delta;
        use common::monitoringserver::NodeInfo;
        use tokio::time::{sleep, Duration};

        loop {
            let node_info_data = extract_node_info_delta();

            // Create NodeInfo message for gRPC
            let node_info = NodeInfo {
                node_name: self.hostname.clone(),
                cpu_usage: node_info_data.cpu_usage as f64,
                cpu_count: node_info_data.cpu_count as u64,
                gpu_count: node_info_data.gpu_count as u64,
                used_memory: node_info_data.used_memory,
                total_memory: node_info_data.total_memory,
                mem_usage: node_info_data.mem_usage as f64,
                rx_bytes: node_info_data.rx_bytes,
                tx_bytes: node_info_data.tx_bytes,
                read_bytes: node_info_data.read_bytes,
                write_bytes: node_info_data.write_bytes,
                os: node_info_data.os,
                arch: node_info_data.arch,
                ip: node_info_data.ip,
            };

            // Send NodeInfo to monitoring server
            {
                let mut sender = self.sender.lock().await;
                if let Err(e) = sender.send_node_info(node_info.clone()).await {
                    eprintln!("[NodeAgent] Error sending node info: {}", e);
                }
            }

            println!(
                "[NodeInfo] CPU: {:.2}%, CPU Count: {}, GPU Count: {}, Mem: {}/{} KB ({:.2}%), Net RX: {} B, Net TX: {} B, Disk Read: {} B, Disk Write: {} B, OS: {}, Arch: {}, IP: {}",
                node_info.cpu_usage,
                node_info.cpu_count,
                node_info.gpu_count,
                node_info.used_memory,
                node_info.total_memory,
                node_info.mem_usage,
                node_info.rx_bytes,
                node_info.tx_bytes,
                node_info.read_bytes,
                node_info.write_bytes,
                node_info.os,
                node_info.arch,
                node_info.ip
            );
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

        // Spawn a background task to periodically extract and print system info
        let nodeinfo_manager = Arc::clone(&arc_self);
        let nodeinfo_task = tokio::spawn(async move {
            nodeinfo_manager.gather_node_info_loop().await;
        });
        let _ = tokio::try_join!(grpc_processor, container_gatherer, nodeinfo_task);
        println!("NodeAgentManager stopped");
        Ok(())
    }
}

fn containers_equal_except_stats<'a>(a: &'a [ContainerInfo], b: &'a [ContainerInfo]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(c1, c2)| {
        c1.id == c2.id
            && c1.names == c2.names
            && c1.image == c2.image
            && c1.state == c2.state
            && c1.config == c2.config
            && c1.annotation == c2.annotation
        // do NOT compare c1.stats/c2.stats
    })
}

//unit test cases
#[cfg(test)]
mod tests {
    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;
    use crate::manager::NodeAgentManager;
    use common::nodeagent::HandleYamlRequest;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_new_creates_instance_with_correct_hostname() {
        let (_tx, rx) = mpsc::channel(1);
        let hostname = "test-host".to_string();

        let manager = NodeAgentManager::new(rx, hostname.clone()).await;

        assert_eq!(manager.hostname, hostname);
    }

    #[tokio::test]
    async fn test_initialize_returns_ok() {
        let (_tx, rx) = mpsc::channel(1);
        let hostname = "test-host".to_string();

        let mut manager = NodeAgentManager::new(rx, hostname).await;
        let result = manager.initialize().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_grpc_requests_handles_empty_channel() {
        let (_tx, rx) = mpsc::channel(1);
        drop(_tx); // close sender so recv returns None immediately
        let hostname = "test-host".to_string();

        let manager = NodeAgentManager::new(rx, hostname).await;
        let result = manager.process_grpc_requests().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_grpc_requests_receives_and_parses_yaml() {
        let (tx, rx) = mpsc::channel(1);
        let hostname = "test-host".to_string();

        let manager = NodeAgentManager::new(rx, hostname.clone()).await;

        let yaml_string = VALID_ARTIFACT_YAML.to_string();
        let request = HandleYamlRequest {
            yaml: yaml_string.clone(),
        };

        assert!(tx.send(request).await.is_ok());
        drop(tx);

        let result = manager.process_grpc_requests().await;
        assert!(result.is_ok());
    }
}

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

        tx.send(request).await.unwrap();
        drop(tx);

        let result = manager.process_grpc_requests().await;
        assert!(result.is_ok());
    }
}

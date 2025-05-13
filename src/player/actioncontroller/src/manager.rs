use crate::runtime::bluechi;
use common::{
    actioncontroller::Status,
    spec::artifact::{Package, Scenario},
    Result,
};

/// Manager for coordinating scenario actions and workload operations
///
/// Responsible for:
/// - Processing scenario requests from gRPC receivers
/// - Determining appropriate actions based on scenario definitions
/// - Delegating workload operations to the appropriate runtime (Bluechi or NodeAgent)
/// - Handling state reconciliation for scenario workloads
pub struct ActionControllerManager {
    /// List of nodes managed by Bluechi
    pub bluechi_nodes: Vec<String>,
    /// List of nodes managed by NodeAgent
    pub nodeagent_nodes: Vec<String>,
    // Add other fields as needed
}

impl ActionControllerManager {
    /// Creates a new ActionControllerManager instance
    ///
    /// Initializes the manager with empty node lists. Node information
    /// should be populated after creation.
    ///
    /// # Returns
    ///
    /// A new ActionControllerManager instance
    pub fn new() -> Self {
        let mut bluechi_nodes = Vec::new();
        let mut nodeagent_nodes = Vec::new();
        let settings = common::setting::get_config();

        if settings.host.r#type == "bluechi" {
            bluechi_nodes.push(settings.host.name.clone());
        } else if settings.host.r#type == "nodeagent" {
            nodeagent_nodes.push(settings.host.name.clone());
        }

        if let Some(guests) = &settings.guest {
            for guest in guests {
                if guest.r#type == "bluechi" {
                    bluechi_nodes.push(guest.name.clone());
                } else if guest.r#type == "nodeagent" {
                    nodeagent_nodes.push(guest.name.clone());
                }
            }
        }

        Self {
            bluechi_nodes,
            nodeagent_nodes,
        }
    }

    /// Processes a trigger action request for a specific scenario
    ///
    /// Retrieves scenario information from ETCD and performs the
    /// appropriate actions based on the scenario definition.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario to trigger
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the action was triggered successfully
    /// * `Err(...)` if the action could not be triggered
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The scenario is not allowed by policy
    /// - The runtime operation fails
    pub async fn trigger_manager_action(&self, scenario_name: &str) -> Result<()> {
        if scenario_name.trim().is_empty() {
            println!("Invalid scenario name: empty or whitespace only");
            return Err("Invalid scenario name: cannot be empty".into());
        }
        let etcd_scenario_key = format!("scenario/{}", scenario_name);
        let scenario_str: String = match common::etcd::get(&etcd_scenario_key).await {
            Ok(value) => value,
            Err(e) => {
                println!("Failed to retrieve scenario data for '{}': {}", scenario_name, e);
                return Err(format!("Scenario '{}' not found: {}", scenario_name, e).into());
            }
        };
        let scenario: Scenario = serde_yaml::from_str(&scenario_str)?;

        let action = scenario.get_actions();

        let etcd_package_key: String = format!("package/{}", scenario.get_targets());
        let package_str = common::etcd::get(&etcd_package_key).await?;
        let package: Package = serde_yaml::from_str(&package_str)?;

        for mi in package.get_models() {
            let model_name = format!("{}.service", mi.get_name());
            let model_node = mi.get_node();
            let node_type = if self.bluechi_nodes.contains(&model_node) {
                "bluechi"
            } else if self.nodeagent_nodes.contains(&model_node) {
                "nodeagent"
            } else {
                continue; // Skip if node type is unknown
            };

            match action.as_str() {
                "launch" => {
                    self.start_workload(&model_name, &model_node, &node_type)
                        .await?;
                }
                "terminate" => {
                    self.stop_workload(&model_name, &model_node, &node_type)
                        .await?;
                }
                "update" | "rollback" => {
                    self.stop_workload(&model_name, &model_node, &node_type)
                        .await?;
                    self.start_workload(&model_name, &model_node, &node_type)
                        .await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Reconciles current and desired states for a scenario
    ///
    /// Compares the current state with the desired state for a given scenario
    /// and performs the necessary actions to align them.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    /// * `current` - Current state value
    /// * `desired` - Desired state value
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the reconciliation was successful
    /// * `Err(...)` if the reconciliation failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The reconciliation action fails
    pub async fn reconcile_do(
        &self,
        scenario_name: String,
        current: Status,
        desired: Status,
    ) -> Result<()> {
        if current == desired {
            return Ok(());
        }

        let etcd_scenario_key = format!("scenario/{}", scenario_name);
        let scenario_str = common::etcd::get(&etcd_scenario_key).await?;
        let scenario: Scenario = serde_yaml::from_str(&scenario_str)?;

        let etcd_package_key = format!("package/{}", scenario.get_targets());
        let package_str = common::etcd::get(&etcd_package_key).await?;
        let package: Package = serde_yaml::from_str(&package_str)?;

        for mi in package.get_models() {
            let model_name = format!("{}.service", mi.get_name());
            let model_node = mi.get_node();
            let node_type = if self.bluechi_nodes.contains(&model_node) {
                "bluechi"
            } else if self.nodeagent_nodes.contains(&model_node) {
                "nodeagent"
            } else {
                continue; // Skip if node type is unknown
            };

            match desired {
                Status::Running => {
                    self.start_workload(&model_name, &model_node, &node_type)
                        .await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Creates a new workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was created successfully
    /// * `Err(...)` if the workload creation failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload already exists
    /// - The runtime operation fails
    pub async fn create_workload(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Deletes an existing workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was deleted successfully
    /// * `Err(...)` if the workload deletion failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The runtime operation fails
    pub async fn delete_workload(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Restarts an existing workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was restarted successfully
    /// * `Err(...)` if the workload restart failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The runtime operation fails
    pub async fn restart_workload(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Pauses an active workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was paused successfully
    /// * `Err(...)` if the workload pause failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The workload is not in a pausable state
    /// - The runtime operation fails
    pub async fn pause_workload(&self, scenario_name: String) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Starts a paused or stopped workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was started successfully
    /// * `Err(...)` if the workload start failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The workload is not in a startable state
    /// - The runtime operation fails
    pub async fn start_workload(
        &self,
        model_name: &str,
        node_name: &str,
        node_type: &str,
    ) -> Result<()> {
        match node_type {
            "bluechi" => {
                let cmd = bluechi::BluechiCmd {
                    command: bluechi::Command::UnitStart,
                };
                bluechi::handle_bluechi_cmd(&model_name, &node_name, cmd).await?;
            }
            "nodeagent" => {
                // let runtime = crate::runtime::nodeagent::NodeAgentRuntime::new();
                // runtime.start_workload(model_name).await?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Stops an active workload for the specified scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the workload was stopped successfully
    /// * `Err(...)` if the workload stop failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario does not exist
    /// - The workload does not exist
    /// - The workload is already stopped
    /// - The runtime operation fails
    pub async fn stop_workload(
        &self,
        model_name: &str,
        node_name: &str,
        node_type: &str,
    ) -> Result<()> {
        match node_type {
            "bluechi" => {
                let cmd = bluechi::BluechiCmd {
                    command: bluechi::Command::UnitStop,
                };
                bluechi::handle_bluechi_cmd(&model_name, &node_name, cmd).await?;
            }
            "nodeagent" => {
                // let runtime = crate::runtime::nodeagent::NodeAgentRuntime::new();
                // runtime.start_workload(model_name).await?;
            }
            _ => {}
        }
        Ok(())
    }
}

//UNIT TEST SKELTON

#[cfg(test)]
    mod tests {
        use std::error::Error;

        use super::*;
        use crate::runtime::bluechi::handle_bluechi_cmd;
        use common::actioncontroller::Status;

        #[tokio::test]
        async fn test_reconcile_do_with_valid_status() {
            // Valid scenario where reconcile_do transitions status successfully
            let manager = ActionControllerManager {
                bluechi_nodes: vec!["bluechi-node1".to_string()],
                nodeagent_nodes: vec![],
            };

            let result = manager
                .reconcile_do("test_scenario".into(), Status::None, Status::Running)
                .await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_trigger_manager_action_with_valid_data() {
            // etcd에 테스트 데이터 설정
            common::etcd::put("scenario/test_scenario", r#"
        apiVersion: v1
        kind: Scenario
        metadata:
          name: test_scenario
        spec:
          actions: launch
          targets: test_package
        "#).await.unwrap();
        
            common::etcd::put("package/test_package", r#"
        apiVersion: v1
        kind: Package
        metadata:
          name: test_package
        spec:
          pattern:
            - type: plain
          models:
            - name: test-model
              node: HOST
              resources:
                volume: test-volume
                network: test-network
        "#).await.unwrap();
        
            // 필요한 리소스 데이터 추가
            common::etcd::put("volume/test-volume", r#"
        apiVersion: v1
        kind: Volume
        metadata:
          name: test-volume
        spec: null
        "#).await.unwrap();
        
            common::etcd::put("network/test-network", r#"
        apiVersion: v1
        kind: Network
        metadata:
          name: test-network
        spec: null
        "#).await.unwrap();
        
            // bluechi_nodes에 test-model의 노드(bluechi-node1)를 포함하는 매니저 생성
            let manager = ActionControllerManager {
                bluechi_nodes: vec!["HOST".to_string()],
                nodeagent_nodes: vec![],
            };
        
            // bluechi 모듈을 모킹하여 실제 시스템 호출이 발생하지 않도록 함
            // (이 부분은 실제 코드에 맞게 수정 필요)
        
            let result = manager.trigger_manager_action("test_scenario").await;
            assert!(result.is_ok());
        
            // 테스트 후 정리
            // common::etcd::delete("scenario/test_scenario").await.unwrap();
            // common::etcd::delete("package/test_package").await.unwrap();
            // common::etcd::delete("volume/test-volume").await.unwrap();
            // common::etcd::delete("network/test-network").await.unwrap();
        }

        #[tokio::test]
        async fn test_trigger_manager_action_invalid_scenario() {
            // Negative case: nonexistent scenario key
            let manager = ActionControllerManager {
                bluechi_nodes: vec!["bluechi".to_string()],
                nodeagent_nodes: vec![],
            };

            let result = manager.trigger_manager_action("invalid_scenario").await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_reconcile_do_invalid_scenario_key() {
            // Negative case: nonexistent scenario key returns error
            let manager = ActionControllerManager {
                bluechi_nodes: vec!["bluechi-node1".to_string()],
                nodeagent_nodes: vec![],
            };

            let result = manager
                .reconcile_do("invalid_scenario".into(), Status::None, Status::Running)
                .await;
            assert!(result.is_err());
        }

        #[tokio::test]
        async fn test_start_workload_invalid_node_type() {
            // Negative case: unknown node type returns Ok but does nothing
            let manager = ActionControllerManager {
                bluechi_nodes: vec![],
                nodeagent_nodes: vec![],
            };

            let result: std::result::Result<(), Box<dyn Error>> = manager
                .start_workload("model-a", "unknown-node", "invalid_type")
                .await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_stop_workload_invalid_node_type() {
            // Negative case: unknown node type returns Ok but does nothing
            let manager = ActionControllerManager {
                bluechi_nodes: vec![],
                nodeagent_nodes: vec![],
            };

            let result = manager
                .stop_workload("model-a", "unknown-node", "invalid_type")
                .await;
            assert!(result.is_ok());
        }

        #[test]
        fn test_manager_initializes_nodes() {
            // Ensures new() returns manager with non-empty nodes
            let manager = ActionControllerManager::new();
            assert!(!manager.bluechi_nodes.is_empty() || !manager.nodeagent_nodes.is_empty());
        }

        #[tokio::test]
        async fn test_create_delete_restart_pause_are_noops() {
            // All of these are currently no-op, so they should succeed regardless of input
            let manager = ActionControllerManager {
                bluechi_nodes: vec![],
                nodeagent_nodes: vec![],
            };

            assert!(manager.create_workload("test".into()).await.is_ok());
            assert!(manager.delete_workload("test".into()).await.is_ok());
            assert!(manager.restart_workload("test".into()).await.is_ok());
            assert!(manager.pause_workload("test".into()).await.is_ok());
        }
    }

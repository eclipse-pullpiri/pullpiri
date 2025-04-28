use common::{
    spec::artifact::{Package, Scenario},
    Result,
};
use crate::runtime::bluechi;
use std::fs;
use serde_json::{
    Value,
};
const JSON: &str = "./config/scenario_config.json";


/// Manager for coordinating scenario actions and workload operations
///
/// Responsible for:
/// - Processing scenario requests from gRPC receivers
/// - Determining appropriate actions based on scenario definitions
/// - Delegating workload operations to the appropriate runtime (Bluechi or NodeAgent)
/// - Handling state reconciliation for scenario workloads
pub struct ActionControllerManager {
    /// List of nodes managed by Bluechi
    bluechi_nodes: Vec<String>,
    /// List of nodes managed by NodeAgent
    nodeagent_nodes: Vec<String>,
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
        Self {
            bluechi_nodes: Vec::new(),
            nodeagent_nodes: Vec::new(),
            // Initialize other fields
        }
    }


    async fn node_type(&self, path: &str) -> Result<String> {
        let file_path = format!("./scenarios/{}.json", path);
        let file_content = fs::read_to_string(&file_path)?;
        let json_data: Value = serde_json::from_str(&file_content)?;
        
        let api_type = json_data["api_type"].to_string();

        Ok(api_type)
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
        // TODO: Implementation
        let etcd_scenario_key = format!("scenario/{}", scenario_name);
        let scenario_str = common::etcd::get(&etcd_scenario_key).await?;
        let scenario: Scenario = serde_yaml::from_str(&scenario_str)?;

        let action = scenario.get_actions();

        let etcd_package_key = format!("package/{}", scenario.get_targets());
        let package_str = common::etcd::get(&etcd_package_key).await?;
        let package: Package = serde_yaml::from_str(&package_str)?;
        let node_type = self.node_type(JSON).await?;
        for mi in package.get_models() {
            let model_name = mi.get_name();
            let model_node = mi.get_node();

            match action.as_str() {
                "launch" => {
                    self.start_workload(&model_name, &model_node, &node_type).await?;
                }
                "terminate" => {
                    self.stop_workload(&model_name, &model_node, &node_type).await?;
                }
                "update" | "rollback" => {
                    self.stop_workload(&model_name, &model_node, &node_type).await?;
                    self.start_workload(&model_name, &model_node, &node_type).await?;
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
        current: i32,
        desired: i32,
    ) -> Result<()> {
        // TODO: Implementation
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
    pub async fn start_workload(&self, model_name: &str, node_name: &str, node_type: &str) -> Result<()> {
        match node_type {
            "bluechi" => {
                let cmd = bluechi::BluechiCmd {command: bluechi::Command::UnitStart,};
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
    pub async fn stop_workload(&self, model_name: &str, node_name: &str, node_type: &str) -> Result<()> {
        match node_type {
            "bluechi" => {
                let cmd = bluechi::BluechiCmd {command: bluechi::Command::UnitStop,};
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

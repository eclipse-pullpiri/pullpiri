use common::Result;
// pub mod controller;
// pub mod node;
// pub mod unit;

use dbus::blocking::{Connection, Proxy};
use dbus::Path;
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc::Receiver;

const DEST: &str = "org.eclipse.bluechi";
const PATH: &str = "/org/eclipse/bluechi";
const DEST_CONTROLLER: &str = "org.eclipse.bluechi.Controller";
const DEST_NODE: &str = "org.eclipse.bluechi.Node";
/// Runtime implementation for Bluechi API interactions
///
/// Handles workload operations for nodes managed by Bluechi,
/// interfacing with the Bluechi Controller API to perform
/// operations like creating, starting, stopping, and deleting workloads.
pub struct BluechiRuntime {
    /// Connection to the Bluechi Controller
    connection: Connection,
    /// Cache of node information for quick access
    node_cache: HashMap<String, String>,
}

impl super::Runtime for BluechiRuntime {
    /// Create a new BluechiRuntime instance
    ///
    /// Initializes a runtime handler for Bluechi operations without
    /// establishing a connection. Use `connect()` to establish the connection.
    ///
    /// # Returns
    ///
    /// A new BluechiRuntime instance
    fn new() -> Self {
        BluechiRuntime {
            connection: Connection::new_system().unwrap(),
            node_cache: HashMap::new(),
        }
    }

    /// Establish connection to the Bluechi Controller and initialize node cache
    ///
    /// # Returns
    ///
    /// * `Ok(())` if connection was successful
    /// * `Err(...)` if connection failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The Bluechi Controller is not reachable
    /// - Authentication fails
    async fn init(&mut self) -> Result<()> {
        let bluechi = self.connection.with_proxy(DEST, PATH, Duration::from_millis(5000));

        // Fetch the main node
        let (node,): (Path,) = bluechi
            .method_call(
                DEST_CONTROLLER,
                "GetNode",
                (&common::setting::get_config().host.name,),
            )
            .unwrap();
        self.node_cache.insert(
            common::setting::get_config().host.name.clone(),
            node.to_string(),
        );

        // Fetch guest nodes if available
        if let Some(guests) = &common::setting::get_config().guest {
            for guest in guests {
                let (node,): (Path,) = bluechi
                    .method_call(DEST_CONTROLLER, "GetNode", (&guest.name,))
                    .unwrap();
                self.node_cache.insert(
                    guest.name.clone(),
                    node.to_string(),
                );
            }
        }
        Ok(())
    }

    /// Get a Proxy object for a node
    ///
    /// # Arguments
    ///
    /// * `node_name` - Name of the node
    ///
    /// # Returns
    ///
    /// * `Some(Proxy)` - Returns a Proxy object if the node exists in cache
    /// * `None` - If the node does not exist in cache
    pub fn get_node_proxy<'a>(&'a self, node_name: &str) -> Option<Proxy<'a, &'a Connection>> {
        self.node_cache.get(node_name).map(|node_path| {
            self.connection.with_proxy(DEST, Path::from(node_path.clone()), Duration::from_millis(5000))
        })
    }

    /// Create a workload using Bluechi API
    ///
    /// Reads the scenario definition and creates the corresponding workload
    /// using the Bluechi Controller API.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario to create workload for
    ///
    /// # Returns
    ///
    /// * `Ok(())` if workload creation was successful
    /// * `Err(...)` if workload creation failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The scenario definition is invalid
    /// - The Bluechi API call fails
    /// - The workload already exists
    async fn create_workload(&self, scenario_name: &str) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Delete a workload using Bluechi API
    ///
    /// Removes an existing workload through the Bluechi Controller API.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario that owns the workload
    ///
    /// # Returns
    ///
    /// * `Ok(())` if workload deletion was successful
    /// * `Err(...)` if workload deletion failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The workload does not exist
    /// - The Bluechi API call fails
    async fn delete_workload(&self, scenario_name: &str) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Restart a workload using Bluechi API
    ///
    /// Restarts an existing workload through the Bluechi Controller API.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario that owns the workload
    ///
    /// # Returns
    ///
    /// * `Ok(())` if workload restart was successful
    /// * `Err(...)` if workload restart failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The workload does not exist
    /// - The Bluechi API call fails
    async fn restart_workload(&self, scenario_name: &str) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Pause a workload using Bluechi API
    ///
    /// Suspends execution of a workload through the Bluechi Controller API.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario that owns the workload
    ///
    /// # Returns
    ///
    /// * `Ok(())` if workload pause was successful
    /// * `Err(...)` if workload pause failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The workload does not exist
    /// - The workload is not in a pausable state
    /// - The Bluechi API call fails
    async fn pause_workload(&self, scenario_name: &str) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Start a workload using Bluechi API
    ///
    /// Starts an existing workload through the Bluechi Controller API.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario that owns the workload
    ///
    /// # Returns
    ///
    /// * `Ok(())` if workload start was successful
    /// * `Err(...)` if workload start failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The workload does not exist
    /// - The workload is already running
    /// - The Bluechi API call fails
    async fn start_workload(&self, scenario_name: &str) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    /// Stop a workload using Bluechi API
    ///
    /// Stops an existing workload through the Bluechi Controller API.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario that owns the workload
    ///
    /// # Returns
    ///
    /// * `Ok(())` if workload stop was successful
    /// * `Err(...)` if workload stop failed
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The workload does not exist
    /// - The workload is already stopped
    /// - The Bluechi API call fails
    async fn stop_workload(&self, scenario_name: &str) -> Result<()> {
        // TODO: Implementation
        Ok(())
    }

    // pub struct BluechiCmd {
    //     pub command: Command,
    //     pub node: Option<String>,
    //     pub unit: Option<String>,
    // }
    
    // #[allow(dead_code)]
    // pub enum Command {
    //     ControllerListNode,
    //     ControllerReloadAllNodes,
    //     NodeListUnit,
    //     NodeReload,
    //     UnitStart,
    //     UnitStop,
    //     UnitRestart,
    //     UnitReload,
    //     UnitEnable,
    //     UnitDisable,
    // }
   
}
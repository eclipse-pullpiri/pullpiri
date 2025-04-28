use dbus::blocking::{Connection, Proxy};
use dbus::Path;
use std::{collections::HashMap, time::Duration};
use common::Result;
use common::setting::get_config;

const DEST: &str = "org.eclipse.bluechi";
const PATH: &str = "/org/eclipse/bluechi";
const DEST_CONTROLLER: &str = "org.eclipse.bluechi.Controller";
const DEST_NODE: &str = "org.eclipse.bluechi.Node";

/// Command structure for Bluechi operations
///
/// Contains the command type to execute and optional node and unit
/// information needed to target specific components.
pub struct BluechiCmd {
    pub command: Command,
}

/// Commands supported by the Bluechi runtime
///
/// Represents the various operations that can be performed
/// on the Bluechi controller, nodes, and units.
#[allow(dead_code)]
pub enum Command {
    ControllerReloadAllNodes,
    UnitStart,
    UnitStop,
    UnitRestart,
    UnitReload,
}

impl Command {
    /// Convert command enum to the corresponding D-Bus method name
    ///
    /// Maps each command type to its corresponding Bluechi D-Bus method name
    /// that will be used in the actual method call.
    ///
    /// # Returns
    ///
    /// The D-Bus method name as a string slice
    fn to_method_name(&self) -> &str {
        match self {
            Command::UnitStart => "StartUnit",
            Command::UnitStop => "StopUnit",
            Command::UnitRestart => "RestartUnit",
            Command::UnitReload => "ReloadUnit",
            _ => "Unknown",
        }
    }
}

/// Handle Bluechi commands for operations
///
/// This function processes a single Bluechi command by:
/// 1. Establishing a D-Bus connection to the Bluechi controller
/// 2. Executing the appropriate operation based on the command type
///
/// # Arguments
///
/// * `scenario_name` - Name of the scenario to operate on
/// * `node` - Name of the node to target
/// * `bluechi_cmd` - The Bluechi command to execute
pub async fn handle_bluechi_cmd(scenario_name: &str, node: &str, bluechi_cmd: BluechiCmd) -> Result<()>{

    let conn = Connection::new_system().unwrap();
    let bluechi = conn.with_proxy(DEST, PATH, Duration::from_millis(5000));

    match bluechi_cmd.command {
        Command::ControllerReloadAllNodes => {
            let _ = reload_all_nodes(&bluechi);
        }
        Command::UnitStart
        | Command::UnitStop
        | Command::UnitRestart
        | Command::UnitReload => {
            let _ = workload_run(&conn, bluechi_cmd.command.to_method_name(), node, &bluechi, scenario_name);
        }
    }
    Ok(())
}

/// Execute a unit-related operation on a Bluechi node
///
/// Calls the specified D-Bus method on the given node proxy to perform
/// operations like starting, stopping, restarting, or reloading a unit.
///
/// # Arguments
///
/// * `method` - The D-Bus method name to call (e.g., "StartUnit", "StopUnit")
/// * `node_proxy` - The proxy to the Bluechi node where the unit is located
/// * `unit_name` - The name of the unit to operate on
///
/// # Returns
///
/// * `Ok(String)` - A successful result message including the job path
/// * `Err(...)` - If the D-Bus call fails
pub async fn workload_run(conn: &Connection, method: &str, node_name: &str, proxy: &Proxy<'_, &Connection>, unit_name: &str) -> Result<String> {
    let (node,): (Path,) =
        proxy.method_call(DEST_CONTROLLER, "GetNode", (&node_name,))?;

    let node_proxy = conn.with_proxy(DEST, node, Duration::from_millis(5000));

    let (job_path,): (Path,) =
        node_proxy.method_call(DEST_NODE, method, (unit_name, "replace"))?;

    Ok(format!("{method} '{unit_name}' : {job_path}\n"))
}

/// Reload all nodes managed by the Bluechi controller
///
/// This function:
/// 1. Lists all nodes registered with the controller
/// 2. Creates a proxy for each node
/// 3. Calls the Reload method on each node
/// 4. Collects status information for reporting
///
/// # Arguments
///
/// * `proxy` - The proxy to the Bluechi controller
///
/// # Returns
///
/// * `Ok(String)` - A successful result message listing all reloaded nodes
/// * `Err(...)` - If any of the D-Bus calls fail
pub async fn reload_all_nodes(proxy: &Proxy<'_, &Connection>) -> Result<String> {
    let (nodes,): (Vec<(String, dbus::Path, String)>,) =
        proxy.method_call(DEST_CONTROLLER, "ListNodes", ())?;

    let conn = Connection::new_system()?;
    let mut result = String::new();
    for (node_name, _, _) in nodes {
        let (node,): (Path,) =
            proxy.method_call(DEST_CONTROLLER, "GetNode", (&node_name,))?;

        let node_proxy = conn.with_proxy(DEST, node, Duration::from_millis(5000));
        node_proxy.method_call::<(), _, _, _>(DEST_NODE, "Reload", ())?;

        result.push_str(&format!("Node - {} is reloaded.\n", &node_name));
    }
    Ok(result)
}


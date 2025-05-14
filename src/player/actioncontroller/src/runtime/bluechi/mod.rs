use common::Result;
use dbus::blocking::{Connection, Proxy};
use dbus::Path;
use std::time::Duration;

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
#[allow(clippy::let_underscore_future)]
pub async fn handle_bluechi_cmd(
    scenario_name: &str,
    node: &str,
    bluechi_cmd: BluechiCmd,
) -> Result<()> {
    let conn = Connection::new_system().unwrap();
    let bluechi = conn.with_proxy(DEST, PATH, Duration::from_millis(5000));

    match bluechi_cmd.command {
        Command::ControllerReloadAllNodes => {
            let _ = reload_all_nodes(&bluechi);
        }
        Command::UnitStart | Command::UnitStop | Command::UnitRestart | Command::UnitReload => {
            let _ = workload_run(
                &conn,
                bluechi_cmd.command.to_method_name(),
                node,
                &bluechi,
                scenario_name,
            );
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
pub async fn workload_run(
    conn: &Connection,
    method: &str,
    node_name: &str,
    proxy: &Proxy<'_, &Connection>,
    unit_name: &str,
) -> Result<String> {
    let (node,): (Path,) = proxy.method_call(DEST_CONTROLLER, "GetNode", (&node_name,))?;

    let node_proxy = conn.with_proxy(DEST, node, Duration::from_millis(5000));

    let (job_path,): (Path,) = node_proxy.method_call(DEST_NODE, method, (unit_name, "replace"))?;

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
        let (node,): (Path,) = proxy.method_call(DEST_CONTROLLER, "GetNode", (&node_name,))?;

        let node_proxy = conn.with_proxy(DEST, node, Duration::from_millis(5000));
        node_proxy.method_call::<(), _, _, _>(DEST_NODE, "Reload", ())?;

        result.push_str(&format!("Node - {} is reloaded.\n", &node_name));
    }
    Ok(result)
}

//UNIT TEST
#[cfg(test)]
mod tests {
    use super::*;
    use dbus::blocking::{Connection, Proxy};
    use std::time::Duration;

    /// Dummy Connection (session bus)
    fn dummy_connection() -> Connection {
        Connection::new_session().unwrap()
    }

    /// Check if BlueChi D-Bus service is available (only for tests)
    fn is_bluechi_service_available(conn: &Connection) -> bool {
        let proxy = conn.with_proxy(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_millis(5000),
        );

        let result: core::result::Result<(bool,), Box<dyn std::error::Error>> = proxy
            .method_call("org.freedesktop.DBus", "NameHasOwner", (DEST,))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>);

        match result {
            Ok((has_owner,)) => has_owner,
            Err(_) => false,
        }
    }

    /// Test handle_bluechi_cmd() with ControllerReloadAllNodes command (positive)
    #[tokio::test]
    async fn test_handle_bluechi_cmd_controller_reload() {
        let scenario = "test-scenario";
        let node = "node1";
        let cmd = BluechiCmd {
            command: Command::ControllerReloadAllNodes,
        };

        let result = handle_bluechi_cmd(scenario, node, cmd).await;
        assert!(result.is_ok());
    }

    /// Test handle_bluechi_cmd() with UnitStart command (positive)
    #[tokio::test]
    async fn test_handle_bluechi_cmd_unit_start() {
        let scenario = "test-scenario";
        let node = "node1";
        let cmd = BluechiCmd {
            command: Command::UnitStart,
        };

        let result = handle_bluechi_cmd(scenario, node, cmd).await;
        assert!(result.is_ok());
    }

    /// Test workload_run() with dummy data (positive)
    #[tokio::test]
    async fn test_workload_run_start_unit() {
        let conn = dummy_connection();

        if !is_bluechi_service_available(&conn) {
            println!("Skipping test_workload_run_start_unit — BlueChi service unavailable.");
            return;
        }

        let scenario = "test-scenario";
        let node = "node1";
        let unit_name = "unitA";

        let bluechi_proxy = conn.with_proxy(DEST, PATH, Duration::from_millis(5000));

        let result = workload_run(&conn, "StartUnit", node, &bluechi_proxy, unit_name).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("StartUnit"));
        assert!(output.contains(unit_name));
    }

    /// Test reload_all_nodes() (positive)
    #[tokio::test]
    async fn test_reload_all_nodes() {
        let conn = dummy_connection();

        if !is_bluechi_service_available(&conn) {
            println!("Skipping test_reload_all_nodes — BlueChi service unavailable.");
            return;
        }

        let proxy = conn.with_proxy(DEST, PATH, Duration::from_millis(5000));
        let result = reload_all_nodes(&proxy).await;
        assert!(result.is_ok());
    }

    /// Test Command::to_method_name() for all command variants (sync test)
    #[tokio::test]
    async fn test_command_to_method_name() {
        assert_eq!(Command::UnitStart.to_method_name(), "StartUnit");
        assert_eq!(Command::UnitStop.to_method_name(), "StopUnit");
        assert_eq!(Command::UnitRestart.to_method_name(), "RestartUnit");
        assert_eq!(Command::UnitReload.to_method_name(), "ReloadUnit");

        let unknown_cmd = Command::ControllerReloadAllNodes;
        assert_eq!(unknown_cmd.to_method_name(), "Unknown");
    }

    // ------------------- NEGATIVE TESTS -------------------

    /// Negative: workload_run() with invalid node name
    #[tokio::test]
    async fn test_workload_run_invalid_node() {
        let conn = dummy_connection();

        if !is_bluechi_service_available(&conn) {
            println!("Skipping test_workload_run_invalid_node — BlueChi service unavailable.");
            return;
        }

        let invalid_node = "nonexistent-node";
        let unit_name = "unitA";

        let bluechi_proxy = conn.with_proxy(DEST, PATH, Duration::from_millis(5000));

        let result =
            workload_run(&conn, "StartUnit", invalid_node, &bluechi_proxy, unit_name).await;
        assert!(result.is_err());
    }

    /// Negative: workload_run() with invalid method
    #[tokio::test]
    async fn test_workload_run_invalid_method() {
        let conn = dummy_connection();

        if !is_bluechi_service_available(&conn) {
            println!("Skipping test_workload_run_invalid_method — BlueChi service unavailable.");
            return;
        }

        let node = "node1";
        let unit_name = "unitA";
        let invalid_method = "NonExistentMethod";

        let bluechi_proxy = conn.with_proxy(DEST, PATH, Duration::from_millis(5000));

        let result = workload_run(&conn, invalid_method, node, &bluechi_proxy, unit_name).await;
        assert!(result.is_err());
    }

    /// Negative: reload_all_nodes() with missing service
    #[tokio::test]
    async fn test_reload_all_nodes_service_missing() {
        let conn = dummy_connection();

        // Intentionally use a WRONG DEST to simulate service missing
        let fake_dest = "org.eclipse.fakebluechi";
        let proxy = conn.with_proxy(fake_dest, PATH, Duration::from_millis(5000));

        let result = reload_all_nodes(&proxy).await;
        assert!(result.is_err());
    }
}

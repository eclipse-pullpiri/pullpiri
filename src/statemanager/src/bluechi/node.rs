/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use dbus::blocking::Connection;
use dbus::Path;
use std::error::Error;
use std::time::Duration;

fn node_list_units(node_name: &str) -> Result<String, Box<dyn Error>> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(super::DEST, super::PATH, Duration::from_millis(5000));
    let (node,): (Path,) = proxy.method_call(super::DEST_CONTROLLER, "GetNode", (node_name,))?;
    let node_proxy = conn.with_proxy(super::DEST, node, Duration::from_millis(5000));

    // we are only interested in the first two response values - unit name and description
    let (units,): (Vec<(String, String)>,) =
        node_proxy.method_call(super::DEST_NODE, "ListUnits", ())?;

    let mut result = String::new();
    for (name, description) in units {
        result.push_str(&format!("{} - {}\n", name, description));
    }

    Ok(result)
}

fn node_daemon_reload(node_name: &str) -> Result<String, Box<dyn Error>> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(super::DEST, super::PATH, Duration::from_millis(5000));
    let (node,): (Path,) = proxy.method_call(super::DEST_CONTROLLER, "GetNode", (node_name,))?;
    let node_proxy = conn.with_proxy(super::DEST, node, Duration::from_millis(5000));

    node_proxy.method_call(super::DEST_NODE, "Reload", ())?;

    Ok(format!("reload node '{}'\n", node_name))
}

pub fn handle_cmd(c: Vec<&str>) -> Result<String, Box<dyn Error>> {
    match c[0] {
        "LIST_UNIT" => node_list_units(c[1]),
        "RELOAD_NODE" => node_daemon_reload(c[1]),
        _ => Err("cannot find command".into()),
    }
}

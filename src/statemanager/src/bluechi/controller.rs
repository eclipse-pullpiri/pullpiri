/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use dbus::blocking::Connection;
use dbus::Path;
use std::error::Error;
use std::time::Duration;

fn list_nodes() -> Result<String, Box<dyn Error>> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(super::DEST, super::PATH, Duration::from_millis(5000));
    let (nodes,): (Vec<(String, dbus::Path, String)>,) =
        proxy.method_call(super::DEST_CONTROLLER, "ListNodes", ())?;

    let mut result = String::new();
    for (name, _, status) in nodes {
        result.push_str(&format!("Node: {}, Status: {}\n", name, status));
    }
    Ok(result)
}

fn reload_all_nodes() -> Result<String, Box<dyn Error>> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(super::DEST, super::PATH, Duration::from_millis(5000));
    let (nodes,): (Vec<(String, dbus::Path, String)>,) =
        proxy.method_call(super::DEST_CONTROLLER, "ListNodes", ())?;

    let mut result = String::new();
    for (node_name, _, _) in nodes {
        let (node,): (Path,) =
            proxy.method_call(super::DEST_CONTROLLER, "GetNode", (&node_name,))?;

        let node_proxy = conn.with_proxy(super::DEST, node, Duration::from_millis(5000));
        node_proxy.method_call(super::DEST_NODE, "Reload", ())?;

        result.push_str(&format!("Node - {} is reloaded.\n", &node_name));
    }
    Ok(result)
}

pub fn handle_cmd(c: Vec<&str>) -> Result<String, Box<dyn Error>> {
    match c[0] {
        "LIST_NODE" => list_nodes(),
        "RELOAD_ALL_NODES" => reload_all_nodes(),
        _ => Err("cannot find command".into()),
    }
}

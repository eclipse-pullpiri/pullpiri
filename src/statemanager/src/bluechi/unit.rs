/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use dbus::blocking::Connection;
use dbus::Path;
use std::error::Error;
use std::time::Duration;

fn unit_lifecycle(
    method: &str,
    node_name: &str,
    unit_name: &str,
) -> Result<String, Box<dyn Error>> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(super::DEST, super::PATH, Duration::from_millis(5000));
    let (node,): (Path,) = proxy.method_call(super::DEST_CONTROLLER, "GetNode", (node_name,))?;
    let node_proxy = conn.with_proxy(super::DEST, node, Duration::from_millis(5000));

    let (job_path,): (Path,) =
        node_proxy.method_call(super::DEST_NODE, method, (unit_name, "replace"))?;

    Ok(format!(
        "{method} '{unit_name}' on node '{node_name}': {job_path}\n"
    ))
}

fn unit_enable(node_name: &str, unit_name: &str) -> Result<String, Box<dyn Error>> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(super::DEST, super::PATH, Duration::from_millis(5000));
    let (node,): (Path,) = proxy.method_call(super::DEST_CONTROLLER, "GetNode", (node_name,))?;
    let node_proxy = conn.with_proxy(super::DEST, node, Duration::from_millis(5000));

    let unit_vector = vec![unit_name.to_owned()];
    let (carries_install_info, changes): (bool, Vec<(String, String, String)>) = node_proxy
        .method_call(
            super::DEST_NODE,
            "EnableUnitFiles",
            (unit_vector, false, false),
        )?;

    let mut result: String = match carries_install_info {
        true => "The unit files included enablement information\n".to_string(),
        false => "The unit files did not include any enablement information\n".to_string(),
    };

    for (op_type, file_name, file_dest) in changes {
        if op_type == "symlink" {
            result.push_str(&format!("Created symlink {file_name} -> {file_dest}\n"));
        } else if op_type == "unlink" {
            result.push_str(&format!("Removed '{file_name}'\n"));
        }
    }

    Ok(result)
}

fn unit_disable(node_name: &str, unit_name: &str) -> Result<String, Box<dyn Error>> {
    let conn = Connection::new_system()?;
    let proxy = conn.with_proxy(super::DEST, super::PATH, Duration::from_millis(5000));
    let (node,): (Path,) = proxy.method_call(super::DEST_CONTROLLER, "GetNode", (node_name,))?;
    let node_proxy = conn.with_proxy(super::DEST, node, Duration::from_millis(5000));

    let unit_vector = vec![unit_name.to_owned()];
    let (changes,): (Vec<(String, String, String)>,) =
        node_proxy.method_call(super::DEST_NODE, "DisableUnitFiles", (unit_vector, false))?;

    let mut result = String::new();
    for (op_type, file_name, file_dest) in changes {
        if op_type == "symlink" {
            result.push_str(&format!("Created symlink {file_name} -> {file_dest}\n"));
        } else if op_type == "unlink" {
            result.push_str(&format!("Removed '{file_name}'\n"));
        }
    }
    Ok(result)
}

pub fn handle_cmd(c: Vec<&str>) -> Result<String, Box<dyn Error>> {
    match c[0] {
        "START" => unit_lifecycle("StartUnit", c[1], c[2]),
        "STOP" => unit_lifecycle("StopUnit", c[1], c[2]),
        "RESTART" => unit_lifecycle("RestartUnit", c[1], c[2]),
        "RELOAD" => unit_lifecycle("ReloadUnit", c[1], c[2]),
        "ENABLE" => unit_enable(c[1], c[2]),
        "DISABLE" => unit_disable(c[1], c[2]),
        _ => Err("cannot find command".into()),
    }
}

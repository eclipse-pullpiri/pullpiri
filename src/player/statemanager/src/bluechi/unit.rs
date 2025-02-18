/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::Result;
use dbus::blocking::{Connection, Proxy};
use dbus::Path;

fn unit_run(method: &str, node_proxy: &Proxy<'_, &Connection>, unit_name: &str) -> Result<String> {
    let (job_path,): (Path,) =
        node_proxy.method_call(super::DEST_NODE, method, (unit_name, "replace"))?;

    Ok(format!("{method} '{unit_name}' : {job_path}\n"))
}

fn unit_enable(node_proxy: &Proxy<'_, &Connection>, unit_name: &str) -> Result<String> {
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

fn unit_disable(node_proxy: &Proxy<'_, &Connection>, unit_name: &str) -> Result<String> {
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

pub fn handle(cmd: super::Command, node: &Proxy<'_, &Connection>, unit: &str) -> Result<String> {
    match cmd {
        super::Command::UnitStart => unit_run("StartUnit", node, unit),
        super::Command::UnitStop => unit_run("StopUnit", node, unit),
        super::Command::UnitRestart => unit_run("RestartUnit", node, unit),
        super::Command::UnitReload => unit_run("ReloadUnit", node, unit),
        super::Command::UnitEnable => unit_enable(node, unit),
        super::Command::UnitDisable => unit_disable(node, unit),
        _ => Err("cannot find command".into()),
    }
}

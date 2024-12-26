/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::Result;
use dbus::blocking::{Connection, Proxy};

fn node_list_units(node_proxy: &Proxy<'_, &Connection>) -> Result<String> {
    // we are only interested in the first two response values - unit name and description
    let (units,): (Vec<(String, String)>,) =
        node_proxy.method_call(super::DEST_NODE, "ListUnits", ())?;

    let mut result = String::new();
    for (name, description) in units {
        result.push_str(&format!("{} - {}\n", name, description));
    }

    Ok(result)
}

fn node_daemon_reload(node_proxy: &Proxy<'_, &Connection>) -> Result<String> {
    node_proxy.method_call(super::DEST_NODE, "Reload", ())?;

    Ok(String::from("reload node\n"))
}

pub fn handle(cmd: super::Command, node: &Proxy<'_, &Connection>) -> Result<String> {
    match cmd {
        super::Command::NodeListUnit => node_list_units(node),
        super::Command::NodeReload => node_daemon_reload(node),
        _ => Err("cannot find command".into()),
    }
}

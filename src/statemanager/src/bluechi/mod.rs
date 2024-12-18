/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub mod controller;
pub mod node;
pub mod unit;

use dbus::blocking::{Connection, Proxy};
use dbus::Path;
use std::{collections::HashMap, time::Duration};
use tokio::sync::mpsc::Receiver;

const DEST: &str = "org.eclipse.bluechi";
const PATH: &str = "/org/eclipse/bluechi";
const DEST_CONTROLLER: &str = "org.eclipse.bluechi.Controller";
const DEST_NODE: &str = "org.eclipse.bluechi.Node";

pub async fn handle_bluechi_cmd(mut rx: Receiver<BluechiCmd>) {
    let conn = Connection::new_system().unwrap();
    let proxy = conn.with_proxy(DEST, PATH, Duration::from_millis(5000));

    let mut map_node_proxy: HashMap<String, Proxy<'_, &Connection>> = HashMap::new();
    // host node proxy
    let (node,): (Path,) = proxy
        .method_call(
            DEST_CONTROLLER,
            "GetNode",
            (&common::get_config().host.name,),
        )
        .unwrap();
    map_node_proxy.insert(
        common::get_config().host.name.clone(),
        conn.with_proxy(DEST, node, Duration::from_millis(5000)),
    );
    // guest node proxy
    if let Some(guests) = &common::get_config().guest {
        for guest in guests {
            let (node,): (Path,) = proxy
                .method_call(DEST_CONTROLLER, "GetNode", (&guest.name,))
                .unwrap();
            map_node_proxy.insert(
                guest.name.clone(),
                conn.with_proxy(DEST, node, Duration::from_millis(5000)),
            );
        }
    }

    while let Some(bluechi_cmd) = rx.recv().await {
        match bluechi_cmd.command {
            Command::ControllerListNode | Command::ControllerReloadAllNodes => {
                let _ = controller::handle(bluechi_cmd.command, &proxy);
            }
            Command::NodeListUnit | Command::NodeReload => {
                let node_proxy = map_node_proxy.get(&bluechi_cmd.node.unwrap()).unwrap();
                let _ = node::handle(bluechi_cmd.command, node_proxy);
            }
            Command::UnitStart
            | Command::UnitStop
            | Command::UnitRestart
            | Command::UnitReload
            | Command::UnitEnable
            | Command::UnitDisable => {
                let node_proxy = map_node_proxy.get(&bluechi_cmd.node.unwrap()).unwrap();
                let _ = unit::handle(bluechi_cmd.command, node_proxy, &bluechi_cmd.unit.unwrap());
            }
        }
    }
}

pub struct BluechiCmd {
    pub command: Command,
    pub node: Option<String>,
    pub unit: Option<String>,
}

#[allow(dead_code)]
pub enum Command {
    ControllerListNode,
    ControllerReloadAllNodes,
    NodeListUnit,
    NodeReload,
    UnitStart,
    UnitStop,
    UnitRestart,
    UnitReload,
    UnitEnable,
    UnitDisable,
}

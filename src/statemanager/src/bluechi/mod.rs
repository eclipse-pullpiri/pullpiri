/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub mod controller;
pub mod node;
pub mod unit;

const DEST: &str = "org.eclipse.bluechi";
const PATH: &str = "/org/eclipse/bluechi";
const DEST_CONTROLLER: &str = "org.eclipse.bluechi.Controller";
const DEST_NODE: &str = "org.eclipse.bluechi.Node";

pub async fn send_dbus(cmd: Vec<&str>) -> Result<String, Box<dyn std::error::Error>> {
    println!("recv msg: {:?}\n", cmd);

    match cmd.len() {
        1 => controller::handle_cmd(cmd),
        2 => node::handle_cmd(cmd),
        3 => unit::handle_cmd(cmd),
        _ => Err("support only 1 ~ 3 parameters".into()),
    }
}

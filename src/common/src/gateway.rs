/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub use api::proto::gateway::*;

pub fn open_server() -> String {
    format!("{}:47002", crate::get_config().host.ip)
}

pub fn connect_server() -> String {
    format!("http://{}:47002", crate::get_config().host.ip)
}

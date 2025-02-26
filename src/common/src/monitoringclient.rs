/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

tonic::include_proto!("monitoringclient");

pub fn open_server() -> String {
    format!("{}:47003", crate::get_config().host.ip)
}

pub fn connect_server() -> String {
    format!("http://{}:47003", crate::get_config().host.ip)
}

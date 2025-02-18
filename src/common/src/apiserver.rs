/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

tonic::include_proto!("apiserver");

pub mod metric_notifier {
    tonic::include_proto!("apiserver.metric_notifier");
}

pub fn open_server() -> String {
    format!("{}:47001", crate::get_config().host.ip)
}

pub fn open_rest_server() -> String {
    format!("{}:47099", crate::get_config().host.ip)
}

pub fn connect_server() -> String {
    format!("http://{}:47001", crate::get_config().host.ip)
}

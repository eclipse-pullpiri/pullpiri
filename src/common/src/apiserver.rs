/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub fn open_rest_server() -> String {
    format!("{}:47099", crate::get_config().host.ip)
}

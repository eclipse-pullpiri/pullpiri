/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

#![allow(non_snake_case)]

pub mod k8s;
pub mod package;
pub mod scenario;

use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct MetaData {
    name: String,
    labels: Option<HashMap<String, String>>,
    annotations: Option<HashMap<String, String>>,
}

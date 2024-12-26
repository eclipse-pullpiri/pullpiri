// SPDX-License-Identifier: Apache-2.0

// use crate::spec::workload::podspec;

use super::MetaData;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Network {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: Option<NetworkSpec>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NetworkSpec {
    dummy: Option<String>,
}

impl Network {
    pub fn get_spec(&self) -> &Option<NetworkSpec> {
        &self.spec
    }
}

impl NetworkSpec {
    pub fn get_network(&self) -> &Option<String> {
        &self.dummy
    }
}

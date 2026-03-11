/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use super::Artifact;
use super::Network;

impl Artifact for Network {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Network {
    pub fn get_spec(&self) -> &NetworkSpec {
        &self.spec
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NetworkSpec {
    #[serde(rename = "networkName")]
    network_name: String,

    #[serde(rename = "networkMode")]
    network_mode: NetworkMode,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    Bridge,
    Host,
    Macvlan,
    Ipvlan,
    Overlay,
}

impl NetworkSpec {
    pub fn get_network_name(&self) -> &str {
        &self.network_name
    }

    pub fn get_network_mode(&self) -> &NetworkMode {
        &self.network_mode
    }
}

impl NetworkMode {
    pub fn as_str(&self) -> &str {
        match self {
            NetworkMode::Bridge => "bridge",
            NetworkMode::Host => "host",
            NetworkMode::Macvlan => "macvlan",
            NetworkMode::Ipvlan => "ipvlan",
            NetworkMode::Overlay => "overlay",
        }
    }
}

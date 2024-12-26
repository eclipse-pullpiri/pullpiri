// SPDX-License-Identifier: Apache-2.0

use crate::spec::k8s::pod;
use crate::spec::MetaData;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Volume {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: Option<VolumeSpec>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VolumeSpec {
    volumes: Option<Vec<pod::Volume>>,
}

impl Volume {
    pub fn get_spec(&self) -> &Option<VolumeSpec> {
        &self.spec
    }
}

impl VolumeSpec {
    pub fn get_volume(&self) -> &Option<Vec<pod::Volume>> {
        &self.volumes
    }
}

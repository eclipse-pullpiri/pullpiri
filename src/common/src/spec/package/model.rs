// SPDX-License-Identifier: Apache-2.0

use crate::spec::k8s::pod::PodSpec;
use crate::spec::MetaData;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Model {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: PodSpec,
}

impl Model {
    pub fn get_name(&self) -> String {
        self.metadata.name.clone()
    }

    pub fn get_podspec(&self) -> PodSpec {
        self.spec.clone()
    }
}

// SPDX-License-Identifier: Apache-2.0

pub mod pod;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Pod {
    apiVersion: String,
    kind: String,
    metadata: super::MetaData,
    spec: pod::PodSpec,
}

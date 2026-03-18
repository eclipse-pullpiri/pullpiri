// SPDX-License-Identifier: Apache-2.0

pub mod pod;

pub use pod::{PodNetwork, PodSpec, Volume, VolumeMount};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Pod {
    apiVersion: String,
    kind: String,
    metadata: super::MetaData,
    pub spec: pod::PodSpec,
}
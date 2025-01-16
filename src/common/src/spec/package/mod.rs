// SPDX-License-Identifier: Apache-2.0

pub mod model;
pub mod network;
pub mod volume;

use super::MetaData;

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct Package {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: PackageSpec,
    status: Option<PackageStatus>,
}

impl Package {
    pub fn get_models(&self) -> &Vec<Model> {
        &self.spec.models
    }
    pub fn get_model_name(&self) -> Vec<String> {
        let mut ret: Vec<String> = Vec::new();
        for m in &self.spec.models {
            ret.push(m.name.clone());
        }
        ret
    }

    pub fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct PackageSpec {
    pattern: Vec<Pattern>,
    models: Vec<Model>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Pattern {
    r#type: String,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct Model {
    name: String,
    node: String,
    resources: Resource,
}

impl Model {
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_node(&self) -> String {
        self.node.clone()
    }

    pub fn get_resources(&self) -> Resource {
        self.resources.clone()
    }
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct Resource {
    volume: Option<String>,
    network: Option<String>,
}

impl Resource {
    pub fn get_volume(&self) -> Option<String> {
        self.volume.clone()
    }
    pub fn get_network(&self) -> Option<String> {
        self.network.clone()
    }
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct PackageStatus {
    model: Vec<ModelStatus>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct ModelStatus {
    name: String,
    state: ModelStatusState,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
enum ModelStatusState {
    None,
    Running,
    Error,
}

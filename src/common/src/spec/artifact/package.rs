use super::Artifact;
use super::Package;

impl Artifact for Package {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Package {
    pub fn get_models(&self) -> &Vec<ModelInfo> {
        &self.spec.models
    }
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct PackageSpec {
    pattern: Vec<Pattern>,
    models: Vec<ModelInfo>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Pattern {
    r#type: String,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct ModelInfo {
    name: String,
    node: String,
    resources: Resource,
}

impl ModelInfo {
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
    status: Vec<ModelStatus>,
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

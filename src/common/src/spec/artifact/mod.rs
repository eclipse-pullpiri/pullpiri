mod model;
mod network;
mod package;
mod scenario;
mod volume;

use serde::{Deserialize, Serialize};
use super::MetaData;

pub trait Artifact {
    fn get_name(&self) -> String;
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Scenario {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: scenario::ScenarioSpec,
    status: Option<scenario::ScenarioStatus>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Package {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: package::PackageSpec,
    status: Option<package::PackageStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Volume {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: Option<volume::VolumeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Network {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: Option<network::NetworkSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Model {
    apiVersion: String,
    kind: String,
    metadata: MetaData,
    spec: model::ModelSpec,
}

// SPDX-License-Identifier: Apache-2.0

use super::Pod;
use crate::spec::artifact::Model;
use crate::spec::MetaData;

impl Pod {
    pub fn new(name: &str, podspec: PodSpec) -> Pod {
        Pod {
            apiVersion: String::from("v1"),
            kind: String::from("Pod"),
            metadata: MetaData {
                name: name.to_string(),
                labels: None,
                annotations: None,
            },
            spec: podspec,
        }
    }

    pub fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl From<Model> for Pod {
    fn from(model: Model) -> Self {
        Pod::new(&model.get_name(), model.get_podspec())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PodSpec {
    hostNetwork: Option<bool>,
    containers: Vec<Container>,
    pub volumes: Option<Vec<Volume>>,
    initContainers: Option<Vec<Container>>,
    restartPolicy: Option<String>,
    terminationGracePeriodSeconds: Option<i32>,
    hostIpc: Option<bool>,
    runtimeClassName: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Container {
    name: String,
    image: String,
    volumeMounts: Option<Vec<VolumeMount>>,
    env: Option<Vec<Env>>,
    ports: Option<Vec<Port>>,
    command: Option<Vec<String>>,
    workingDir: Option<String>,
    resources: Option<Resources>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Volume {
    name: String,
    hostPath: HostPath,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HostPath {
    path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VolumeMount {
    name: String,
    mountPath: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Env {
    name: String,
    value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Port {
    containerPort: Option<i32>,
    hostPort: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Resources {
    requests: Option<Requests>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Requests {
    cpu: Option<String>,
    memory: Option<String>,
}

impl PodSpec {
    pub fn get_image(&self) -> &str {
        &self.containers[0].image
    }

    pub fn get_volume(&mut self) -> &Option<Vec<Volume>> {
        &self.volumes
    }
}

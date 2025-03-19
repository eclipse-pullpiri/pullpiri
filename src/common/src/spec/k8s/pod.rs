// SPDX-License-Identifier: Apache-2.0

use super::Pod;
use crate::spec::MetaData;

impl Pod {
    pub fn new(name: &str, podspec: PodSpec) -> Pod {
        Pod {
            apiVersion: "v1".to_string(),
            kind: "Pod".to_string(),
            metadata: MetaData {
                name: name.to_string(),
                labels: None,
                annotations: None,
            },
            spec: podspec,
        }
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

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
    securityContext: Option<SecurityContext>,
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
    limits: Option<Limits>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Requests {
    cpu: Option<String>,
    memory: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Limits {
    cpu: Option<String>,
    memory: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct SecurityContext {
    privileged: Option<bool>,
    capabilities: Option<Capabilities>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Capabilities {
    add: Option<Vec<String>>,
    drop: Option<Vec<String>>,
}

impl PodSpec {
    /// Returns the image of the first container in the PodSpec.
    /// If no containers are present, returns `None`.
    pub fn get_image(&self) -> Option<&str> {
        self.containers
            .get(0)
            .map(|container| container.image.as_str())
    }

    pub fn get_volume(&mut self) -> &Option<Vec<Volume>> {
        &self.volumes
    }
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;

    // Positive Test: Validate that `get_image` returns the image of the first container
    // when multiple containers are present in the PodSpec.
    #[tokio::test]
    async fn test_get_image_with_multiple_containers() {
        let container1 = Container {
            name: String::from("container-1"),
            image: String::from("image-1"),
            volumeMounts: None,
            env: None,
            ports: None,
            command: None,
            workingDir: None,
            resources: None,
            securityContext: None,
        };
        let container2 = Container {
            name: String::from("container-2"),
            image: String::from("image-2"),
            volumeMounts: None,
            env: None,
            ports: None,
            command: None,
            workingDir: None,
            resources: None,
            securityContext: None,
        };
        let podspec = PodSpec {
            hostNetwork: None,
            containers: vec![container1, container2],
            volumes: None,
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(podspec.get_image(), Some("image-1"));
    }

    // Negative Test: Validate that `get_image` returns `None` when no containers are present.
    #[tokio::test]
    async fn test_get_image_with_no_containers() {
        let podspec = PodSpec {
            hostNetwork: None,
            containers: vec![],
            volumes: None,
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(podspec.get_image(), None);
    }

    // Negative Test: Validate that `get_image` correctly handles containers with an empty
    // image field and returns an empty string.
    #[tokio::test]
    async fn test_get_image_with_null_image_field() {
        let container = Container {
            name: String::from("test-container"),
            image: String::from(""),
            volumeMounts: None,
            env: None,
            ports: None,
            command: None,
            workingDir: None,
            resources: None,
            securityContext: None,
        };
        let podspec = PodSpec {
            hostNetwork: None,
            containers: vec![container],
            volumes: None,
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(podspec.get_image(), Some(""));
    }

    // Positive Test: Validate that `get_volume` correctly returns all volumes when
    // multiple volumes are present in the PodSpec.
    #[tokio::test]
    async fn test_get_volume_with_multiple_volumes() {
        let volume1 = Volume {
            name: String::from("volume-1"),
            hostPath: HostPath {
                path: String::from("/path/1"),
            },
        };
        let volume2 = Volume {
            name: String::from("volume-2"),
            hostPath: HostPath {
                path: String::from("/path/2"),
            },
        };
        let mut podspec = PodSpec {
            hostNetwork: None,
            containers: vec![],
            volumes: Some(vec![volume1, volume2]),
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(
            podspec.get_volume(),
            &Some(vec![
                Volume {
                    name: String::from("volume-1"),
                    hostPath: HostPath {
                        path: String::from("/path/1"),
                    },
                },
                Volume {
                    name: String::from("volume-2"),
                    hostPath: HostPath {
                        path: String::from("/path/2"),
                    },
                },
            ])
        );
    }

    // Negative Test: Validate that `get_volume` returns `None` when no volumes are present.
    #[tokio::test]
    async fn test_get_volume_with_no_volumes() {
        let mut podspec = PodSpec {
            hostNetwork: None,
            containers: vec![],
            volumes: None,
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(podspec.get_volume(), &None);
    }

    // Negative Test: Validate that `get_volume` correctly handles an empty volume list.
    #[tokio::test]
    async fn test_get_volume_with_empty_volume_list() {
        let mut podspec = PodSpec {
            hostNetwork: None,
            containers: vec![],
            volumes: Some(vec![]),
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(podspec.get_volume(), &Some(vec![]));
    }

    // Negative Test: Validate that `get_volume` correctly handles invalid volume data.
    #[tokio::test]
    async fn test_get_volume_with_invalid_volume() {
        let volume = Volume {
            name: String::from(""),
            hostPath: HostPath {
                path: String::from(""),
            },
        };
        let mut podspec = PodSpec {
            hostNetwork: None,
            containers: vec![],
            volumes: Some(vec![volume]),
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(
            podspec.get_volume(),
            &Some(vec![Volume {
                name: String::from(""),
                hostPath: HostPath {
                    path: String::from(""),
                },
            }])
        );
    }

    // Positive Test: Validate that `get_image` correctly handles container image names
    // with special characters such as colons and tags.
    #[tokio::test]
    async fn test_get_image_with_special_characters_in_image_name() {
        let container = Container {
            name: String::from("test-container"),
            image: String::from("special:image@tag"),
            volumeMounts: None,
            env: None,
            ports: None,
            command: None,
            workingDir: None,
            resources: None,
            securityContext: None,
        };
        let podspec = PodSpec {
            hostNetwork: None,
            containers: vec![container],
            volumes: None,
            initContainers: None,
            restartPolicy: None,
            terminationGracePeriodSeconds: None,
            hostIpc: None,
            runtimeClassName: None,
        };
        assert_eq!(podspec.get_image(), Some("special:image@tag"));
    }
}

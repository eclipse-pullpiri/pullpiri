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

    /// Returns the restart policy of the pod spec, if set.
    pub fn get_restart_policy(&self) -> Option<&str> {
        self.spec.restartPolicy.as_deref()
    }

    /// Returns the probe configuration of the pod spec, if set.
    pub fn get_probe_config(&self) -> Option<&ProbeConfig> {
        self.spec.probeConfig.as_ref()
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
    pub containers: Vec<Container>,
    pub networks: Option<Vec<PodNetwork>>,
    pub volumes: Option<Vec<Volume>>,
    initContainers: Option<Vec<Container>>,
    restartPolicy: Option<String>,
    terminationGracePeriodSeconds: Option<i32>,
    hostIPC: Option<bool>,
    runtimeClassName: Option<String>,
    securityContext: Option<PodSecurityContext>,
    pub probeConfig: Option<ProbeConfig>,
}

/// Configuration for health probes in the Pod YAML spec.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ProbeConfig {
    pub liveness: Option<LivenessProbeSpec>,
}

/// Default probe period: 10 seconds (aligns with Kubernetes convention).
fn default_period_seconds() -> u32 {
    10
}
/// Default probe timeout: 1 second (suitable for fast local health-check endpoints).
fn default_timeout_seconds() -> u32 {
    1
}
/// Default failure threshold: 3 consecutive failures before marking unhealthy
/// (aligns with Kubernetes convention and provides a small buffer against transient failures).
fn default_failure_threshold() -> u8 {
    3
}

/// Liveness probe configuration as specified in Pod YAML.
#[allow(non_snake_case)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct LivenessProbeSpec {
    pub http: Option<HttpProbeSpec>,
    pub tcp: Option<TcpProbeSpec>,
    pub exec: Option<ExecProbeSpec>,
    /// Seconds to wait before the first probe after container start (default: 0).
    #[serde(default)]
    pub initialDelaySeconds: u32,
    /// How often (in seconds) to probe (default: 10).
    #[serde(default = "default_period_seconds")]
    pub periodSeconds: u32,
    /// Seconds after which each probe attempt times out (default: 1).
    #[serde(default = "default_timeout_seconds")]
    pub timeoutSeconds: u32,
    /// Minimum consecutive failures to mark the container as unhealthy (default: 3).
    #[serde(default = "default_failure_threshold")]
    pub failureThreshold: u8,
}

/// HTTP GET probe configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HttpProbeSpec {
    pub path: String,
    pub port: u16,
}

/// TCP socket probe configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TcpProbeSpec {
    pub port: u16,
}

/// Command execution probe configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ExecProbeSpec {
    pub command: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Container {
    name: String,
    image: String,
    volumeMounts: Option<Vec<VolumeMount>>,
    env: Option<Vec<Env>>,
    ports: Option<Vec<Port>>,
    pub command: Option<Vec<String>>,
    workingDir: Option<String>,
    resources: Option<Resources>,
    securityContext: Option<SecurityContext>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PodSecurityContext {
    runAsUser: Option<i64>,
    runAsGroup: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Volume {
    pub name: String,
    pub hostPath: Option<HostPath>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PodNetwork {
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HostPath {
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct VolumeMount {
    pub name: String,
    pub mountPath: String,
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
    runAsUser: Option<i64>,
    runAsGroup: Option<i64>,
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
            .first()
            .map(|container| container.image.as_str())
    }

    pub fn get_volume(&mut self) -> &Option<Vec<Volume>> {
        &self.volumes
    }

    /// Adds a network to the pod spec
    pub fn add_network(&mut self, network_name: &str) {
        let network = PodNetwork {
            name: network_name.to_string(),
        };
        match &mut self.networks {
            Some(networks) => networks.push(network),
            None => self.networks = Some(vec![network]),
        }
    }

    /// Adds a volume to the pod spec
    pub fn add_volume(&mut self, volume: Volume) {
        match &mut self.volumes {
            Some(volumes) => volumes.push(volume),
            None => self.volumes = Some(vec![volume]),
        }
    }

    /// Adds a volume mount to all containers in the pod
    pub fn add_volume_mount(&mut self, volume_name: &str, mount_path: &str) {
        let volume_mount = VolumeMount {
            name: volume_name.to_string(),
            mountPath: mount_path.to_string(),
        };

        for container in &mut self.containers {
            match &mut container.volumeMounts {
                Some(mounts) => mounts.push(volume_mount.clone()),
                None => container.volumeMounts = Some(vec![volume_mount.clone()]),
            }
        }
    }

    /// Adds a volume mount to the first container
    pub fn add_volume_mount_to_first_container(&mut self, volume_mount: VolumeMount) {
        if let Some(container) = self.containers.first_mut() {
            match &mut container.volumeMounts {
                Some(mounts) => mounts.push(volume_mount),
                None => container.volumeMounts = Some(vec![volume_mount]),
            }
        }
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
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
            hostIPC: None,
            runtimeClassName: None,
            securityContext: None,
            probeConfig: None,
        };
        assert_eq!(podspec.get_image(), Some("special:image@tag"));
    }

    // Test: probeConfig with all timing fields omitted uses sensible defaults.
    #[test]
    fn test_liveness_probe_spec_defaults_when_fields_omitted() {
        let yaml = r#"
http:
  path: /healthz
  port: 8080
"#;
        let spec: LivenessProbeSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.initialDelaySeconds, 0);
        assert_eq!(spec.periodSeconds, 10);
        assert_eq!(spec.timeoutSeconds, 1);
        assert_eq!(spec.failureThreshold, 3);
        assert!(spec.http.is_some());
    }

    // Test: probeConfig with explicit values overrides defaults.
    #[test]
    fn test_liveness_probe_spec_explicit_values_override_defaults() {
        let yaml = r#"
http:
  path: /ready
  port: 9090
initialDelaySeconds: 15
periodSeconds: 30
timeoutSeconds: 5
failureThreshold: 5
"#;
        let spec: LivenessProbeSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.initialDelaySeconds, 15);
        assert_eq!(spec.periodSeconds, 30);
        assert_eq!(spec.timeoutSeconds, 5);
        assert_eq!(spec.failureThreshold, 5);
    }

    // Test: full Pod YAML with probeConfig using only minimal fields parses correctly.
    #[test]
    fn test_pod_yaml_with_partial_probe_config_uses_defaults() {
        let yaml = r#"
apiVersion: v1
kind: Pod
metadata:
  name: partial-probe-pod
spec:
  containers:
    - name: app
      image: myapp:latest
  probeConfig:
    liveness:
      tcp:
        port: 8080
"#;
        let pod = serde_yaml::from_str::<crate::spec::k8s::Pod>(yaml).unwrap();
        let probe_config = pod.get_probe_config().unwrap();
        let liveness = probe_config.liveness.as_ref().unwrap();
        assert_eq!(liveness.initialDelaySeconds, 0);
        assert_eq!(liveness.periodSeconds, 10);
        assert_eq!(liveness.timeoutSeconds, 1);
        assert_eq!(liveness.failureThreshold, 3);
        assert!(liveness.tcp.is_some());
        assert_eq!(liveness.tcp.as_ref().unwrap().port, 8080);
    }
}

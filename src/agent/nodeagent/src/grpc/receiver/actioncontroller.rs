/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
use crate::desired_state::{DesiredState, LivenessProbe, ProbeConfig, ProbeType, RestartPolicy};
use crate::runtime::podman::resource;
use common::nodeagent::fromactioncontroller::{
    HandleWorkloadRequest, HandleWorkloadResponse, WorkloadCommand,
};
use common::nodeagent::{
    GetResourceStatusRequest, GetResourceStatusResponse, UpdateResourcesRequest,
    UpdateResourcesResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

/// Extract pod name from pod YAML string.
#[cfg(test)]
fn extract_pod_name(pod_yaml: &str) -> Result<String, Box<dyn std::error::Error>> {
    let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(pod_yaml)?;
    Ok(pod.get_name())
}

/// Convert a common crate ProbeConfig into a nodeagent DesiredState ProbeConfig.
///
/// Returns `None` if the liveness probe configuration has no recognized probe type
/// (i.e., no `http`, `tcp`, or `exec` field), as this would be a configuration error.
fn convert_probe_config(common_probe: &common::spec::k8s::pod::ProbeConfig) -> Option<ProbeConfig> {
    let liveness = common_probe.liveness.as_ref().and_then(|lp| {
        let probe_type = if let Some(http) = &lp.http {
            ProbeType::Http {
                path: http.path.clone(),
                port: http.port,
            }
        } else if let Some(tcp) = &lp.tcp {
            ProbeType::Tcp { port: tcp.port }
        } else if let Some(exec) = &lp.exec {
            ProbeType::Exec {
                command: exec.command.clone(),
            }
        } else {
            eprintln!(
                "[NodeAgent] Liveness probe configuration has no probe type (http/tcp/exec); ignoring"
            );
            return None;
        };

        Some(LivenessProbe {
            probe_type,
            initial_delay_seconds: lp.initialDelaySeconds,
            period_seconds: lp.periodSeconds,
            timeout_seconds: lp.timeoutSeconds,
            failure_threshold: lp.failureThreshold,
        })
    });

    Some(ProbeConfig { liveness })
}

pub async fn handle_workload(
    request: Request<HandleWorkloadRequest>,
    desired_states_cache: Arc<Mutex<HashMap<String, DesiredState>>>,
) -> Result<Response<HandleWorkloadResponse>, Status> {
    let req = request.into_inner();
    let pod_yaml = req.pod.clone();
    let command = req.workload_command;

    // Parse full pod YAML to extract name, restartPolicy, and probeConfig
    let pod = match serde_yaml::from_str::<common::spec::k8s::Pod>(&pod_yaml) {
        Ok(p) => p,
        Err(e) => {
            return Err(Status::invalid_argument(format!(
                "Failed to parse pod YAML: {}",
                e
            )));
        }
    };

    let pod_name = pod.get_name();

    if command == WorkloadCommand::Start as i32 {
        // Build DesiredState with restart policy and probe config from YAML
        let mut desired_state = DesiredState::new(pod_name.clone());

        // Parse restart policy from pod spec
        desired_state.restart_policy = match pod.get_restart_policy() {
            Some("Always") => RestartPolicy::Always,
            Some("OnFailure") => RestartPolicy::OnFailure,
            Some("Never") => RestartPolicy::Never,
            _ => RestartPolicy::OnFailure,
        };

        // Parse probe config from pod spec
        desired_state.probe_config = pod.get_probe_config().and_then(convert_probe_config);

        // Store the pod YAML so the reconciliation loop can recreate the container
        // if it is completely removed from Podman.
        desired_state.pod_yaml = pod_yaml.clone();

        // Insert into memory cache before starting the container
        {
            let mut cache = desired_states_cache.lock().await;
            cache.insert(pod_name.clone(), desired_state);
        }

        // Start the container via Podman API and convert any error to String immediately
        // to avoid holding Box<dyn Error> (not Send) across the subsequent await points.
        let start_result = crate::runtime::podman::handle_workload(command, &pod_yaml)
            .await
            .map_err(|e| e.to_string());

        match start_result {
            Ok(container_ids) => {
                // Update cache entry with the Podman container ID
                if let Some(first_id) = container_ids.into_iter().next() {
                    let mut cache = desired_states_cache.lock().await;
                    if let Some(state) = cache.get_mut(&pod_name) {
                        state.container_id = first_id;
                    }
                }
                println!(
                    "Workload started and desired state cached for: {}",
                    pod_name
                );
                Ok(Response::new(HandleWorkloadResponse {
                    status: true,
                    desc: format!(
                        "Container started and desired state cached for {}",
                        pod_name
                    ),
                }))
            }
            Err(err_msg) => {
                // Remove from cache on container start failure
                let mut cache = desired_states_cache.lock().await;
                cache.remove(&pod_name);
                println!(
                    "Failed to start container for {}, removed from cache: {:?}",
                    pod_name, err_msg
                );
                Err(Status::internal(format!(
                    "Failed to start container: {}",
                    err_msg
                )))
            }
        }
    } else if command == WorkloadCommand::Stop as i32 || command == WorkloadCommand::Remove as i32 {
        // Remove from memory cache before stopping
        {
            let mut cache = desired_states_cache.lock().await;
            cache.remove(&pod_name);
        }
        println!("Removed desired state from cache for: {}", pod_name);

        // Stop/remove the container via Podman API
        match crate::runtime::podman::handle_workload(command, &pod_yaml).await {
            Ok(_) => Ok(Response::new(HandleWorkloadResponse {
                status: true,
                desc: format!(
                    "Container stopped and desired state removed for {}",
                    pod_name
                ),
            })),
            Err(e) => Err(Status::internal(format!("Failed to stop container: {}", e))),
        }
    } else {
        // For other commands (Restart, Pause, Unpause, etc.), forward to Podman without cache changes
        match crate::runtime::podman::handle_workload(command, &pod_yaml).await {
            Ok(_) => {
                println!("Workload command {} executed for: {}", command, pod_name);
                Ok(Response::new(HandleWorkloadResponse {
                    status: true,
                    desc: format!("Workload command executed for {}", pod_name),
                }))
            }
            Err(e) => Err(Status::unimplemented(format!(
                "handle_workload is not implemented yet: {}",
                e
            ))),
        }
    }
}

/// Handle an `UpdateResources` request from the ActionController.
///
/// Applies the requested CPU / memory limits to the running container through
/// the Podman REST API and returns the actual runtime state read back after the
/// update (Dynamic Resource Scaling, #514 / #526).
pub async fn update_resources(
    request: Request<UpdateResourcesRequest>,
) -> Result<Response<UpdateResourcesResponse>, Status> {
    let req = request.into_inner();
    println!(
        "[NodeAgent] UpdateResources workload='{}' cpu={:?} mem={:?}MiB",
        req.workload_id, req.cpu_limit, req.memory_limit
    );

    match resource::update_resources(&req.workload_id, req.cpu_limit, req.memory_limit).await {
        Ok(status) => Ok(Response::new(UpdateResourcesResponse {
            success: true,
            message: "resource update applied".to_string(),
            actual_cpu_limit: status.cpu_limit,
            actual_memory_limit: status.memory_limit,
        })),
        Err(e) => Ok(Response::new(UpdateResourcesResponse {
            success: false,
            message: format!("resource update failed: {}", e),
            actual_cpu_limit: 0,
            actual_memory_limit: 0,
        })),
    }
}

/// Handle a `GetResourceStatus` request: report the actual runtime CPU / memory
/// limits currently applied to the container.
pub async fn get_resource_status(
    request: Request<GetResourceStatusRequest>,
) -> Result<Response<GetResourceStatusResponse>, Status> {
    let req = request.into_inner();
    match resource::get_resource_status(&req.workload_id).await {
        Ok(status) => Ok(Response::new(GetResourceStatusResponse {
            found: true,
            cpu_limit: status.cpu_limit,
            memory_limit: status.memory_limit,
            message: "ok".to_string(),
        })),
        Err(e) => Ok(Response::new(GetResourceStatusResponse {
            found: false,
            cpu_limit: 0,
            memory_limit: 0,
            message: format!("resource status query failed: {}", e),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::nodeagent::fromactioncontroller::WorkloadCommand;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    fn make_cache() -> Arc<Mutex<HashMap<String, DesiredState>>> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    const VALID_POD_YAML: &str = r#"
apiVersion: v1
kind: Pod
metadata:
  name: test-pod
spec:
  containers:
    - name: test-container
      image: nginx:latest
"#;

    #[test]
    fn test_extract_pod_name_valid() {
        let result = extract_pod_name(VALID_POD_YAML);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-pod");
    }

    #[test]
    fn test_extract_pod_name_invalid_yaml() {
        let result = extract_pod_name("not: valid: yaml: [");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_pod_name_empty() {
        let result = extract_pod_name("");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_workload_invalid_yaml_returns_error() {
        let cache = make_cache();
        let request = tonic::Request::new(HandleWorkloadRequest {
            workload_command: WorkloadCommand::Start as i32,
            pod: "invalid yaml [[[".to_string(),
        });

        let result = handle_workload(request, cache).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_handle_workload_stop_removes_from_cache() {
        let cache = make_cache();

        // Pre-populate cache with a desired state
        {
            let mut c = cache.lock().await;
            c.insert(
                "test-pod".to_string(),
                DesiredState::new("test-pod".to_string()),
            );
        }

        // Verify entry exists
        assert_eq!(cache.lock().await.len(), 1);

        // Send STOP command (will fail at podman level since no podman, but cache should be cleared)
        let request = tonic::Request::new(HandleWorkloadRequest {
            workload_command: WorkloadCommand::Stop as i32,
            pod: VALID_POD_YAML.to_string(),
        });

        let _ = handle_workload(request, Arc::clone(&cache)).await;

        // Cache entry should be removed regardless of podman result
        assert_eq!(cache.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_workload_remove_clears_from_cache() {
        let cache = make_cache();

        // Pre-populate cache
        {
            let mut c = cache.lock().await;
            c.insert(
                "test-pod".to_string(),
                DesiredState::new("test-pod".to_string()),
            );
        }

        let request = tonic::Request::new(HandleWorkloadRequest {
            workload_command: WorkloadCommand::Remove as i32,
            pod: VALID_POD_YAML.to_string(),
        });

        // Even if podman fails, the cache should be cleared
        let _ = handle_workload(request, Arc::clone(&cache)).await;
        assert_eq!(cache.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_workload_start_clears_cache_on_podman_failure() {
        let cache = make_cache();

        // START command will fail because podman is not running
        let request = tonic::Request::new(HandleWorkloadRequest {
            workload_command: WorkloadCommand::Start as i32,
            pod: VALID_POD_YAML.to_string(),
        });

        let result = handle_workload(request, Arc::clone(&cache)).await;

        // Should return an error
        assert!(result.is_err());
        // Cache should be empty (cleaned up after failure)
        assert_eq!(cache.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_workload_stop_missing_from_cache_is_noop() {
        let cache = make_cache();
        // Cache is empty - stopping should still attempt podman stop

        let request = tonic::Request::new(HandleWorkloadRequest {
            workload_command: WorkloadCommand::Stop as i32,
            pod: VALID_POD_YAML.to_string(),
        });

        // Should not panic even if pod is not in cache
        let _ = handle_workload(request, Arc::clone(&cache)).await;
        assert_eq!(cache.lock().await.len(), 0);
    }

    #[test]
    fn test_convert_probe_config_http() {
        use common::spec::k8s::pod::{HttpProbeSpec, LivenessProbeSpec, ProbeConfig};
        let common_probe = ProbeConfig {
            liveness: Some(LivenessProbeSpec {
                http: Some(HttpProbeSpec {
                    path: "/healthz".to_string(),
                    port: 8080,
                }),
                tcp: None,
                exec: None,
                initialDelaySeconds: 5,
                periodSeconds: 10,
                timeoutSeconds: 3,
                failureThreshold: 3,
            }),
        };

        let result = convert_probe_config(&common_probe);
        assert!(result.is_some());
        let probe_config = result.unwrap();
        assert!(probe_config.liveness.is_some());
        let liveness = probe_config.liveness.unwrap();
        assert_eq!(liveness.initial_delay_seconds, 5);
        assert_eq!(liveness.period_seconds, 10);
        assert_eq!(liveness.timeout_seconds, 3);
        assert_eq!(liveness.failure_threshold, 3);
        if let ProbeType::Http { path, port } = liveness.probe_type {
            assert_eq!(path, "/healthz");
            assert_eq!(port, 8080);
        } else {
            panic!("Expected Http probe type");
        }
    }

    #[test]
    fn test_convert_probe_config_tcp() {
        use common::spec::k8s::pod::{LivenessProbeSpec, ProbeConfig, TcpProbeSpec};
        let common_probe = ProbeConfig {
            liveness: Some(LivenessProbeSpec {
                http: None,
                tcp: Some(TcpProbeSpec { port: 6379 }),
                exec: None,
                initialDelaySeconds: 0,
                periodSeconds: 5,
                timeoutSeconds: 1,
                failureThreshold: 3,
            }),
        };

        let result = convert_probe_config(&common_probe);
        assert!(result.is_some());
        let liveness = result.unwrap().liveness.unwrap();
        if let ProbeType::Tcp { port } = liveness.probe_type {
            assert_eq!(port, 6379);
        } else {
            panic!("Expected Tcp probe type");
        }
    }

    #[test]
    fn test_convert_probe_config_exec() {
        use common::spec::k8s::pod::{ExecProbeSpec, LivenessProbeSpec, ProbeConfig};
        let common_probe = ProbeConfig {
            liveness: Some(LivenessProbeSpec {
                http: None,
                tcp: None,
                exec: Some(ExecProbeSpec {
                    command: vec!["cat".to_string(), "/tmp/healthy".to_string()],
                }),
                initialDelaySeconds: 0,
                periodSeconds: 10,
                timeoutSeconds: 5,
                failureThreshold: 3,
            }),
        };

        let result = convert_probe_config(&common_probe);
        assert!(result.is_some());
        let liveness = result.unwrap().liveness.unwrap();
        if let ProbeType::Exec { command } = liveness.probe_type {
            assert_eq!(command, vec!["cat", "/tmp/healthy"]);
        } else {
            panic!("Expected Exec probe type");
        }
    }

    #[test]
    fn test_convert_probe_config_no_liveness() {
        use common::spec::k8s::pod::ProbeConfig;
        let common_probe = ProbeConfig { liveness: None };
        let result = convert_probe_config(&common_probe);
        assert!(result.is_some());
        assert!(result.unwrap().liveness.is_none());
    }

    #[test]
    fn test_pod_yaml_with_probe_config_parses_correctly() {
        let yaml = r#"
apiVersion: v1
kind: Pod
metadata:
  name: nginx-probe
spec:
  containers:
    - name: nginx
      image: nginx:latest
  probeConfig:
    liveness:
      http:
        path: /
        port: 80
      initialDelaySeconds: 5
      periodSeconds: 10
      timeoutSeconds: 3
      failureThreshold: 3
"#;
        let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(yaml).unwrap();
        assert_eq!(pod.get_name(), "nginx-probe");
        let probe_config = pod.get_probe_config();
        assert!(probe_config.is_some());
        let liveness = probe_config.unwrap().liveness.as_ref().unwrap();
        assert!(liveness.http.is_some());
        assert_eq!(liveness.http.as_ref().unwrap().path, "/");
        assert_eq!(liveness.http.as_ref().unwrap().port, 80);
        assert_eq!(liveness.initialDelaySeconds, 5);
        assert_eq!(liveness.periodSeconds, 10);
        assert_eq!(liveness.failureThreshold, 3);
    }

    #[test]
    fn test_pod_yaml_with_restart_policy_parses_correctly() {
        let yaml = r#"
apiVersion: v1
kind: Pod
metadata:
  name: restart-pod
spec:
  restartPolicy: OnFailure
  containers:
    - name: test
      image: busybox:latest
"#;
        let pod = serde_yaml::from_str::<common::spec::k8s::Pod>(yaml).unwrap();
        assert_eq!(pod.get_restart_policy(), Some("OnFailure"));
    }
}

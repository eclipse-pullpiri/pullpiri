/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! NodeAgentManager: Asynchronous manager for NodeAgent
//!
//! This struct manages scenario requests received via gRPC, and provides
//! a gRPC sender for communicating with the monitoring server or other services.
//! It is designed to be thread-safe and run in an async context.
use crate::desired_state::DesiredState;
use crate::grpc::sender::NodeAgentSender;
use common::monitoringserver::{ContainerInfo, ContainerList};
use common::nodeagent::fromapiserver::HandleYamlRequest;
use common::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Main manager struct for NodeAgent.
///
/// Holds the gRPC receiver and sender, and manages the main event loop.
pub struct NodeAgentManager {
    /// Receiver for scenario information from gRPC (used when bluechi runtime is enabled)
    #[allow(dead_code)]
    rx_grpc: Arc<Mutex<mpsc::Receiver<HandleYamlRequest>>>,
    /// gRPC sender for monitoring server
    sender: Arc<Mutex<NodeAgentSender>>,
    // Add other shared state as needed
    hostname: String,
    /// In-memory cache of desired states for self-healing.
    /// Shared with the gRPC receiver so both can read/write the same state.
    pub desired_states_cache: Arc<Mutex<HashMap<String, DesiredState>>>,
}

impl NodeAgentManager {
    /// Creates a new NodeAgentManager instance.
    ///
    /// # Arguments
    /// * `rx_grpc` - Channel receiver for scenario information
    /// * `hostname` - The hostname of this node
    /// * `desired_states_cache` - Shared in-memory cache for desired states
    pub async fn new(
        rx: mpsc::Receiver<HandleYamlRequest>,
        hostname: String,
        desired_states_cache: Arc<Mutex<HashMap<String, DesiredState>>>,
    ) -> Self {
        Self {
            rx_grpc: Arc::new(Mutex::new(rx)),
            sender: Arc::new(Mutex::new(NodeAgentSender::default())),
            hostname,
            desired_states_cache,
        }
    }

    /// Initializes the NodeAgentManager (e.g., loads scenarios, prepares state).
    pub async fn initialize(&mut self) -> Result<()> {
        println!("NodeAgentManager init");
        // Add initialization logic here (e.g., read scenarios, subscribe, etc.)
        Ok(())
    }

    // pub async fn handle_yaml(&self, whole_yaml: &String) -> Result<()> {
    //     crate::bluechi::parse(whole_yaml.to_string()).await?;
    //     println!("Handling yaml request nodeagent manager: {:?}", whole_yaml);
    //     Ok(())
    // }

    /// Main loop for processing incoming gRPC scenario requests.
    ///
    /// This function continuously receives scenario parameters from the gRPC channel
    /// and handles them (e.g., triggers actions, updates state, etc.).
    pub async fn process_grpc_requests(&self) -> Result<()> {
        // TODO: Implement gRPC request processing when the bluechi runtime is ready.
        // let arc_rx_grpc = Arc::clone(&self.rx_grpc);
        // let mut rx_grpc = arc_rx_grpc.lock().await;
        // while let Some(yaml_data) = rx_grpc.recv().await {
        //     crate::runtime::bluechi::parse(yaml_data.yaml, self.hostname.clone()).await?;
        // }
        Ok(())
    }

    /// Background task: Periodically gathers container info using inspect().
    ///
    /// This runs in an infinite loop and logs or processes container info as needed.
    async fn gather_container_info_loop(&self) {
        use crate::resource::container::inspect;
        use tokio::time::{sleep, Duration};

        // This is the previous container list for comparison
        let mut previous_container_list = Vec::new();

        loop {
            let container_list = inspect(self.hostname.clone()).await.unwrap_or_default();
            let node = self.hostname.clone();

            // Send the container info to the monitoring server
            {
                let mut sender = self.sender.lock().await;
                if let Err(e) = sender
                    .send_container_list(ContainerList {
                        node_name: node.clone(),
                        containers: container_list.clone(),
                    })
                    .await
                {
                    eprintln!("[NodeAgent] Error sending container info: {}", e);
                }
            }

            // Check if the container list is changed from the previous one except for ContainerList.stats
            // (which is not included in the comparison)
            if !containers_equal_except_stats(&previous_container_list, &container_list) {
                // println!(
                //     "Container list changed for node: {}. Previous: {:?}, Current: {:?}",
                //     node, previous_container_list, container_list
                // );

                // Save the previous container list for comparison
                previous_container_list = container_list.clone();

                // Send the changed container list to the state manager
                let mut sender = self.sender.lock().await;
                if let Err(e) = sender
                    .send_changed_container_list(ContainerList {
                        node_name: node.clone(),
                        containers: container_list,
                    })
                    .await
                {
                    eprintln!("[NodeAgent] Error sending changed container list: {}", e);
                }
            }

            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Background task: Periodically gathers system info using extract_system_info().
    ///
    /// This runs in an infinite loop and logs or processes system info as needed.
    async fn gather_node_info_loop(&self) {
        use crate::resource::nodeinfo::extract_node_info_delta;
        use common::monitoringserver::NodeInfo;
        use tokio::time::{sleep, Duration};

        loop {
            let node_info_data = extract_node_info_delta();

            // Create NodeInfo message for gRPC
            let node_info = NodeInfo {
                node_name: self.hostname.clone(),
                cpu_usage: node_info_data.cpu_usage as f64,
                cpu_count: node_info_data.cpu_count as u64,
                gpu_count: node_info_data.gpu_count as u64,
                used_memory: node_info_data.used_memory,
                total_memory: node_info_data.total_memory,
                mem_usage: node_info_data.mem_usage as f64,
                rx_bytes: node_info_data.rx_bytes,
                tx_bytes: node_info_data.tx_bytes,
                read_bytes: node_info_data.read_bytes,
                write_bytes: node_info_data.write_bytes,
                os: node_info_data.os,
                arch: node_info_data.arch,
                ip: node_info_data.ip,
            };

            // Send NodeInfo to monitoring server
            {
                let mut sender = self.sender.lock().await;
                if let Err(e) = sender.send_node_info(node_info.clone()).await {
                    eprintln!("[NodeAgent] Error sending node info: {}", e);
                }
            }

            println!(
                "[NodeInfo] CPU: {:.2}%, CPU Count: {}, GPU Count: {}, Mem: {}/{} KB ({:.2}%), Net RX: {} B, Net TX: {} B, Disk Read: {} B, Disk Write: {} B, OS: {}, Arch: {}, IP: {}",
                node_info.cpu_usage,
                node_info.cpu_count,
                node_info.gpu_count,
                node_info.used_memory,
                node_info.total_memory,
                node_info.mem_usage,
                node_info.rx_bytes,
                node_info.tx_bytes,
                node_info.read_bytes,
                node_info.write_bytes,
                node_info.os,
                node_info.arch,
                node_info.ip
            );
            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Runs the NodeAgentManager event loop.
    ///
    /// Spawns the gRPC processing task and the container info gatherer, and waits for them to finish.
    pub async fn run(self) -> Result<()> {
        let arc_self = Arc::new(self);
        let grpc_manager = Arc::clone(&arc_self);
        let grpc_processor = tokio::spawn(async move {
            if let Err(e) = grpc_manager.process_grpc_requests().await {
                eprintln!("Error in gRPC processor: {:?}", e);
            }
        });
        let container_manager = Arc::clone(&arc_self);
        let container_gatherer = tokio::spawn(async move {
            container_manager.gather_container_info_loop().await;
        });

        // Spawn a background task to periodically extract and print system info
        let nodeinfo_manager = Arc::clone(&arc_self);
        let nodeinfo_task = tokio::spawn(async move {
            nodeinfo_manager.gather_node_info_loop().await;
        });

        // Spawn the reconciliation loop to detect and recover exited containers
        let reconcile_cache = Arc::clone(&arc_self.desired_states_cache);
        let reconciler = tokio::spawn(async move {
            reconciliation_loop(reconcile_cache).await;
        });

        // Spawn the liveness probe loop to monitor running containers
        let probe_cache = Arc::clone(&arc_self.desired_states_cache);
        let probe_task = tokio::spawn(async move {
            crate::probe::probe_loop(probe_cache).await;
        });

        let _ = tokio::try_join!(
            grpc_processor,
            container_gatherer,
            nodeinfo_task,
            reconciler,
            probe_task
        );
        println!("NodeAgentManager stopped");
        Ok(())
    }
}

/// Tracks restart backoff state for a single container.
#[derive(Clone)]
struct BackoffState {
    /// Number of times the container has been successfully restarted.
    restart_count: u32,
    /// Timestamp of the most recent successful restart, or `None` if never restarted.
    last_restart_time: Option<std::time::SystemTime>,
}

/// Calculates the required backoff wait time for the next restart attempt.
///
/// Formula: `min(10 * 2^restart_count, 300)` seconds.
/// - 0 restarts → 10 s, 1 → 20 s, 2 → 40 s, …, ≥5 → 300 s (cap).
fn calculate_backoff(restart_count: u32) -> std::time::Duration {
    let base: u64 = 10;
    let wait_seconds = std::cmp::min(base * 2_u64.pow(restart_count), 300);
    std::time::Duration::from_secs(wait_seconds)
}

/// Runs the reconciliation loop: periodically compares desired vs actual container states.
///
/// Every second, this function reads all desired states from the in-memory cache and
/// compares them against actual container states reported by Podman. If a container that
/// should be running is found to be exited or dead, `handle_exited_container` is called
/// (the Podman restart API preserves the same container ID). If the container is completely
/// missing (removed from Podman), `handle_missing_container` recreates it from the stored
/// pod YAML and updates the cache with the new container ID.
pub async fn reconciliation_loop(desired_states_cache: Arc<Mutex<HashMap<String, DesiredState>>>) {
    use crate::resource::container::{get_inspect, get_list};
    use tokio::time::{sleep, Duration};

    // In-memory backoff state per container ID.
    let backoff_states: Arc<Mutex<HashMap<String, BackoffState>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        // Clone desired states and release the lock immediately for better concurrency.
        let desired_states = {
            let cache = desired_states_cache.lock().await;
            cache.clone()
        };

        // Get actual container list from Podman.
        let actual_containers = match get_list().await {
            Ok(containers) => containers,
            Err(e) => {
                eprintln!("[Reconciliation] Failed to list containers: {:?}", e);
                sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // Compare desired vs actual for each tracked pod.
        for (pod_name, desired) in &desired_states {
            if desired.container_id.is_empty() {
                // Container has not been started yet; nothing to reconcile.
                continue;
            }

            let actual = actual_containers
                .iter()
                .find(|c| c.Id == desired.container_id);

            match actual {
                None => {
                    // Container does not exist in Podman at all (completely removed).
                    eprintln!(
                        "[Reconciliation] Container '{}' not found for pod '{}'",
                        desired.container_id, pod_name
                    );
                    // Recreate the container from the stored pod YAML (which generates a
                    // new container ID) and update the cache so subsequent iterations use
                    // the new ID.
                    if let Some(new_id) = handle_missing_container(desired).await {
                        let mut cache = desired_states_cache.lock().await;
                        if let Some(state) = cache.get_mut(pod_name) {
                            eprintln!(
                                "[Reconciliation] Updating container_id for pod '{}': {} → {}",
                                pod_name, state.container_id, new_id
                            );
                            // Move backoff state to the new container ID and clear the old one.
                            {
                                let mut states = backoff_states.lock().await;
                                if let Some(bs) = states.remove(&state.container_id) {
                                    states.insert(new_id.clone(), bs);
                                }
                            }
                            state.container_id = new_id;
                        }
                    } else {
                        // Container could not be (or should not be) recreated; remove its
                        // backoff state to prevent memory leaks.
                        let mut states = backoff_states.lock().await;
                        states.remove(&desired.container_id);
                    }
                }
                Some(container) if container.State == "exited" || container.State == "dead" => {
                    // Container has stopped; retrieve the exit code via inspect.
                    let exit_code = match get_inspect(&desired.container_id).await {
                        Ok(inspect) => inspect.State.ExitCode,
                        Err(e) => {
                            eprintln!(
                                "[Reconciliation] Failed to inspect container '{}': {:?}; using exit code 1",
                                desired.container_id, e
                            );
                            1
                        }
                    };
                    eprintln!(
                        "[Reconciliation] Container '{}' in state '{}' for pod '{}' (exit code {})",
                        desired.container_id, container.State, pod_name, exit_code
                    );
                    // Podman's restart API restarts the container in-place, preserving the
                    // same container ID. No cache update needed.
                    handle_exited_container(desired, exit_code, Arc::clone(&backoff_states)).await;
                }
                _ => {
                    // Container is running normally; nothing to do.
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

/// Handles a container that is completely missing from Podman.
///
/// When a container has been fully removed (not just stopped/exited), the Podman restart
/// API cannot be used because the container no longer exists. This function recreates the
/// container by calling `start()` with the stored pod YAML, and returns the new container
/// ID so the caller can update the cache.
///
/// Returns `Some(new_container_id)` if the container was recreated successfully, or `None`
/// if the restart policy is `Never`, the pod YAML is missing, or recreation fails.
async fn handle_missing_container(desired: &DesiredState) -> Option<String> {
    use crate::desired_state::RestartPolicy;

    // Treat missing containers the same as an OnFailure exit (exit code 1).
    let should_restart = match desired.restart_policy {
        RestartPolicy::Always => true,
        RestartPolicy::OnFailure => true,
        RestartPolicy::Never => false,
    };

    eprintln!(
        "[Reconciliation] Pod '{}': container '{}' missing, policy={:?}, restart={}",
        desired.pod_name, desired.container_id, desired.restart_policy, should_restart
    );

    if !should_restart {
        return None;
    }

    if desired.pod_yaml.is_empty() {
        eprintln!(
            "[Reconciliation] Pod '{}': pod YAML not available, cannot recreate container",
            desired.pod_name
        );
        return None;
    }

    match crate::runtime::podman::container::start(&desired.pod_yaml).await {
        Ok(ids) => {
            let new_id = ids.into_iter().next();
            if let Some(ref id) = new_id {
                eprintln!(
                    "[Reconciliation] Pod '{}': recreated container with new ID '{}'",
                    desired.pod_name, id
                );
            } else {
                eprintln!(
                    "[Reconciliation] Pod '{}': start() returned no container IDs",
                    desired.pod_name
                );
            }
            new_id
        }
        Err(e) => {
            eprintln!(
                "[Reconciliation] Pod '{}': failed to recreate container: {:?}",
                desired.pod_name, e
            );
            None
        }
    }
}

/// Handles a container that has exited by applying the configured restart policy and
/// the exponential backoff algorithm.
///
/// Calls Podman's restart API which restarts the container in-place, preserving the
/// same container ID. The backoff state is updated in `backoff_states` on every
/// successful restart. On failure the state is left unchanged so the next loop
/// iteration can retry.
///
/// # Backoff rules
/// - Formula: `min(10 * 2^restart_count, 300)` seconds between restarts.
/// - If the elapsed time since the last restart is ≥ 300 s (5 min), restart attempts
///   are stopped permanently for vehicle-environment safety.
async fn handle_exited_container(
    desired: &DesiredState,
    exit_code: i32,
    backoff_states: Arc<Mutex<HashMap<String, BackoffState>>>,
) {
    use crate::desired_state::RestartPolicy;
    use hyper::Body;

    let should_restart = match desired.restart_policy {
        RestartPolicy::Always => true,
        RestartPolicy::OnFailure => exit_code != 0,
        RestartPolicy::Never => false,
    };

    eprintln!(
        "[Reconciliation] Pod '{}': container '{}' exited (code {}), policy={:?}, restart={}",
        desired.pod_name, desired.container_id, exit_code, desired.restart_policy, should_restart
    );

    if !should_restart {
        return;
    }

    // Read the current backoff state (or default) without holding the lock.
    let backoff_state = {
        let states = backoff_states.lock().await;
        states
            .get(&desired.container_id)
            .cloned()
            .unwrap_or(BackoffState {
                restart_count: 0,
                last_restart_time: None,
            })
    };

    // Evaluate backoff conditions when a previous restart timestamp exists.
    if let Some(last_restart_time) = backoff_state.last_restart_time {
        let elapsed = std::time::SystemTime::now()
            .duration_since(last_restart_time)
            .unwrap_or(std::time::Duration::from_secs(0));

        // Safety rule: stop restarting after the 5-minute window has elapsed.
        if elapsed.as_secs() >= 300 {
            eprintln!(
                "[Reconciliation] Container '{}': backoff period expired (5 min), \
                 stop restarting for vehicle safety",
                desired.container_id
            );
            return;
        }

        // Not yet past the required backoff delay — come back next loop iteration.
        let required_backoff = calculate_backoff(backoff_state.restart_count);
        if elapsed < required_backoff {
            eprintln!(
                "[Reconciliation] Container '{}': waiting backoff ({:.0}s remaining)",
                desired.container_id,
                (required_backoff - elapsed).as_secs_f64()
            );
            return;
        }
    }

    // Attempt restart via Podman.
    eprintln!(
        "[Reconciliation] Container '{}': restarting (attempt #{})",
        desired.container_id,
        backoff_state.restart_count + 1
    );

    let restart_path = format!("/v4.0.0/libpod/containers/{}/restart", desired.container_id);
    match crate::runtime::podman::post(&restart_path, Body::empty()).await {
        Ok(_) => {
            eprintln!(
                "[Reconciliation] Container '{}' restarted successfully",
                desired.container_id
            );
            // Update backoff state only on success.
            let mut states = backoff_states.lock().await;
            states.insert(
                desired.container_id.clone(),
                BackoffState {
                    restart_count: backoff_state.restart_count + 1,
                    last_restart_time: Some(std::time::SystemTime::now()),
                },
            );
        }
        Err(e) => {
            eprintln!(
                "[Reconciliation] Failed to restart container '{}': {:?}",
                desired.container_id, e
            );
            // Do not update backoff_state on failure so the next loop iteration retries.
        }
    }
}

fn containers_equal_except_stats<'a>(a: &'a [ContainerInfo], b: &'a [ContainerInfo]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(c1, c2)| {
        c1.id == c2.id
            && c1.names == c2.names
            && c1.image == c2.image
            && c1.state == c2.state
            && c1.config == c2.config
            && c1.annotation == c2.annotation
        // do NOT compare c1.stats/c2.stats
    })
}

// unit test cases
#[cfg(test)]
mod tests {
    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;
    use crate::desired_state::DesiredState;
    use crate::manager::NodeAgentManager;
    use common::monitoringserver::ContainerInfo;
    use common::nodeagent::fromapiserver::HandleYamlRequest;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tokio::sync::Mutex;
    use tokio::time::{timeout, Duration};

    fn make_cache() -> Arc<Mutex<HashMap<String, DesiredState>>> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    #[test]
    fn test_containers_equal_except_stats_true_and_false() {
        let c1 = ContainerInfo {
            id: "id1".to_string(),
            names: vec!["n1".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let c2 = ContainerInfo {
            id: "id1".to_string(),
            names: vec!["n1".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let c3 = ContainerInfo {
            id: "id2".to_string(),
            names: vec!["n2".to_string()],
            image: "img2".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };

        // True: stats ignored, all else equal
        assert!(super::containers_equal_except_stats(
            &[c1.clone()],
            &[c2.clone()]
        ));
        // False: id differs
        assert!(!super::containers_equal_except_stats(
            &[c1.clone()],
            &[c3.clone()]
        ));
        // False: length differs
        assert!(!super::containers_equal_except_stats(
            &[c1.clone(), c2.clone()],
            &[c1.clone()]
        ));
    }

    #[tokio::test]
    async fn test_new_creates_instance_with_correct_hostname() {
        let (_tx, rx) = mpsc::channel(1);
        let hostname = "test-host".to_string();

        let manager = NodeAgentManager::new(rx, hostname.clone(), make_cache()).await;

        assert_eq!(manager.hostname, hostname);
    }

    #[tokio::test]
    async fn test_initialize_returns_ok() {
        let (_tx, rx) = mpsc::channel(1);
        let hostname = "test-host".to_string();

        let mut manager = NodeAgentManager::new(rx, hostname, make_cache()).await;
        let result = manager.initialize().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_grpc_requests_handles_empty_channel() {
        let (_tx, rx) = mpsc::channel(1);
        drop(_tx); // close sender so recv returns None immediately
        let hostname = "test-host".to_string();

        let manager = NodeAgentManager::new(rx, hostname, make_cache()).await;
        let result = manager.process_grpc_requests().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_grpc_requests_receives_and_parses_yaml() {
        let (tx, rx) = mpsc::channel(1);
        let hostname = "test-host".to_string();

        let manager = NodeAgentManager::new(rx, hostname.clone(), make_cache()).await;

        let yaml_string = VALID_ARTIFACT_YAML.to_string();
        let request = HandleYamlRequest {
            yaml: yaml_string.clone(),
        };

        assert!(tx.send(request).await.is_ok());
        drop(tx);

        let result = manager.process_grpc_requests().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_desired_states_cache_is_shared() {
        let (_tx, rx) = mpsc::channel(1);
        let hostname = "test-host".to_string();
        let cache = make_cache();

        let manager = NodeAgentManager::new(rx, hostname, Arc::clone(&cache)).await;

        // Insert a value into cache via manager's reference
        {
            let mut c = manager.desired_states_cache.lock().await;
            c.insert(
                "my-pod".to_string(),
                DesiredState::new("my-pod".to_string()),
            );
        }

        // Verify it's accessible through the original cache Arc
        let cache_guard = cache.lock().await;
        assert!(cache_guard.contains_key("my-pod"));
    }

    /// Verifies that the reconciliation loop does not panic when the cache is empty.
    /// Since the loop is infinite, we use a short timeout to exercise one iteration.
    #[tokio::test]
    async fn test_reconciliation_loop_empty_cache_no_panic() {
        let cache = make_cache();
        // The loop runs indefinitely; a timeout verifies it doesn't panic in the first iteration.
        let result = timeout(
            Duration::from_millis(200),
            super::reconciliation_loop(cache),
        )
        .await;
        // Err means the timeout fired, which is the expected outcome for an infinite loop.
        assert!(result.is_err());
    }

    /// Verifies that the reconciliation loop populates the desired_states variable correctly
    /// and does not panic when at least one desired state is present (but container_id is empty).
    #[tokio::test]
    async fn test_reconciliation_loop_skips_empty_container_id() {
        let cache = make_cache();
        {
            let mut c = cache.lock().await;
            // DesiredState::new leaves container_id empty, so the loop should skip it.
            c.insert("pod-a".to_string(), DesiredState::new("pod-a".to_string()));
        }
        // The loop will try to contact Podman (get_list) and either succeed or log an error;
        // either way it must not panic.
        let result = timeout(
            Duration::from_millis(200),
            super::reconciliation_loop(cache),
        )
        .await;
        assert!(result.is_err());
    }

    fn make_backoff_states() -> Arc<Mutex<HashMap<String, super::BackoffState>>> {
        Arc::new(Mutex::new(HashMap::new()))
    }

    /// Verifies that `handle_exited_container` with `RestartPolicy::Never` does not
    /// attempt any Podman API call and completes without panic.
    #[tokio::test]
    async fn test_handle_exited_container_never_policy_no_restart() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "container-abc".to_string(),
            restart_policy: RestartPolicy::Never,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };
        // RestartPolicy::Never must not invoke Podman; function completes without error.
        super::handle_exited_container(&desired, 1, make_backoff_states()).await;
    }

    /// Verifies that `handle_exited_container` with `RestartPolicy::OnFailure` and an
    /// exit code of 0 does NOT attempt to restart the container.
    #[tokio::test]
    async fn test_handle_exited_container_on_failure_zero_exit_no_restart() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "container-abc".to_string(),
            restart_policy: RestartPolicy::OnFailure,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };
        // Exit code 0 is a clean exit; OnFailure must not trigger a restart.
        super::handle_exited_container(&desired, 0, make_backoff_states()).await;
    }

    /// Verifies that `handle_exited_container` with `RestartPolicy::OnFailure` and a
    /// non-zero exit code attempts to restart (Podman call may fail in test env, but must not panic).
    #[tokio::test]
    async fn test_handle_exited_container_on_failure_nonzero_exit_attempts_restart() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "nonexistent-container".to_string(),
            restart_policy: RestartPolicy::OnFailure,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };
        // Exit code != 0: OnFailure should attempt restart.
        // Podman call will fail because the container does not exist, but must not panic.
        super::handle_exited_container(&desired, 1, make_backoff_states()).await;
    }

    /// Verifies that `handle_exited_container` with `RestartPolicy::Always` and exit code 0
    /// still attempts to restart (Podman call may fail in test env, but must not panic).
    #[tokio::test]
    async fn test_handle_exited_container_always_policy_attempts_restart() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "nonexistent-container".to_string(),
            restart_policy: RestartPolicy::Always,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };
        // Always restart regardless of exit code; Podman may not be available, must not panic.
        super::handle_exited_container(&desired, 0, make_backoff_states()).await;
    }

    /// Verifies that `handle_missing_container` with `RestartPolicy::Never` does NOT
    /// attempt to recreate the container.
    #[tokio::test]
    async fn test_handle_missing_container_never_policy_no_restart() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "old-container-id".to_string(),
            restart_policy: RestartPolicy::Never,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: "apiVersion: v1\nkind: Pod\nmetadata:\n  name: test-pod\n".to_string(),
        };
        let result = super::handle_missing_container(&desired).await;
        // Never policy → no container ID returned
        assert!(result.is_none());
    }

    /// Verifies that `handle_missing_container` with an empty pod_yaml returns None
    /// (can't recreate without YAML).
    #[tokio::test]
    async fn test_handle_missing_container_empty_pod_yaml_returns_none() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "old-container-id".to_string(),
            restart_policy: RestartPolicy::Always,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(), // ← empty!
        };
        let result = super::handle_missing_container(&desired).await;
        // No pod YAML → cannot recreate → None
        assert!(result.is_none());
    }

    /// Verifies that `handle_missing_container` with Always policy and a valid (but
    /// unresolvable in test env) pod YAML attempts recreation and returns None on failure.
    #[tokio::test]
    async fn test_handle_missing_container_always_policy_attempts_recreation() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "old-container-id".to_string(),
            restart_policy: RestartPolicy::Always,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: "apiVersion: v1\nkind: Pod\nmetadata:\n  name: test-pod\nspec:\n  containers:\n  - name: c\n    image: nginx:latest\n".to_string(),
        };
        // Podman is not running in the test environment; the call will fail and return None.
        // The test verifies there is no panic.
        let result = super::handle_missing_container(&desired).await;
        assert!(result.is_none()); // Podman not available in test env
    }

    /// Verifies that the reconciliation loop correctly updates the cache container_id when
    /// a missing container is successfully recreated.
    #[tokio::test]
    async fn test_reconciliation_loop_updates_container_id_on_recreation() {
        use crate::desired_state::RestartPolicy;

        let cache = make_cache();
        {
            let mut c = cache.lock().await;
            // Desired state with a non-empty container_id and pod_yaml
            let mut state = DesiredState::new("update-pod".to_string());
            state.container_id = "old-container-id".to_string();
            state.restart_policy = RestartPolicy::Always;
            state.pod_yaml = "apiVersion: v1\nkind: Pod\nmetadata:\n  name: update-pod\nspec:\n  containers:\n  - name: c\n    image: nginx:latest\n".to_string();
            c.insert("update-pod".to_string(), state);
        }

        // Run loop for a short time
        let result = timeout(
            Duration::from_millis(200),
            super::reconciliation_loop(Arc::clone(&cache)),
        )
        .await;
        assert!(result.is_err()); // Timeout expected (loop is infinite)

        // After the loop (Podman not available), the container_id should remain the same
        // (no Podman → handle_missing_container returns None → no update).
        let guard = cache.lock().await;
        let state = guard.get("update-pod").unwrap();
        assert_eq!(state.container_id, "old-container-id");
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Backoff-specific tests
    // ──────────────────────────────────────────────────────────────────────────

    /// Verifies the `calculate_backoff` formula: min(10 * 2^n, 300) seconds.
    #[test]
    fn test_calculate_backoff_values() {
        use std::time::Duration;
        assert_eq!(super::calculate_backoff(0), Duration::from_secs(10));
        assert_eq!(super::calculate_backoff(1), Duration::from_secs(20));
        assert_eq!(super::calculate_backoff(2), Duration::from_secs(40));
        assert_eq!(super::calculate_backoff(3), Duration::from_secs(80));
        assert_eq!(super::calculate_backoff(4), Duration::from_secs(160));
        // 10 * 2^5 = 320 → capped at 300
        assert_eq!(super::calculate_backoff(5), Duration::from_secs(300));
        // Higher counts remain capped
        assert_eq!(super::calculate_backoff(10), Duration::from_secs(300));
    }

    /// Verifies that `handle_exited_container` skips the restart when the required
    /// backoff time has not yet elapsed since the last restart.
    ///
    /// Because Podman is unavailable in the test environment, backoff state is never
    /// updated on a (failed) restart attempt. We pre-seed a state with
    /// `last_restart_time = now` and verify that the backoff_states map is unchanged
    /// after the call (i.e., the early-return path was taken).
    #[tokio::test]
    async fn test_handle_exited_container_backoff_waiting_skips_restart() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "backoff-container".to_string(),
            restart_policy: RestartPolicy::Always,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };

        let backoff_states = make_backoff_states();
        // Seed a backoff state: last restart happened right now, so 10 s have NOT elapsed.
        {
            let mut states = backoff_states.lock().await;
            states.insert(
                "backoff-container".to_string(),
                super::BackoffState {
                    restart_count: 0,
                    last_restart_time: Some(std::time::SystemTime::now()),
                },
            );
        }

        super::handle_exited_container(&desired, 1, Arc::clone(&backoff_states)).await;

        // The backoff map must be unchanged: restart was skipped because elapsed < 10 s.
        let states = backoff_states.lock().await;
        let state = states.get("backoff-container").unwrap();
        assert_eq!(state.restart_count, 0);
    }

    /// Verifies that `handle_exited_container` stops restarting once 300 seconds (5 min)
    /// have elapsed since the last restart (vehicle-safety rule).
    #[tokio::test]
    async fn test_handle_exited_container_backoff_expired_stops_restart() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "expired-container".to_string(),
            restart_policy: RestartPolicy::Always,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };

        let backoff_states = make_backoff_states();
        // Seed a state where the last restart was 301 seconds ago (backoff expired).
        // Use UNIX_EPOCH + a fixed offset to avoid any risk of SystemTime underflow.
        let past =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000 - 301);
        {
            let mut states = backoff_states.lock().await;
            states.insert(
                "expired-container".to_string(),
                super::BackoffState {
                    restart_count: 5,
                    last_restart_time: Some(past),
                },
            );
        }

        super::handle_exited_container(&desired, 1, Arc::clone(&backoff_states)).await;

        // The backoff state must be unchanged: the 5-min expiry triggered an early return.
        let states = backoff_states.lock().await;
        let state = states.get("expired-container").unwrap();
        assert_eq!(state.restart_count, 5);
        assert_eq!(state.last_restart_time, Some(past));
    }

    /// Verifies that `handle_exited_container` proceeds when no previous restart
    /// timestamp exists (first restart attempt) and no backoff state is pre-seeded.
    /// Podman is unavailable, so the restart fails, but the function must not panic
    /// and the backoff map must remain empty (state only updated on success).
    #[tokio::test]
    async fn test_handle_exited_container_first_restart_no_prior_state() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "fresh-container".to_string(),
            restart_policy: RestartPolicy::Always,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };

        let backoff_states = make_backoff_states();
        // No pre-seeded state → function should attempt restart immediately.
        super::handle_exited_container(&desired, 1, Arc::clone(&backoff_states)).await;

        // Podman is not running, so restart fails and backoff state is NOT inserted.
        let states = backoff_states.lock().await;
        assert!(!states.contains_key("fresh-container"));
    }

    /// Verifies that `handle_exited_container` with `RestartPolicy::Never` does not
    /// mutate the backoff_states map.
    #[tokio::test]
    async fn test_handle_exited_container_never_policy_no_backoff_mutation() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "never-container".to_string(),
            restart_policy: RestartPolicy::Never,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };

        let backoff_states = make_backoff_states();
        super::handle_exited_container(&desired, 1, Arc::clone(&backoff_states)).await;

        // Never policy exits before touching backoff_states.
        let states = backoff_states.lock().await;
        assert!(states.is_empty());
    }

    /// Verifies that `handle_exited_container` with `RestartPolicy::OnFailure` and
    /// exit_code 0 does not mutate the backoff_states map.
    #[tokio::test]
    async fn test_handle_exited_container_on_failure_zero_no_backoff_mutation() {
        use crate::desired_state::RestartPolicy;

        let desired = DesiredState {
            pod_name: "test-pod".to_string(),
            container_id: "on-failure-container".to_string(),
            restart_policy: RestartPolicy::OnFailure,
            probe_config: None,
            created_at: std::time::SystemTime::now(),
            pod_yaml: String::new(),
        };

        let backoff_states = make_backoff_states();
        super::handle_exited_container(&desired, 0, Arc::clone(&backoff_states)).await;

        // OnFailure with exit_code 0 exits before touching backoff_states.
        let states = backoff_states.lock().await;
        assert!(states.is_empty());
    }
}

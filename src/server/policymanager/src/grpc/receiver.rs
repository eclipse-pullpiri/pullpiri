/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::policymanager::policy_manager_connection_server::PolicyManagerConnection;
use common::policymanager::{
    CheckNodePolicyRequest, CheckNodePolicyResponse, ReportNodeMetricsRequest,
    ReportNodeMetricsResponse, RunningContainer,
};
use common::spec::artifact::Policy;
use common::statemanager::OffloadingRequest;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};
use tonic::{Request, Response, Status};

const ETCD_POLICY_PREFIX: &str = "Policy";
/// Cooldown duration before allowing another offload for the same package
const OFFLOAD_COOLDOWN_SECS: u64 = 30;
/// Cache TTL for policies (seconds)
const POLICY_CACHE_TTL_SECS: u64 = 10;

/// Cached policy with timestamp
struct CachedPolicy {
    policy: Policy,
    cached_at: Instant,
}

lazy_static::lazy_static! {
    /// Track last offload time per package to prevent duplicate offloading
    static ref OFFLOAD_COOLDOWNS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
    /// Policy cache to reduce etcd calls
    static ref POLICY_CACHE: RwLock<HashMap<String, CachedPolicy>> = RwLock::new(HashMap::new());
}

/// Get policy from cache or fetch from etcd
async fn get_policy_cached(policy_name: &str) -> Option<Policy> {
    // Try to get from cache first
    {
        let cache = POLICY_CACHE.read().unwrap();
        if let Some(cached) = cache.get(policy_name) {
            if cached.cached_at.elapsed() < Duration::from_secs(POLICY_CACHE_TTL_SECS) {
                return Some(cached.policy.clone());
            }
        }
    }

    // Cache miss or expired - fetch from etcd
    let etcd_key = format!("{}/{}", ETCD_POLICY_PREFIX, policy_name);
    let policy_str = common::etcd::get(&etcd_key).await.ok()?;
    let policy: Policy = serde_yaml::from_str(&policy_str).ok()?;

    // Store in cache
    {
        let mut cache = POLICY_CACHE.write().unwrap();
        cache.insert(
            policy_name.to_string(),
            CachedPolicy {
                policy: policy.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    Some(policy)
}

/// gRPC server implementation for PolicyManager
pub struct PolicyManagerGrpcServer {}

impl PolicyManagerGrpcServer {
    pub fn new() -> Self {
        Self {}
    }

    /// Check if resource threshold is exceeded and trigger offloading if needed
    /// Returns true if offloading was triggered, false otherwise
    async fn check_threshold_and_trigger_offloading(
        &self,
        node_info: &common::monitoringserver::NodeInfo,
        container: &RunningContainer,
    ) -> bool {
        let policy_name = &container.policy_name;
        let package_name = &container.package_name;

        if policy_name.is_empty() {
            return false;
        }

        // Check cooldown - skip if this package was offloaded recently
        {
            let cooldowns = OFFLOAD_COOLDOWNS.lock().unwrap();
            if let Some(last_offload) = cooldowns.get(package_name) {
                if last_offload.elapsed() < Duration::from_secs(OFFLOAD_COOLDOWN_SECS) {
                    println!(
                        "[PolicyManager] Skipping offload for package '{}': cooldown active ({:.1}s remaining)",
                        package_name,
                        OFFLOAD_COOLDOWN_SECS as f64 - last_offload.elapsed().as_secs_f64()
                    );
                    return false;
                }
            }
        }

        // Fetch policy from cache (or etcd if not cached)
        let policy = match get_policy_cached(policy_name).await {
            Some(p) => p,
            None => return false, // Policy not found or parse error, skip
        };

        // Get threshold from policy
        let procedure = policy.get_procedure();
        let trigger = procedure.get_trigger();

        let threshold = match &trigger.resourceThreshold {
            Some(t) => t,
            None => return false, // No threshold defined
        };

        // Check CPU threshold
        let cpu_exceeded = threshold.cpu.map_or(false, |cpu_threshold| {
            node_info.cpu_usage > cpu_threshold as f64
        });

        // Check memory threshold
        let mem_exceeded = threshold.memory.map_or(false, |mem_threshold| {
            node_info.mem_usage > mem_threshold as f64
        });

        if !cpu_exceeded && !mem_exceeded {
            return false; // No threshold exceeded
        }

        // Find target node for offloading
        let placement = policy.get_placement();
        let available_nodes = placement.get_available_nodes();
        let current_node = &node_info.node_name;

        // Find first available node that is not the current node
        let target_node = available_nodes.iter().find(|n| *n != current_node).cloned();

        let target_node = match target_node {
            Some(n) => n,
            None => {
                println!(
                    "[PolicyManager] No target node available for offloading package '{}' from '{}'",
                    package_name, current_node
                );
                return false;
            }
        };

        // Build reason message
        let reason = if cpu_exceeded && mem_exceeded {
            format!(
                "CPU ({:.1}% > {}%) and Memory ({:.1}% > {}%) threshold exceeded",
                node_info.cpu_usage,
                threshold.cpu.unwrap_or(0),
                node_info.mem_usage,
                threshold.memory.unwrap_or(0)
            )
        } else if cpu_exceeded {
            format!(
                "CPU threshold exceeded: {:.1}% > {}%",
                node_info.cpu_usage,
                threshold.cpu.unwrap_or(0)
            )
        } else {
            format!(
                "Memory threshold exceeded: {:.1}% > {}%",
                node_info.mem_usage,
                threshold.memory.unwrap_or(0)
            )
        };

        println!(
            "[PolicyManager] Triggering offloading: package '{}' from '{}' to '{}'. Reason: {}",
            package_name, current_node, target_node, reason
        );

        // Trigger offloading via StateManager
        let offloading_request = OffloadingRequest {
            scenario_name: container.scenario_name.clone(),
            package_name: container.package_name.clone(),
            model_name: container.model_name.clone(),
            source_node: current_node.clone(),
            target_node: target_node.clone(),
            policy_name: policy_name.clone(),
            reason,
        };

        match super::sender::trigger_offloading(offloading_request).await {
            Ok(response) => {
                let resp = response.into_inner();
                if resp.accepted {
                    println!(
                        "[PolicyManager] Offloading request accepted: {}",
                        resp.message
                    );
                    // Record cooldown for this package
                    {
                        let mut cooldowns = OFFLOAD_COOLDOWNS.lock().unwrap();
                        cooldowns.insert(package_name.clone(), Instant::now());
                    }
                    return true;
                } else {
                    println!(
                        "[PolicyManager] Offloading request rejected: {}",
                        resp.message
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "[PolicyManager] Failed to trigger offloading: {}",
                    e.message()
                );
            }
        }
        false
    }
}

impl Default for PolicyManagerGrpcServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl PolicyManagerConnection for PolicyManagerGrpcServer {
    /// Check if deployment to a specific node is allowed based on policy
    ///
    /// # Arguments
    /// * `request` - Contains policy_name and target_node
    ///
    /// # Returns
    /// * `CheckNodePolicyResponse` - allowed status, fallback_node, and message
    async fn check_node_policy(
        &self,
        request: Request<CheckNodePolicyRequest>,
    ) -> Result<Response<CheckNodePolicyResponse>, Status> {
        let req = request.into_inner();
        let policy_name = req.policy_name;
        let target_node = req.target_node;

        println!(
            "[PolicyManager] Checking policy '{}' for node '{}'",
            policy_name, target_node
        );

        // If no policy specified, allow by default
        if policy_name.is_empty() {
            println!("[PolicyManager] No policy specified, allowing deployment");
            return Ok(Response::new(CheckNodePolicyResponse {
                allowed: true,
                suggested_node: String::new(),
                message: "No policy specified, deployment allowed".to_string(),
            }));
        }

        // Fetch policy from cache (or etcd if not cached)
        let policy = match get_policy_cached(&policy_name).await {
            Some(p) => p,
            None => {
                println!(
                    "[PolicyManager] Policy '{}' not found or parse error",
                    policy_name
                );
                // If policy not found, allow by default (fail-open)
                return Ok(Response::new(CheckNodePolicyResponse {
                    allowed: true,
                    suggested_node: String::new(),
                    message: format!("Policy '{}' not found, allowing deployment", policy_name),
                }));
            }
        };

        // Check if target_node is in availableNodes
        let placement = policy.get_placement();
        let available_nodes = placement.get_available_nodes();
        let preferred_node = placement.get_preferred_node().unwrap_or("").to_string();

        let allowed = available_nodes.contains(&target_node);

        if allowed {
            println!(
                "[PolicyManager] Node '{}' is in availableNodes {:?}",
                target_node, available_nodes
            );
        } else {
            println!(
                "[PolicyManager] Node '{}' is NOT in availableNodes {:?}",
                target_node, available_nodes
            );
            if !preferred_node.is_empty() {
                println!(
                    "[PolicyManager] Suggested preferred node: '{}'",
                    preferred_node
                );
            }
        }

        Ok(Response::new(CheckNodePolicyResponse {
            allowed,
            suggested_node: preferred_node,
            message: if allowed {
                format!(
                    "Node '{}' is allowed by policy '{}'",
                    target_node, policy_name
                )
            } else {
                format!(
                    "Node '{}' is not in availableNodes {:?}",
                    target_node, available_nodes
                )
            },
        }))
    }

    /// Report node metrics from monitoring server for threshold-based policy evaluation
    ///
    /// This method is called by MonitoringServer whenever NodeInfo is received.
    /// It checks if any running containers have policies with resource thresholds,
    /// and triggers offloading if thresholds are exceeded.
    async fn report_node_metrics(
        &self,
        request: Request<ReportNodeMetricsRequest>,
    ) -> Result<Response<ReportNodeMetricsResponse>, Status> {
        let req = request.into_inner();

        let node_info = match req.node_info {
            Some(info) => info,
            None => {
                return Ok(Response::new(ReportNodeMetricsResponse {
                    processed: false,
                    message: "No NodeInfo provided".to_string(),
                }));
            }
        };

        let running_containers = req.running_containers;

        println!(
            "[PolicyManager] Received metrics for node '{}': CPU={:.1}%, Mem={:.1}%, Containers={}",
            node_info.node_name,
            node_info.cpu_usage,
            node_info.mem_usage,
            running_containers.len()
        );

        // Track which packages have been processed in this request to avoid duplicates
        let mut processed_packages: HashSet<String> = HashSet::new();

        // Check each container with a policy for threshold violations
        for container in &running_containers {
            // Skip if no policy or package defined
            if container.policy_name.is_empty() || container.package_name.is_empty() {
                continue;
            }

            // Skip if this package was already processed in this request
            if processed_packages.contains(&container.package_name) {
                println!(
                    "[PolicyManager] Skipping container '{}': package '{}' already processed in this request",
                    container.container_name, container.package_name
                );
                continue;
            }

            println!(
                "[PolicyManager] Checking container '{}' (package: {}, policy: {})",
                container.container_name, container.package_name, container.policy_name
            );

            // Check threshold and trigger offloading if needed
            if self
                .check_threshold_and_trigger_offloading(&node_info, container)
                .await
            {
                // Mark this package as processed
                processed_packages.insert(container.package_name.clone());
            }
        }

        Ok(Response::new(ReportNodeMetricsResponse {
            processed: true,
            message: format!(
                "Processed metrics for node '{}' with {} containers",
                node_info.node_name,
                running_containers.len()
            ),
        }))
    }
}

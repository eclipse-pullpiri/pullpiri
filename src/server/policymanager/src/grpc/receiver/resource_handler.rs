/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Resource threshold handler for PolicyManager
//!
//! This module handles CPU/memory threshold-based policy evaluation and
//! triggers workload offloading when thresholds are exceeded.

use super::policy_cache::get_policy_cached;
use common::monitoringserver::NodeInfo;
use common::policymanager::RunningContainer;
use common::statemanager::OffloadingRequest;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Cooldown duration before allowing another offload for the same package
const OFFLOAD_COOLDOWN_SECS: u64 = 30;

lazy_static::lazy_static! {
    /// Track last offload time per package to prevent duplicate offloading
    static ref OFFLOAD_COOLDOWNS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
}

/// Check if resource threshold is exceeded and trigger offloading if needed
///
/// # Arguments
/// * `node_info` - Current node metrics (CPU, memory usage)
/// * `container` - Running container information
///
/// # Returns
/// * `true` if offloading was triggered
/// * `false` otherwise
pub async fn check_threshold_and_trigger_offloading(
    node_info: &NodeInfo,
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

    // Fetch policy from cache (or kvstore if not cached)
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

    match crate::grpc::sender::trigger_offloading(offloading_request).await {
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

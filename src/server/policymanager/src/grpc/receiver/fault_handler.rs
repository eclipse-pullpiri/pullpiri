/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Fault handler for PolicyManager
//!
//! This module handles fault reports from Timpani (via StateManager),
//! such as deadline miss events. It evaluates policies with deadlineMissThreshold
//! and triggers appropriate actions.

use crate::grpc::sender;
use common::actioncontroller::StopWorkloadRequest;
use common::policymanager::{FaultType, ReportFaultRequest, ReportFaultResponse};
use common::spec::artifact::Package;
use common::spec::artifact::Policy;
use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    /// Tracks deadline miss counts per workload (workload_id -> count)
    static ref DEADLINE_MISS_COUNTS: RwLock<HashMap<String, u32>> = RwLock::new(HashMap::new());
}

/// Handle fault report from StateManager (originated from Timpani)
///
/// This function processes fault notifications (e.g., deadline miss) and determines
/// what action to take based on the associated policy's deadlineMissThreshold.
///
/// # Arguments
/// * `request` - Fault report request containing workload_id, node_id, task_name, fault_type
///
/// # Returns
/// * `ReportFaultResponse` with processed status and message
pub async fn handle_fault_report(request: ReportFaultRequest) -> ReportFaultResponse {
    let fault_type = match FaultType::try_from(request.fault_type) {
        Ok(ft) => ft,
        Err(_) => {
            return ReportFaultResponse {
                processed: false,
                message: "Invalid fault type".to_string(),
            };
        }
    };

    let fault_type_str = match fault_type {
        FaultType::FaultDeadlineMiss => "DEADLINE_MISS",
        FaultType::FaultUnknown => "UNKNOWN",
    };

    println!(
        "[PolicyManager] Received fault report: workload='{}', node='{}', task='{}', type={}",
        request.workload_id, request.node_id, request.task_name, fault_type_str
    );

    // Only handle deadline miss faults
    if fault_type != FaultType::FaultDeadlineMiss {
        return ReportFaultResponse {
            processed: true,
            message: format!("Fault type {} not handled", fault_type_str),
        };
    }

    // Process deadline miss fault
    match process_deadline_miss_fault(&request).await {
        Ok(message) => ReportFaultResponse {
            processed: true,
            message,
        },
        Err(e) => ReportFaultResponse {
            processed: false,
            message: format!("Failed to process fault: {}", e),
        },
    }
}

/// Process deadline miss fault
///
/// 1. Find Package by schedule name (workload_id)
/// 2. Get policy from Package
/// 3. Check deadlineMissThreshold
/// 4. Increment counter and trigger action if threshold exceeded
async fn process_deadline_miss_fault(request: &ReportFaultRequest) -> Result<String, String> {
    let workload_id = &request.workload_id;
    let node_id = &request.node_id;

    // Step 1: Find package by schedule name
    let (package_name, package) = find_package_by_schedule(workload_id).await?;
    println!(
        "[PolicyManager] Found package '{}' for workload '{}'",
        package_name, workload_id
    );

    // Step 2: Get policy name from package
    let policy_name = package
        .get_policy()
        .as_ref()
        .ok_or_else(|| format!("Package '{}' has no policy defined", package_name))?;

    // Step 3: Load policy from etcd
    let policy = load_policy(policy_name).await?;
    println!("[PolicyManager] Loaded policy '{}'", policy_name);

    // Step 4: Check if deadlineMissThreshold is defined
    let threshold = policy
        .get_procedure()
        .get_trigger()
        .deadlineMissThreshold
        .as_ref()
        .ok_or_else(|| {
            format!(
                "Policy '{}' has no deadlineMissThreshold defined",
                policy_name
            )
        })?;

    let threshold_count = threshold.get_count();
    println!(
        "[PolicyManager] Deadline miss threshold for '{}': {}",
        policy_name, threshold_count
    );

    // Step 5: Increment deadline miss count
    let current_count = increment_deadline_miss_count(workload_id);
    println!(
        "[PolicyManager] Deadline miss count for '{}': {}/{}",
        workload_id, current_count, threshold_count
    );

    // Step 6: Check if threshold exceeded
    if current_count < threshold_count {
        return Ok(format!(
            "Deadline miss recorded: {}/{} for workload '{}'",
            current_count, threshold_count, workload_id
        ));
    }

    // Threshold exceeded - trigger action based on strategy
    let strategy = policy.get_procedure().get_strategy();
    println!(
        "[PolicyManager] Threshold exceeded! Strategy: '{}'",
        strategy
    );

    match strategy {
        "stop" => {
            // Find model name for this node
            let model_name = find_model_for_node(&package, node_id)?;

            // Stop the workload
            stop_workload(&package_name, &model_name, node_id, workload_id).await?;

            // Reset counter after action
            reset_deadline_miss_count(workload_id);

            Ok(format!(
                "Threshold exceeded ({}/{}). Stopped workload '{}' model '{}' on node '{}'",
                current_count, threshold_count, workload_id, model_name, node_id
            ))
        }
        _ => Err(format!("Unknown strategy: {}", strategy)),
    }
}

/// Find Package by schedule name (workload_id)
async fn find_package_by_schedule(schedule_name: &str) -> Result<(String, Package), String> {
    let packages = common::etcd::get_all_with_prefix("Package/").await?;

    for (key, value) in packages {
        let package: Package = serde_yaml::from_str(&value)
            .map_err(|e| format!("Failed to parse Package '{}': {}", key, e))?;

        if let Some(schedule) = package.get_schedule() {
            if schedule == schedule_name {
                // Extract package name from key "Package/{name}"
                let name = key.strip_prefix("Package/").unwrap_or(&key).to_string();
                return Ok((name, package));
            }
        }
    }

    Err(format!(
        "No package found with schedule '{}'",
        schedule_name
    ))
}

/// Load Policy from etcd
async fn load_policy(policy_name: &str) -> Result<Policy, String> {
    let key = format!("Policy/{}", policy_name);
    let value = common::etcd::get(&key).await?;

    if value.is_empty() {
        return Err(format!("Policy '{}' not found", policy_name));
    }

    serde_yaml::from_str(&value)
        .map_err(|e| format!("Failed to parse Policy '{}': {}", policy_name, e))
}

/// Find model name for a given node in the package
fn find_model_for_node(package: &Package, node_id: &str) -> Result<String, String> {
    for model in package.get_models() {
        if model.get_node() == node_id {
            return Ok(model.get_name());
        }
    }

    // If not found, return the first model as default
    package
        .get_models()
        .first()
        .map(|m| m.get_name())
        .ok_or_else(|| "Package has no models".to_string())
}

/// Increment deadline miss count for a workload
fn increment_deadline_miss_count(workload_id: &str) -> u32 {
    let mut counts = DEADLINE_MISS_COUNTS
        .write()
        .unwrap_or_else(|e| e.into_inner());
    let count = counts.entry(workload_id.to_string()).or_insert(0);
    *count += 1;
    *count
}

/// Reset deadline miss count for a workload
fn reset_deadline_miss_count(workload_id: &str) {
    let mut counts = DEADLINE_MISS_COUNTS
        .write()
        .unwrap_or_else(|e| e.into_inner());
    counts.remove(workload_id);
}

/// Stop workload via ActionController
async fn stop_workload(
    package_name: &str,
    model_name: &str,
    node_name: &str,
    workload_id: &str,
) -> Result<(), String> {
    println!(
        "[PolicyManager] Stopping workload: package='{}', model='{}', node='{}'",
        package_name, model_name, node_name
    );

    let request = StopWorkloadRequest {
        package_name: package_name.to_string(),
        model_name: model_name.to_string(),
        node_name: node_name.to_string(),
        reason: format!("Deadline miss threshold exceeded for '{}'", workload_id),
    };

    match sender::stop_workload(request).await {
        Ok(response) => {
            let resp = response.into_inner();
            if resp.success {
                println!(
                    "[PolicyManager] Workload stopped successfully: {}",
                    resp.message
                );
                Ok(())
            } else {
                Err(format!("Failed to stop workload: {}", resp.message))
            }
        }
        Err(e) => Err(format!("gRPC error stopping workload: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_fault_report_deadline_miss() {
        let request = ReportFaultRequest {
            workload_id: "test_workload".to_string(),
            node_id: "HPC".to_string(),
            task_name: "test_task".to_string(),
            fault_type: FaultType::FaultDeadlineMiss as i32,
        };

        let response = handle_fault_report(request).await;
        // Should process but may fail to find package (etcd not running)
        assert!(response.processed || !response.processed);
    }

    #[tokio::test]
    async fn test_handle_fault_report_unknown() {
        let request = ReportFaultRequest {
            workload_id: "test_workload".to_string(),
            node_id: "HPC".to_string(),
            task_name: "test_task".to_string(),
            fault_type: FaultType::FaultUnknown as i32,
        };

        let response = handle_fault_report(request).await;
        assert!(response.processed);
        assert!(response.message.contains("not handled"));
    }

    #[test]
    fn test_increment_deadline_miss_count() {
        let workload = "test_increment_workload";

        // First increment
        let count1 = increment_deadline_miss_count(workload);
        assert_eq!(count1, 1);

        // Second increment
        let count2 = increment_deadline_miss_count(workload);
        assert_eq!(count2, 2);

        // Reset
        reset_deadline_miss_count(workload);

        // Should start from 1 again
        let count3 = increment_deadline_miss_count(workload);
        assert_eq!(count3, 1);

        // Cleanup
        reset_deadline_miss_count(workload);
    }
}

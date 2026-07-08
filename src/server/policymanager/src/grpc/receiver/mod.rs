/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC receiver module for PolicyManager
//!
//! This module implements the PolicyManagerConnection gRPC service, handling:
//! - Node policy checks for deployment decisions
//! - Resource threshold monitoring (CPU/memory) for offloading
//! - Fault reports (deadline miss) from Timpani via StateManager

mod fault_handler;
mod policy_cache;
mod resource_handler;

use common::policymanager::policy_manager_connection_server::PolicyManagerConnection;
use common::policymanager::{
    CheckNodePolicyRequest, CheckNodePolicyResponse, ReportFaultRequest, ReportFaultResponse,
    ReportNodeMetricsRequest, ReportNodeMetricsResponse,
};
use policy_cache::get_policy_cached;
use std::collections::HashSet;
use tonic::{Request, Response, Status};

/// gRPC server implementation for PolicyManager
pub struct PolicyManagerGrpcServer {}

impl PolicyManagerGrpcServer {
    /// Create a new PolicyManagerGrpcServer
    pub fn new() -> Self {
        Self {}
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
    /// * `CheckNodePolicyResponse` - allowed status, suggested_node, and message
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

        // Fetch policy from cache (or kvstore if not cached)
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
            if resource_handler::check_threshold_and_trigger_offloading(&node_info, container).await
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

    /// Handle fault reports from StateManager (originated from Timpani)
    ///
    /// This method receives fault notifications (e.g., deadline miss) and processes them.
    async fn report_fault(
        &self,
        request: Request<ReportFaultRequest>,
    ) -> Result<Response<ReportFaultResponse>, Status> {
        let req = request.into_inner();
        let response = fault_handler::handle_fault_report(req).await;
        Ok(Response::new(response))
    }
}

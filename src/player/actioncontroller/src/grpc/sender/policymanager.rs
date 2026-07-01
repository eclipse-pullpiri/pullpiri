/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use common::logd;
use common::policymanager::{
    policy_manager_connection_client::PolicyManagerConnectionClient, CheckNodePolicyRequest,
    CheckNodePolicyResponse,
};
use common::Result;

/// Response from policy check containing deployment decision
#[derive(Debug, Clone)]
pub struct PolicyCheckResult {
    pub allowed: bool,
    pub suggested_node: Option<String>,
    pub message: String,
}

/// Check if deployment to a specific node is allowed by policy
///
/// Makes a gRPC request to PolicyManager to check if deployment
/// to the target node is allowed based on the specified policy.
///
/// # Arguments
///
/// * `policy_name` - The name of the policy to check (e.g., "policy_helloworld")
/// * `target_node` - The node where deployment is requested (e.g., "HPC")
///
/// # Returns
///
/// * `Ok(PolicyCheckResult)` containing the policy decision
/// * `Err(...)` if the request fails
///
/// # Errors
///
/// Returns an error if:
/// - The connection to PolicyManager cannot be established
/// - The gRPC request fails
pub async fn check_node_policy(policy_name: &str, target_node: &str) -> Result<PolicyCheckResult> {
    let addr = common::policymanager::connect_server();

    logd!(
        2,
        "Checking policy '{}' for node '{}' at {}",
        policy_name,
        target_node,
        addr
    );

    let mut client = PolicyManagerConnectionClient::connect(addr)
        .await
        .map_err(|e| format!("Failed to connect to PolicyManager: {}", e))?;

    let request = tonic::Request::new(CheckNodePolicyRequest {
        policy_name: policy_name.to_string(),
        target_node: target_node.to_string(),
    });

    let response: CheckNodePolicyResponse = client
        .check_node_policy(request)
        .await
        .map_err(|e| format!("PolicyManager gRPC error: {}", e))?
        .into_inner();

    let result = PolicyCheckResult {
        allowed: response.allowed,
        suggested_node: if response.suggested_node.is_empty() {
            None
        } else {
            Some(response.suggested_node)
        },
        message: response.message,
    };

    if result.allowed {
        logd!(2, "Policy allows deployment to node '{}'", target_node);
    } else {
        logd!(
            3,
            "Policy denies deployment to node '{}': {}",
            target_node,
            result.message
        );
        if let Some(ref suggested) = result.suggested_node {
            logd!(3, "Suggested node: '{}'", suggested);
        }
    }

    Ok(result)
}

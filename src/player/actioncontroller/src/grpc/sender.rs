use common::nodeagent::{
    node_agent_connection_client::NodeAgentConnectionClient, HandleWorkloadRequest,
};
use common::policymanager::{
    policy_manager_connection_client::PolicyManagerConnectionClient, CheckPolicyRequest,
};
use common::Result;
use tonic::Request;

/// Check if a scenario is allowed by policy
///
/// Makes a gRPC request to PolicyManager to check if the scenario
/// meets the current policy requirements.
///
/// # Arguments
///
/// * `scenario_name` - The name of the scenario to check
///
/// # Returns
///
/// * `Ok(())` if the policy check passes
/// * `Err(...)` if the policy check fails or the request fails
///
/// # Errors
///
/// Returns an error if:
/// - The connection to PolicyManager is not established
/// - The gRPC request fails
/// - The policy check fails
pub async fn check_policy(scenario_name: String) -> Result<i32> {
    if scenario_name.trim().is_empty() {
        return Err("Invalid scenario name: cannot be empty".into());
    }

    let addr = common::policymanager::connect_server();
    let mut client = PolicyManagerConnectionClient::connect(addr).await.unwrap();
    let request = Request::new(CheckPolicyRequest { scenario_name });
    let response = client.check_policy(request).await?;
    let response_inner = response.into_inner();

    println!("Error: {}", response_inner.desc);
    Ok(response_inner.status)
}

/// Send a workload handling request to NodeAgent
///
/// Makes a gRPC request to NodeAgent to perform an action on a workload
/// (create, delete, start, stop, etc.)
///
/// # Arguments
///
/// * `workload_name` - The name of the workload to handle
/// * `action` - The action to perform (numeric code)
/// * `description` - Additional information about the action
///
/// # Returns
///
/// * `Ok(())` if the request was successful
/// * `Err(...)` if the request failed
///
/// # Errors
///
/// Returns an error if:
/// - The connection to NodeAgent is not established
/// - The gRPC request fails
/// - The workload handling operation fails
pub async fn handle_workload(
    workload_name: String,
    action: i32,
    description: String,
) -> Result<i32> {
    if workload_name.trim().is_empty() || description.trim().is_empty() {
        return Err("Invalid input: workload name and description cannot be empty".into());
    }
    if action < 0 {
        return Err("Invalid action: must be a non-negative integer".into());
    }

    let addr = common::nodeagent::connect_server();
    let mut client = NodeAgentConnectionClient::connect(addr).await.unwrap();
    let request = Request::new(HandleWorkloadRequest {
        workload_name,
        action,
        description,
    });
    let response = client.handle_workload(request).await?;
    let response_inner = response.into_inner();

    println!("Error: {}", response_inner.desc);
    Ok(response_inner.status)
}

// ===========================
// UNIT TESTS
// ===========================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_policy_success() {
        let scenario_name = "antipinch-enable".to_string();

        let result = check_policy(scenario_name).await;
        if let Err(ref e) = result {
            println!("Error in test_check_policy_success: {:?}", e);
        } else {
            println!("test_check_policy_success successful");
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_policy_failure_invalid_scenario() {
        // Sending invalid scenario_name to simulate policy check failure
        let scenario_name = "".to_string(); // Empty string is invalid

        let result = check_policy(scenario_name).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_workload_success() {
        let workload_name = "test-workload".to_string();
        let action = 1;
        let description = "example description".to_string();

        let result = handle_workload(workload_name, action, description).await;
        if let Err(ref e) = result {
            println!("Error in test_handle_workload_success: {:?}", e);
        } else {
            println!("test_handle_workload_success successful");
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_workload_failure_invalid_workload() {
        // Sending invalid workload_name and invalid action to trigger failure
        let workload_name = "".to_string(); // Invalid empty workload
        let action = -999; // Invalid action code
        let description = "".to_string(); // Empty description

        let result = handle_workload(workload_name, action, description).await;

        assert!(result.is_err());
    }
}

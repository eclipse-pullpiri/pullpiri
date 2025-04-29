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

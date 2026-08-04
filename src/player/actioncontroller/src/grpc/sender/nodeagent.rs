use common::nodeagent::fromactioncontroller::{
    connect_server, HandleWorkloadRequest, HandleWorkloadResponse,
};
use common::nodeagent::node_agent_connection_client::NodeAgentConnectionClient;
use common::nodeagent::{UpdateResourcesRequest, UpdateResourcesResponse};
use tonic::{Request, Status};

pub async fn send_workload_handle_request(
    addr: &str,
    request: HandleWorkloadRequest,
) -> Result<HandleWorkloadResponse, Status> {
    let mut client = NodeAgentConnectionClient::connect(connect_server(&addr))
        .await
        .unwrap();

    let response = client
        .handle_workload(Request::new(request))
        .await?
        .into_inner();
    Ok(response)
}

/// Send a runtime resource update request to the NodeAgent on `node_ip`.
///
/// Part of the Dynamic Resource Scaling workflow (#514 / #526).
pub async fn send_update_resources(
    node_ip: &str,
    request: UpdateResourcesRequest,
) -> Result<UpdateResourcesResponse, Status> {
    let mut client = NodeAgentConnectionClient::connect(connect_server(&node_ip))
        .await
        .map_err(|e| Status::unavailable(format!("NodeAgent connect failed: {}", e)))?;

    let response = client
        .update_resources(Request::new(request))
        .await?
        .into_inner();
    Ok(response)
}

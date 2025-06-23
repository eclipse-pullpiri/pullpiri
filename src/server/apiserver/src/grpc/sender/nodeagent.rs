use common::nodeagent::{
    connect_server, node_agent_connection_client::NodeAgentConnectionClient, HandleWorkloadRequest,
    HandleWorkloadResponse,
};
use tonic::{Request, Response, Status};

pub async fn send(
    action: HandleWorkloadRequest,
) -> Result<Response<HandleWorkloadResponse>, Status> {
    let mut client: NodeAgentConnectionClient<tonic::transport::Channel> =
        NodeAgentConnectionClient::connect(connect_server())
            .await
            .unwrap();
    client.handle_workload(Request::new(action)).await
}

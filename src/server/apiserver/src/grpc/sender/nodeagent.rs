use common::nodeagent::{
    connect_guest_server, connect_server, node_agent_connection_client::NodeAgentConnectionClient,
    HandleYamlRequest, HandleYamlResponse,
};
use tonic::{Request, Response, Status};

pub async fn send(action: HandleYamlRequest) -> Result<Response<HandleYamlResponse>, Status> {
    let mut client: NodeAgentConnectionClient<tonic::transport::Channel> =
        NodeAgentConnectionClient::connect(connect_server())
            .await
            .unwrap();
    client.handle_yaml(Request::new(action)).await
}

pub async fn send_guest(action: HandleYamlRequest) -> Result<Response<HandleYamlResponse>, Status> {
    let mut client: NodeAgentConnectionClient<tonic::transport::Channel> =
        NodeAgentConnectionClient::connect(connect_guest_server())
            .await
            .unwrap();
    client.handle_yaml(Request::new(action)).await
}

use common::nodeagent::node_agent_connection_server::NodeAgentConnection;
use common::nodeagent::{HandleYamlRequest, HandleYamlResponse};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// NodeAgent gRPC service handler
#[derive(Clone)]
pub struct NodeAgentReceiver {
    pub tx: mpsc::Sender<HandleYamlRequest>,
}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentReceiver {
    /// Handle a yaml request from API-Server
    ///
    /// Receives a yaml from API-Server and forwards it to the NodeAgent manager for processing.
    async fn handle_yaml<'life>(
        &'life self,
        request: Request<HandleYamlRequest>,
    ) -> Result<Response<HandleYamlResponse>, Status> {
        println!("Got a Yamlrequest from api-server");
        let req: HandleYamlRequest = request.into_inner();

        match self.tx.send(req).await {
            Ok(_) => Ok(tonic::Response::new(HandleYamlResponse {
                status: true,
                desc: "Successfully processed YAML".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send condition: {}", e),
            )),
        }
    }
}

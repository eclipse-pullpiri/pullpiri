use common::monitoringclient::monitoring_client_connection_server::MonitoringClientConnection;
use common::monitoringclient::{ContainerList, ImageList, McResponse, PodList};
use common::nodeagent::node_agent_connection_server::NodeAgentConnection;
use common::nodeagent::{HandleYamlRequest, HandleYamlResponse};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// MonitoringClient gRPC service handler
pub struct MonitoringClientGrpcServer {}

#[tonic::async_trait]
impl MonitoringClientConnection for MonitoringClientGrpcServer {
    async fn send_image_list(
        &self,
        request: tonic::Request<ImageList>,
    ) -> Result<tonic::Response<McResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.node_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }

    async fn send_container_list(
        &self,
        request: tonic::Request<ContainerList>,
    ) -> Result<tonic::Response<McResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.node_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }

    async fn send_pod_list(
        &self,
        request: tonic::Request<PodList>,
    ) -> Result<tonic::Response<McResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.node_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}

/// NodeAgent gRPC service handler
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
            Err(_) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                "cannot send condition",
            )),
        }
    }
}

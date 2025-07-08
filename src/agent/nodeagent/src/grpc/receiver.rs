use std::sync::Arc;

use crate::manager::NodeAgentParameter;
use common::monitoringclient::monitoring_client_connection_server::MonitoringClientConnection;
use common::monitoringclient::{ContainerList, ImageList, McResponse, PodList};
use common::nodeagent::node_agent_connection_server::{
    NodeAgentConnection, NodeAgentConnectionServer,
};
use common::nodeagent::{HandleWorkloadRequest, HandleWorkloadResponse};
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
    tx: mpsc::Sender<NodeAgentParameter>,
}

impl NodeAgentReceiver {
    /// Create a new NodeAgentReceiver
    ///
    /// # Arguments
    /// * `tx` - Channel sender for NodeAgentParameter information
    pub fn new(tx: mpsc::Sender<NodeAgentParameter>) -> Self {
        Self { tx }
    }

    /// Get the gRPC server for this receiver
    pub fn into_service(self) -> NodeAgentConnectionServer<Self> {
        NodeAgentConnectionServer::new(self)
    }

    /// Handle a workload request from API-Server
    ///
    /// Receives a workload name from API-Server and forwards it to the NodeAgent manager for processing.
    pub async fn handle_workload(&self, workload_name: String) -> Result<(), std::io::Error> {
        // Try to parse the scenario YAML string into a Scenario struct
        let scenario = match serde_yaml::from_str(&workload_name) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to parse scenario: {}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Failed to parse scenario YAML",
                ));
            }
        };
        let param = NodeAgentParameter {
            action: 0, // You may want to parse this from the request
            scenario,
        };
        self.tx.send(param).await.map_err(|e| {
            eprintln!("Failed to send workload: {}", e);
            std::io::Error::new(std::io::ErrorKind::Other, "Failed to send workload")
        })?;
        Ok(())
    }
}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentReceiver {
    async fn handle_workload(
        &self,
        request: Request<HandleWorkloadRequest>,
    ) -> Result<Response<HandleWorkloadResponse>, Status> {
        let req = request.into_inner();
        println!("Received workload handling request");
        match self.handle_workload(req.workload_name).await {
            Ok(_) => {
                println!("Successfully handled workload");
            }
            Err(e) => {
                eprintln!("Error handling workload: {}", e);
                return Err(Status::internal(format!(
                    "Failed to handle workload: {}",
                    e
                )));
            }
        }
        Ok(Response::new(HandleWorkloadResponse {
            status: 0,
            desc: "Successfully handled workload".to_string(),
        }))
    }
}

use std::sync::Arc;

use common::monitoringclient::monitoring_client_connection_server::MonitoringClientConnection;
use common::monitoringclient::{ContainerList, ImageList, PodList, Response};
use common::nodeagent::node_agent_connection_server::{
    NodeAgentConnection, NodeAgentConnectionServer,
};
use common::nodeagent::{HandleWorkloadRequest, HandleWorkloadResponse};

pub struct MonitoringClientGrpcServer {}

#[tonic::async_trait]
impl MonitoringClientConnection for MonitoringClientGrpcServer {
    async fn send_image_list(
        &self,
        request: tonic::Request<ImageList>,
    ) -> Result<tonic::Response<Response>, tonic::Status> {
        let req = request.into_inner();
        let command = req.node_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }

    async fn send_container_list(
        &self,
        request: tonic::Request<ContainerList>,
    ) -> Result<tonic::Response<Response>, tonic::Status> {
        let req = request.into_inner();
        let command = req.node_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }

    async fn send_pod_list(
        &self,
        request: tonic::Request<PodList>,
    ) -> Result<tonic::Response<Response>, tonic::Status> {
        let req = request.into_inner();
        let command = req.node_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}

pub struct NodeAgentReceiver {
    manager: Arc<crate::manager::NodeAgentManager>,
}

impl NodeAgentReceiver {
    pub fn new(manager: Arc<crate::manager::NodeAgentManager>) -> Self {
        Self { manager }
    }

    pub fn into_service(self) -> NodeAgentConnectionServer<Self> {
        NodeAgentConnectionServer::new(self)
    }
}

#[tonic::async_trait]
impl NodeAgentConnection for NodeAgentReceiver {
    async fn handle_workload(
        &self,
        request: tonic::Request<HandleWorkloadRequest>,
    ) -> Result<tonic::Response<HandleWorkloadResponse>, tonic::Status> {
        let req = request.into_inner();
        println!(
            "workload_name in node_agent gprc receiver : {}",
            req.workload_name
        );

        match self.manager.handle_workload(&req.workload_name).await {
            Ok(_) => Ok(tonic::Response::new(HandleWorkloadResponse {
                status: 0, // Success
                desc: "Action triggered successfully".to_string(),
            })),
            Err(e) => Ok(tonic::Response::new(HandleWorkloadResponse {
                status: 1, // Error
                desc: format!("Failed to trigger action: {}", e),
            })),
        }
    }
}

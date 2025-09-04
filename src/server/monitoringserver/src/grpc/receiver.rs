use common::monitoringserver::monitoring_server_connection_server::MonitoringServerConnection;
use common::monitoringserver::{
    ContainerList, NodeInfo, SendContainerListResponse, SendNodeInfoResponse,
};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// MonitoringServer gRPC service handler
#[derive(Clone)]
pub struct MonitoringServerReceiver {
    pub tx_container: mpsc::Sender<ContainerList>,
    pub tx_node: mpsc::Sender<NodeInfo>,
}

#[tonic::async_trait]
impl MonitoringServerConnection for MonitoringServerReceiver {
    /// Handle a ContainerList message from nodeagent
    ///
    /// Receives a ContainerList from nodeagent and forwards it to the MonitoringServer manager for processing.
    async fn send_container_list<'life>(
        &'life self,
        request: Request<ContainerList>,
    ) -> Result<Response<SendContainerListResponse>, Status> {
        let req: ContainerList = request.into_inner();

        match self.tx_container.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendContainerListResponse {
                resp: "Successfully processed ContainerList".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send container list: {}", e),
            )),
        }
    }

    /// Handle a NodeInfo message from nodeagent
    ///
    /// Receives a NodeInfo from nodeagent and forwards it to the MonitoringServer manager for processing.
    async fn send_node_info<'life>(
        &'life self,
        request: Request<NodeInfo>,
    ) -> Result<Response<SendNodeInfoResponse>, Status> {
        let req: NodeInfo = request.into_inner();

        match self.tx_node.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendNodeInfoResponse {
                resp: "Successfully processed NodeInfo".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send node info: {}", e),
            )),
        }
    }
}

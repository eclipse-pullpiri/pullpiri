use common::monitoringserver::monitoring_server_connection_server::MonitoringServerConnection;
use common::monitoringserver::{ContainerList, SendContainerListResponse};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// MonitoringServer gRPC service handler
#[derive(Clone)]
pub struct MonitoringServerReceiver {
    pub tx: mpsc::Sender<ContainerList>,
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
        println!("Got a ContainerList from nodeagent");
        let req: ContainerList = request.into_inner();

        match self.tx.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendContainerListResponse {
                resp: "Successfully processed ContainerList".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send container list: {}", e),
            )),
        }
    }
}

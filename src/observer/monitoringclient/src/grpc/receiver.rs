use common::monitoringclient::monitoring_client_connection_server::MonitoringClientConnection;
use common::monitoringclient::{ContainerList, ImageList, PodList, Response};

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

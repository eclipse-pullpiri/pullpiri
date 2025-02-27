use common::actioncontroller::action_controller_connection_server::ActionControllerConnection;
use common::actioncontroller::{
    ReconcileRequest, ReconcileResponse, TriggerActionRequest, TriggerActionResponse,
};

pub struct ActionControllerGrpcServer {}

#[tonic::async_trait]
impl ActionControllerConnection for ActionControllerGrpcServer {
    async fn trigger_action(
        &self,
        request: tonic::Request<TriggerActionRequest>,
    ) -> Result<tonic::Response<TriggerActionResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.scenario_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }

    async fn reconcile(
        &self,
        request: tonic::Request<ReconcileRequest>,
    ) -> Result<tonic::Response<ReconcileResponse>, tonic::Status> {
        let req = request.into_inner();
        let command = req.scenario_name;

        Err(tonic::Status::new(tonic::Code::Unavailable, command))
    }
}

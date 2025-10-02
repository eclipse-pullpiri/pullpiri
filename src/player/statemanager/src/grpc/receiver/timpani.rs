use common::external::fault_service_server::FaultService;
use common::external::{FaultInfo, Response as TimpaniResponse};
use tonic::{Request, Response, Status};

#[derive(Default)]
pub struct TimpaniReceiver {}

#[tonic::async_trait]
impl FaultService for TimpaniReceiver {
    async fn notify_fault(
        &self,
        info: Request<FaultInfo>,
    ) -> Result<Response<TimpaniResponse>, Status> {
        let info = info.into_inner();
        println!("Received fault notification: {:?}", info);

        // Process the fault information and generate a response
        let response = TimpaniResponse { status: 0 };
        Ok(Response::new(response))
    }
}

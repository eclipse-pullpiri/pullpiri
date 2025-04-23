use common::spec::artifact::Scenario;
use common::Result;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

// Import the generated protobuf code from filtergateway.proto
use common::filtergateway::{
    filter_gateway_connection_server::{FilterGatewayConnection, FilterGatewayConnectionServer},
    HandleScenarioRequest, HandleScenarioResponse,
};

/// FilterGateway gRPC service handler
pub struct FilterGatewayReceiver {
    tx: mpsc::Sender<Scenario>,
}

impl FilterGatewayReceiver {
    /// Create a new FilterGatewayReceiver
    ///
    /// # Arguments
    ///
    /// * `tx` - Channel sender for scenario information
    ///
    /// # Returns
    ///
    /// A new FilterGatewayReceiver instance
    pub fn new(tx: mpsc::Sender<Scenario>) -> Self {
        Self { tx }
    }

    /// Get the gRPC server for this receiver
    ///
    /// # Returns
    ///
    /// A gRPC server for handling requests
    pub fn into_service(self) -> FilterGatewayConnectionServer<Self> {
        FilterGatewayConnectionServer::new(self)
    }

    /// Handle a scenario from API-Server
    ///
    /// Receives a scenario YAML string from API-Server, parses it into a Scenario struct,
    /// and forwards it to the FilterGateway manager for processing.
    ///
    /// # Arguments
    ///
    /// * `scenario_yaml_str` - YAML string of the scenario
    /// * `action` - Action code (0 for APPLY, 1 for WITHDRAW)
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn handle_scenario(&self, scenario_yaml_str: String, action: i32) -> Result<()> {
        let _ = (scenario_yaml_str, action); // 사용하지 않는 변수 경고 방지
                                             // TODO: Implementation
        Ok(())
    }
}

#[tonic::async_trait]
impl FilterGatewayConnection for FilterGatewayReceiver {
    async fn handle_scenario(
        &self,
        request: Request<HandleScenarioRequest>,
    ) -> std::result::Result<Response<HandleScenarioResponse>, Status> {
        let _ = request; // 사용하지 않는 변수 경고 방지
                         // TODO: Implementation
        Ok(Response::new(HandleScenarioResponse {
            status: true,
            desc: "Successfully handled scenario".to_string(),
        }))
    }
}

use core::sync;
use std::io::Error;

use crate::manager::ScenarioParameter;
use crate::vehicle::dds::DdsData;


use common::spec::artifact::{Artifact, Scenario};
use common::Result;
use tokio::sync::mpsc::{self, error::SendError};
use tonic::{Request, Response, Status};

// Import the generated protobuf code from filtergateway.proto
use common::filtergateway::{
    filter_gateway_connection_server::{FilterGatewayConnection, FilterGatewayConnectionServer},
    HandleScenarioRequest, HandleScenarioResponse,
};

/// FilterGateway gRPC service handler
pub struct FilterGatewayReceiver {
    tx: mpsc::Sender<ScenarioParameter>
}

impl FilterGatewayReceiver {
    /// Create a new FilterGatewayReceiver
    ///
    /// # Arguments
    ///
    /// * `tx` - Channel sender for ScenarioParameter information
    ///
    /// # Returns
    ///
    /// A new FilterGatewayReceiver instance
    pub fn new(tx: mpsc::Sender<ScenarioParameter>) -> Self {
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
        // Parse the scenario YAML string into a Scenario struct
        let scenario = serde_yaml::from_str::<Scenario>(&scenario_yaml_str)?;

        let param = ScenarioParameter {
            action: action,
            scenario: scenario,
        };
        
        self.tx.send(param).await.map_err(|e| {
            eprintln!("Failed to send scenario: {}", e);
            Error::new(std::io::ErrorKind::Other, "Failed to send scenario")
        })?;
        

        Ok(())
    }
}

#[tonic::async_trait]
impl FilterGatewayConnection for FilterGatewayReceiver {
    async fn handle_scenario(
        &self,
        request: Request<HandleScenarioRequest>,
    ) -> std::result::Result<Response<HandleScenarioResponse>, Status> {
        let req = request.into_inner();
        println!("Received scenario handling request");

        // Extract the scenario YAML string and action from the request
        match self.handle_scenario(req.scenario, req.action).await {
            Ok(_) => {
                println!("Successfully handled scenario");
            }
            Err(e) => {
                eprintln!("Error handling scenario: {}", e);
                return Err(Status::internal(format!(
                    "Failed to handle scenario: {}",
                    e
                )));
            }
        }
        Ok(Response::new(HandleScenarioResponse {
            status: true,
            desc: "Successfully handled scenario".to_string(),
        }))
    }
}

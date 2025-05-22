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
//Unit Test Cases
#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;
    use crate::grpc::receiver::FilterGatewayReceiver;
    use serde_yaml;

    // Test case for handling valid YAML input
    #[tokio::test]
    async fn test_handle_scenario_with_valid_yaml() {
        let (tx, mut rx) = mpsc::channel(1);
        let receiver = FilterGatewayReceiver::new(tx);

        let scenario_yaml = r#"
        apiVersion: v1
        kind: Scenario
        metadata:
          name: helloworld
        spec:
          condition:
          action: update
          target: helloworld
        "#;

        let action = 0;

        let result = receiver.handle_scenario(scenario_yaml.to_string(), action).await;
        assert!(result.is_ok());

        let received_param = rx.recv().await.unwrap();
        assert_eq!(received_param.action, action);

        let scenario: serde_yaml::Value = serde_yaml::from_str(&scenario_yaml).unwrap();
        assert_eq!(scenario["metadata"]["name"], "helloworld");
        assert_eq!(scenario["spec"]["action"], "update");
        assert_eq!(scenario["spec"]["target"], "helloworld");
    }

    // Test case for handling invalid YAML input
    #[tokio::test]
    async fn test_handle_scenario_with_invalid_yaml() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = FilterGatewayReceiver::new(tx);

        let invalid_yaml = r#"
        apiVersion: v1
        kind: Scenario
        metadata:
          name: helloworld
        spec:
          condition:
          action: update
          target: helloworld
        ---
        apiVersion: v1
        kind: Package
        metadata:
          label: null
          name: helloworld
        spec:
          pattern:
            - type: plain
          models:
            - name: helloworld-core
              node: HPC
              resources:
                volume:
                network:
        "#; // Invalid YAML due to missing resource definitions

        let action = 0;

        let result = receiver.handle_scenario(invalid_yaml.to_string(), action).await;
        assert!(result.is_err());
    }

    // Test case for handling empty YAML input
    #[tokio::test]
    async fn test_handle_scenario_with_empty_yaml() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = FilterGatewayReceiver::new(tx);

        let empty_yaml = "";

        let action = 0;

        let result = receiver.handle_scenario(empty_yaml.to_string(), action).await;
        assert!(result.is_err());
    }

    // Test case for handling YAML with missing required fields
    #[tokio::test]
    async fn test_handle_scenario_with_missing_fields() {
        let (tx, _rx) = mpsc::channel(1);
        let receiver = FilterGatewayReceiver::new(tx);

        let incomplete_yaml = r#"
        apiVersion: v1
        kind: Scenario
        metadata:
          name: helloworld
        spec:
          action: update
        "#; // Missing "target" field

        let action = 0;

        let result = receiver.handle_scenario(incomplete_yaml.to_string(), action).await;
        assert!(result.is_err());
    }

    // Negative test case for handling a scenario when the channel is closed
    #[tokio::test]
    async fn test_handle_scenario_with_closed_channel() {
        let (tx, _rx) = mpsc::channel(1); // Use a buffer size greater than 0
        drop(tx.clone()); // Explicitly close the channel
        let receiver = FilterGatewayReceiver::new(tx);

        let scenario_yaml = r#"
        apiVersion: v1
        kind: Scenario
        metadata:
          name: helloworld
        spec:
          condition:
          action: update
          target: helloworld
        "#;

        let action = 0;

        let result = receiver.handle_scenario(scenario_yaml.to_string(), action).await;
        assert!(result.is_ok());
    }
}
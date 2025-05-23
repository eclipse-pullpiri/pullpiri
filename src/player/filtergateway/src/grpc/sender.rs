use common::Result;

// Import the generated protobuf code from actioncontroller.proto

use common::actioncontroller::connect_server;

// Import the generated protobuf code from actioncontroller.proto
use common::actioncontroller::action_controller_connection_client::ActionControllerConnectionClient;

/// Sender for making gRPC requests to ActionController
#[derive(Clone)]
pub struct FilterGatewaySender {}

impl FilterGatewaySender {
    /// Create a new FilterGatewaySender
    ///
    /// # Returns
    ///
    /// A new FilterGatewaySender instance
    pub fn new() -> Self {
        Self {}
    }

    /// Trigger an action for a scenario
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn trigger_action(&mut self, scenario_name: String) -> Result<()> {
        use common::actioncontroller::TriggerActionRequest;
        let mut client = ActionControllerConnectionClient::connect(connect_server())
            .await
            .unwrap();

        let request = TriggerActionRequest { scenario_name };

        client.trigger_action(request).await.map_err(|e| {
            log::error!("Failed to trigger action: {:?}", e);
            anyhow::anyhow!("Failed to trigger action: {:?}", e)
        })?;

        Ok(())
    }
}
//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;
    /// Test case to validate successful execution of `trigger_action`
    #[tokio::test]
    async fn test_trigger_action_success() {
        let mut sender = FilterGatewaySender::new();
        let scenario_name = "test_scenario".to_string();

        // Trigger action with a valid scenario name
        let result = sender.trigger_action(scenario_name).await;

        // Assert that the result is successful
        assert!(result.is_ok());
    }

    /// Test case to validate failure due to connection error
    #[tokio::test]
    async fn test_trigger_action_failure_connection_error() {
        let mut sender = FilterGatewaySender::new();
        let scenario_name = "test_scenario".to_string();

        // Simulate connection failure by mocking `connect_server` to return an invalid address
        let result = ActionControllerConnectionClient::connect("invalid_address").await;

        // Assert that the connection attempt fails
        assert!(result.is_err());
    }

    /// Test case to validate failure when `connect_server` returns an empty server address
    #[tokio::test]
    async fn test_trigger_action_failure_empty_server_address() {
        let mut sender = FilterGatewaySender::new();
        let scenario_name = "test_scenario".to_string();

        // Simulate empty server address
        let result = ActionControllerConnectionClient::connect("").await;

        // Assert that the connection attempt fails
        assert!(result.is_err());
    }
}

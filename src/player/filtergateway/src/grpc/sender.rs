use common::Result;


// Import the generated protobuf code from actioncontroller.proto

use common::actioncontroller::connect_server;


// Import the generated protobuf code from actioncontroller.proto
use common::actioncontroller::action_controller_connection_client::ActionControllerConnectionClient;

/// Sender for making gRPC requests to ActionController
#[derive(Clone)]
pub struct FilterGatewaySender {
}

impl FilterGatewaySender {
    /// Create a new FilterGatewaySender
    ///
    /// # Returns
    ///
    /// A new FilterGatewaySender instance
    pub fn new() -> Self {
        Self {
        }
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
        let mut client 
        = ActionControllerConnectionClient::connect(connect_server()).await.unwrap();

        let request = TriggerActionRequest {
            scenario_name
        };

        client.trigger_action(request).await.map_err(|e| {
            log::error!("Failed to trigger action: {:?}", e);
            anyhow::anyhow!("Failed to trigger action: {:?}", e)
        })?;

        Ok(())
    }
}

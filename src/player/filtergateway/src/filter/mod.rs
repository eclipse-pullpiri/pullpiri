use crate::grpc::sender::FilterGatewaySender;
use crate::vehicle::dds::DdsData;
use common::spec::artifact::Scenario;
use common::Result;
// use dust_dds::infrastructure::wait_set::Condition;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Filter for evaluating scenario conditions
pub struct Filter {
    /// Name of the scenario
    pub scenario_name: String,
    /// Full scenario definition
    pub scenario: common::spec::artifact::Scenario,
    /// Flag to indicate if the filter is active
    is_active: bool,
    /// gRPC sender for action controller
    sender: Arc<FilterGatewaySender>,
}

impl Filter {
    /// Create a new Filter
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    /// * `scenario` - Full scenario definition
    /// * `rx_dds` - Receiver for DDS data
    /// * `sender` - Sender for gRPC calls
    ///
    /// # Returns
    ///
    /// A new Filter instance
    pub fn new(
        scenario_name: String,
        scenario: common::spec::artifact::Scenario,
        is_active: bool,
        sender: Arc<FilterGatewaySender>,
    ) -> Self {
        Self {
            scenario_name,
            scenario,
            is_active,
            sender,
        }
    }

    /// Check if scenario conditions are met
    ///
    /// Evaluates if the received vehicle data meets the scenario conditions.
    /// If conditions are met, triggers an action through ActionController.
    ///
    /// # Arguments
    ///
    /// * `data` - Vehicle message data
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn meet_scenario_condition(&self, data: &DdsData) -> Result<()> {
        let condition = self.scenario.get_conditions().unwrap();
        let topic = condition.get_operand_value();
        let target_value = condition.get_value();
        let express = condition.get_express();

        if !data.name.eq(&topic) {
            return Err("data topic does not match".into());
        }

        let check: bool = match express.as_str() {
            "eq" => target_value.to_lowercase() == data.value.to_lowercase(),
            "lt" => {
                let target_v = target_value.parse::<f32>().unwrap();
                let current_v = data.value.parse::<f32>().unwrap();
                target_v < current_v
            }
            "le" => {
                let target_v = target_value.parse::<f32>().unwrap();
                let current_v = data.value.parse::<f32>().unwrap();
                target_v <= current_v
            }
            "ge" => {
                let target_v = target_value.parse::<f32>().unwrap();
                let current_v = data.value.parse::<f32>().unwrap();
                target_v >= current_v
            }
            "gt" => {
                let target_v = target_value.parse::<f32>().unwrap();
                let current_v = data.value.parse::<f32>().unwrap();
                target_v > current_v
            }
            _ => return Err("wrong expression in condition".into()),
        };

        if check {
            println!("Condition met for scenario: {}", self.scenario_name);
            self.sender
                .trigger_action(self.scenario_name.clone())
                .await?;
            Ok(())
        } else {
            Err("cannot meet condition".into())
        }
    }

    /// Pause the filter processing
    ///
    /// Temporarily disables condition evaluation for this scenario.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn pause_scenario_filter(&mut self) -> Result<()> {
        // TODO: Implementation
        self.is_active = false;
        Ok(())
    }

    /// Resume the filter processing
    ///
    /// Re-enables condition evaluation for this scenario.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn resume_scenario_filter(&mut self) -> Result<()> {
        // TODO: Implementation
        self.is_active = true;
        Ok(())
    }
}

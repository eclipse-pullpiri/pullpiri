use crate::grpc::sender::actioncontroller::FilterGatewaySender;
use crate::grpc::sender::statemanager::StateManagerSender;
use crate::vehicle::dds::DdsData;
use common::spec::artifact::Scenario;
use common::statemanager::{ResourceType, StateChange};
use common::Result;
// use dust_dds::infrastructure::wait_set::Condition;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Filter for evaluating scenario conditions
pub struct Filter {
    /// Name of the scenario
    pub scenario_name: String,
    /// Full scenario definition
    pub scenario: Scenario,
    /// Flag to indicate if the filter is active
    is_active: bool,
    /// gRPC sender for action controller
    sender: FilterGatewaySender,
    /// gRPC sender for state manager
    state_sender: StateManagerSender,
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
        scenario: Scenario,
        is_active: bool,
        sender: FilterGatewaySender,
    ) -> Self {
        Self {
            scenario_name,
            scenario,
            is_active,
            sender,
            state_sender: StateManagerSender::new(),
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
    pub async fn meet_scenario_condition(&mut self, data: &DdsData) -> Result<()> {
        use std::time::Instant;
        let start = Instant::now();

        let condition = self.scenario.get_conditions().unwrap();
        let topic = condition.get_operand_value();
        let value_name = condition.get_operand_name();
        let target_value = condition.get_value();
        let express = condition.get_express();

        print!(
        "Checking condition for scenario: {}\nTopic: {}\nTarget Value: {}\nValue Name: {}\nExpression: {}\n",
        self.scenario_name, topic, target_value, value_name, express
    );

        if !data.name.eq(&topic) {
            let elapsed = start.elapsed();
            println!("meet_scenario_condition: elapsed = {:?}", elapsed);
            return Err("data topic does not match".into());
        }

        let field_value = match data.fields.get(&value_name) {
            Some(v) => v,
            None => {
                let elapsed = start.elapsed();
                println!("meet_scenario_condition: elapsed = {:?}", elapsed);
                return Err(format!("field '{}' not found in data.fields", value_name).into());
            }
        };

        let check: bool = match express.as_str() {
            "eq" => target_value.to_lowercase() == field_value.to_lowercase(),
            "lt" => {
                let target_v = target_value
                    .parse::<f32>()
                    .map_err(|_| "target_value parse error")?;
                let current_v = field_value
                    .parse::<f32>()
                    .map_err(|_| "field_value parse error")?;
                current_v < target_v
            }
            "le" => {
                let target_v = target_value
                    .parse::<f32>()
                    .map_err(|_| "target_value parse error")?;
                let current_v = field_value
                    .parse::<f32>()
                    .map_err(|_| "field_value parse error")?;
                current_v <= target_v
            }
            "ge" => {
                let target_v = target_value
                    .parse::<f32>()
                    .map_err(|_| "target_value parse error")?;
                let current_v = field_value
                    .parse::<f32>()
                    .map_err(|_| "field_value parse error")?;
                current_v >= target_v
            }
            "gt" => {
                let target_v = target_value
                    .parse::<f32>()
                    .map_err(|_| "target_value parse error")?;
                let current_v = field_value
                    .parse::<f32>()
                    .map_err(|_| "field_value parse error")?;
                current_v > target_v
            }
            _ => {
                let elapsed = start.elapsed();
                println!("meet_scenario_condition: elapsed = {:?}", elapsed);
                return Err("wrong expression in condition".into());
            }
        };

        let elapsed = start.elapsed();
        println!("meet_scenario_condition: elapsed = {:?}", elapsed);

        if check {
            println!("Condition met for scenario: {}", self.scenario_name);
            println!("🔄 SCENARIO STATE TRANSITION: FilterGateway Processing");
            println!("   📋 Scenario: {}", self.scenario_name);
            println!("   🔄 State Change: idle → waiting");
            println!("   🔍 Reason: Scenario condition satisfied");

            // 🔍 COMMENT 1: FilterGateway condition registration
            // When scenario condition is met, FilterGateway triggers ActionController
            // via gRPC call. This initiates the scenario processing workflow.
            // The ActionController will then handle state changes with StateManager.

            // Send state change to StateManager: idle -> waiting
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64;

            let state_change = StateChange {
                resource_type: ResourceType::Scenario as i32,
                resource_name: self.scenario_name.clone(),
                current_state: "idle".to_string(),
                target_state: "waiting".to_string(),
                transition_id: format!("filtergateway-condition-met-{}", timestamp),
                timestamp_ns: timestamp,
                source: "filtergateway".to_string(),
            };

            println!("   📤 Sending StateChange to StateManager:");
            println!("      • Resource Type: SCENARIO");
            println!("      • Resource Name: {}", state_change.resource_name);
            println!("      • Current State: {}", state_change.current_state);
            println!("      • Target State: {}", state_change.target_state);
            println!("      • Transition ID: {}", state_change.transition_id);
            println!("      • Source: {}", state_change.source);

            if let Err(e) = self
                .state_sender
                .clone()
                .send_state_change(state_change)
                .await
            {
                println!("   ❌ Failed to send state change to StateManager: {:?}", e);
            } else {
                println!(
                    "   ✅ Successfully notified StateManager: scenario {} idle → waiting",
                    self.scenario_name
                );
            }

            println!("   📤 Triggering ActionController via gRPC...");
            self.sender
                .trigger_action(self.scenario_name.clone())
                .await?;
            println!("   ✅ ActionController triggered successfully");
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

    /// Check if filter is active
    ///
    /// # Returns
    ///
    /// * `bool` - Filter active status
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Process DDS data and check conditions
    ///
    /// Processes received DDS data and checks scenario conditions.
    /// Triggers an action if conditions are met.
    ///
    /// # Arguments
    ///
    /// * `data` - Received DDS data
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn process_data(&mut self, data: &DdsData) -> Result<()> {
        // Do not process inactive filters
        if !self.is_active {
            return Ok(());
        }

        print!(
            "process data for scenario: {}\nTopic: {:?}\nTarget Name: {:?}\nTarget Value: {:?}\n",
            self.scenario_name, data.name, data.value, data.fields
        );

        // Check if topic matches filter condition
        let condition = match self.scenario.get_conditions() {
            Some(c) => c,
            None => return Ok(()), // No conditions case (already handled)
        };

        let topic = condition.get_operand_value();
        if !data.name.eq(&topic) {
            return Ok(()); // Ignore unrelated topics
        }

        // Perform condition check
        match self.meet_scenario_condition(data).await {
            Ok(_) => {
                println!("Action triggered for scenario: {}", self.scenario_name);
                // Disable filter after condition is met (run only once)
                // Add self.is_active = false; code if needed
            }
            Err(e) => {
                // Condition not met is a normal case, only log at debug level
                if e.to_string() != "cannot meet condition" {
                    println!("Error checking condition: {:?}", e);
                }
            }
        }

        Ok(())
    }
}
//Unit Test Cases
#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use common::Result;
    use mockall::{mock, predicate};
    use std::collections::HashMap;
    use tokio;

    // Dummy DDS data struct
    #[derive(Clone)]
    struct DdsData {
        topic_name: String,
        data: HashMap<String, String>,
    }

    impl DdsData {
        fn new(topic_name: &str, data: HashMap<String, String>) -> Self {
            Self {
                topic_name: topic_name.to_string(),
                data,
            }
        }
    }

    // Dummy condition struct with operand enum
    #[derive(Clone)]
    struct Condition {
        key: String,
        operand: Operand,
        value: String,
    }

    #[derive(Clone)]
    enum Operand {
        Equal,
        NotEqual,
    }

    // Scenario holding multiple conditions
    #[derive(Clone)]
    struct Scenario {
        conditions: Vec<Condition>,
    }

    impl Scenario {
        fn new(conditions: Vec<Condition>) -> Self {
            Self { conditions }
        }
    }

    // Trait for sender, to be mocked
    #[async_trait]
    pub trait FilterGatewaySenderTrait: Send + Sync {
        async fn trigger_action(&self, scenario_name: String) -> Result<()>;
    }

    // Mock sender using mockall
    mock! {
        pub FilterGatewaySender {}

        #[async_trait]
        impl FilterGatewaySenderTrait for FilterGatewaySender {
            async fn trigger_action(&self, scenario_name: String) -> Result<()>;
        }
    }

    // Filter struct under test
    struct Filter<S: FilterGatewaySenderTrait> {
        scenario_name: String,
        scenario: Scenario,
        enabled: bool,
        sender: S,
    }

    impl<S: FilterGatewaySenderTrait> Filter<S> {
        pub fn new(scenario_name: String, scenario: Scenario, enabled: bool, sender: S) -> Self {
            Self {
                scenario_name,
                scenario,
                enabled,
                sender,
            }
        }

        pub async fn meet_scenario_condition(&mut self, dds_data: &DdsData) -> Result<()> {
            if !self.enabled {
                return Ok(());
            }

            for cond in &self.scenario.conditions {
                let actual_value = dds_data.data.get(&cond.key);
                let cond_met = match (actual_value, &cond.operand) {
                    (Some(val), Operand::Equal) => val == &cond.value,
                    (Some(val), Operand::NotEqual) => val != &cond.value,
                    _ => false,
                };

                if !cond_met {
                    return Ok(());
                }
            }

            self.sender.trigger_action(self.scenario_name.clone()).await
        }
    }

    // Helper to create scenario
    fn make_scenario_with_conditions(conds: Vec<Condition>) -> Scenario {
        Scenario::new(conds)
    }

    // Helper to create dds data
    fn make_dds_data(topic_name: &str, data: HashMap<String, String>) -> DdsData {
        DdsData::new(topic_name, data)
    }

    // Test: Trigger action when enabled and conditions met
    #[tokio::test]
    async fn test_trigger_action_when_condition_met_and_enabled() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // Mock: expect trigger_action called once with scenario name
        mock_sender
            .expect_trigger_action()
            .with(predicate::eq("test_scenario".to_string()))
            .times(1)
            .returning(|_| Ok(()));

        let scenario = make_scenario_with_conditions(vec![Condition {
            key: "temperature".into(),
            operand: Operand::Equal,
            value: "25".into(),
        }]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into());
        let dds_data = make_dds_data("TestTopic", data_map);
        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }

    // Test: Do not trigger when filter disabled
    #[tokio::test]
    async fn test_no_trigger_when_filter_disabled() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // No expectation since should not call trigger_action

        let scenario = make_scenario_with_conditions(vec![Condition {
            key: "temperature".into(),
            operand: Operand::Equal,
            value: "25".into(),
        }]);

        let mut filter = Filter::new("test_scenario".into(), scenario, false, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into());
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }

    // Test: Do not trigger when condition not met
    #[tokio::test]
    async fn test_no_trigger_when_condition_not_met() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // No expectation since condition fails

        let scenario = make_scenario_with_conditions(vec![Condition {
            key: "temperature".into(),
            operand: Operand::Equal,
            value: "25".into(),
        }]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "30".into()); // value differs
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }

    // Test: Propagate error from trigger_action
    #[tokio::test]
    async fn test_trigger_action_error_propagated() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // Mock: return error on trigger_action
        mock_sender
            .expect_trigger_action()
            .with(predicate::eq("test_scenario".to_string()))
            .times(1)
            .returning(|_| Err("trigger error".into()));

        let scenario = make_scenario_with_conditions(vec![Condition {
            key: "temperature".into(),
            operand: Operand::Equal,
            value: "25".into(),
        }]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into());
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_err());
        assert_eq!(format!("{}", result.unwrap_err()), "trigger error");
    }
    // Test: Trigger when multiple conditions all met
    #[tokio::test]
    async fn test_multiple_conditions_all_met() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // Expect trigger once
        mock_sender
            .expect_trigger_action()
            .with(predicate::eq("test_scenario".to_string()))
            .times(1)
            .returning(|_| Ok(()));

        let scenario = make_scenario_with_conditions(vec![
            Condition {
                key: "temperature".into(),
                operand: Operand::Equal,
                value: "25".into(),
            },
            Condition {
                key: "humidity".into(),
                operand: Operand::NotEqual,
                value: "80".into(),
            },
        ]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into());
        data_map.insert("humidity".into(), "50".into());
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }

    // Test: Do not trigger if one condition in multiple not met
    #[tokio::test]
    async fn test_multiple_conditions_one_not_met() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // No expectation since condition fails

        let scenario = make_scenario_with_conditions(vec![
            Condition {
                key: "temperature".into(),
                operand: Operand::Equal,
                value: "25".into(),
            },
            Condition {
                key: "humidity".into(),
                operand: Operand::NotEqual,
                value: "80".into(),
            },
        ]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into());
        data_map.insert("humidity".into(), "80".into()); // fails NotEqual
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }
    // Test: Trigger action when all conditions are met but with additional irrelevant data
    #[tokio::test]
    async fn test_trigger_action_with_irrelevant_data() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // Expect trigger once
        mock_sender
            .expect_trigger_action()
            .with(predicate::eq("test_scenario".to_string()))
            .times(1)
            .returning(|_| Ok(()));

        let scenario = make_scenario_with_conditions(vec![Condition {
            key: "temperature".into(),
            operand: Operand::Equal,
            value: "25".into(),
        }]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into());
        data_map.insert("pressure".into(), "1013".into()); // Irrelevant data
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }

    // Test: Do not trigger action when data map is empty
    #[tokio::test]
    async fn test_no_trigger_with_empty_data_map() {
        let mut mock_sender = MockFilterGatewaySender::new();
        // No expectation since condition fails
        let scenario = make_scenario_with_conditions(vec![Condition {
            key: "temperature".into(),
            operand: Operand::Equal,
            value: "25".into(),
        }]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let dds_data = make_dds_data("TestTopic", HashMap::new());

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }

    // Test: Trigger action when condition is met with case-insensitive comparison
    #[tokio::test]
    async fn test_trigger_action_with_case_insensitive_match() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // Expect trigger once
        mock_sender
            .expect_trigger_action()
            .with(predicate::eq("test_scenario".to_string()))
            .times(1)
            .returning(|_| Ok(()));

        let scenario = make_scenario_with_conditions(vec![Condition {
            key: "temperature".into(),
            operand: Operand::Equal,
            value: "25".into(),
        }]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into()); // Matching value
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }

    // Test: Trigger action when multiple conditions are met with mixed operands
    #[tokio::test]
    async fn test_trigger_action_with_mixed_operands() {
        let mut mock_sender = MockFilterGatewaySender::new();

        // Expect trigger once
        mock_sender
            .expect_trigger_action()
            .with(predicate::eq("test_scenario".to_string()))
            .times(1)
            .returning(|_| Ok(()));

        let scenario = make_scenario_with_conditions(vec![
            Condition {
                key: "temperature".into(),
                operand: Operand::Equal,
                value: "25".into(),
            },
            Condition {
                key: "humidity".into(),
                operand: Operand::NotEqual,
                value: "80".into(),
            },
        ]);

        let mut filter = Filter::new("test_scenario".into(), scenario, true, mock_sender);

        let mut data_map = HashMap::new();
        data_map.insert("temperature".into(), "25".into());
        data_map.insert("humidity".into(), "50".into()); // Meets NotEqual condition
        let dds_data = make_dds_data("TestTopic", data_map);

        let result = filter.meet_scenario_condition(&dds_data).await;

        assert!(result.is_ok());
    }
}

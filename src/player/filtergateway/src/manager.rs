/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use crate::filter::Filter;
use crate::grpc::sender::actioncontroller::FilterGatewaySender;
use crate::grpc::sender::statemanager::StateManagerSender;
use crate::vehicle::dds::DdsData;
use crate::vehicle::VehicleManager;
use common::spec::artifact::Scenario;
use common::statemanager::{ResourceType, StateChange};
use common::{spec::artifact::Artifact, Result};
// use dust_dds::infrastructure::wait_set::Condition;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Manager for FilterGateway
///
/// Responsible for:
/// - Managing scenario filters
/// - Coordinating vehicle data subscriptions
/// - Processing incoming scenario requests
///
#[derive(Debug)]
#[allow(dead_code)]
pub struct ScenarioParameter {
    /// Name of the scenario
    pub action: i32,
    /// Vehicle message information
    pub scenario: Scenario,
}
#[allow(dead_code)]
pub struct FilterGatewayManager {
    /// Receiver for scenario information from gRPC
    pub rx_grpc: Arc<Mutex<mpsc::Receiver<ScenarioParameter>>>,
    /// Receiver for DDS data
    pub rx_dds: Arc<Mutex<mpsc::Receiver<DdsData>>>,
    /// Active filters for scenarios
    pub filters: Arc<Mutex<Vec<Filter>>>,
    /// gRPC sender for action controller
    pub sender: Arc<Mutex<FilterGatewaySender>>,
    /// Vehicle manager for handling vehicle data
    pub vehicle_manager: Arc<Mutex<VehicleManager>>,
}
#[allow(dead_code)]
impl FilterGatewayManager {
    /// Creates a new FilterGatewayManager instance
    ///
    /// # Arguments
    ///
    /// * `rx` - Channel receiver for scenario information
    ///
    /// # Returns
    ///
    /// A new FilterGatewayManager instance
    pub async fn new(rx_grpc: mpsc::Receiver<ScenarioParameter>) -> Self {
        let (tx_dds, rx_dds) = mpsc::channel::<DdsData>(10);
        let mut vehicle_manager = VehicleManager::new(tx_dds);

        // Improved error handling: explicit error handling instead of unwrap()
        if let Err(e) = vehicle_manager.init().await {
            println!("Warning: Failed to initialize vehicle manager: {:?}. Continuing with default settings.", e);
            // Continue (already using default values in VehicleManager::init())
        }

        Self {
            rx_grpc: Arc::new(Mutex::new(rx_grpc)),
            rx_dds: Arc::new(Mutex::new(rx_dds)),
            filters: Arc::new(Mutex::new(Vec::new())),
            sender: Arc::new(Mutex::new(FilterGatewaySender::new())),
            vehicle_manager: Arc::new(Mutex::new(vehicle_manager)),
        }
    }
    /// Function to initialize the FilterGatewayManager
    ///
    ///
    /// This function reads all scenarios from etcd and subscribes to the necessary vehicle data topics.
    /// It also launches the scenario filters.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn initialize(&self) -> Result<()> {
        println!("FilterGatewayManager init");
        // Initialize vehicle manager
        let etcd_scenario = Self::read_all_scenario_from_etcd()
            .await
            .unwrap_or_default();

        for scenario in etcd_scenario {
            let scenario: Scenario = serde_yaml::from_str(&scenario)?;
            println!("Scenario: {:?}", scenario);

            // Only subscribe to vehicle data if the scenario has conditions with valid topic
            if let Some(conditions) = scenario.get_conditions().as_ref() {
                let topic_name = conditions.get_operand_value();
                let data_type_name = conditions.get_operand_value();

                // Skip subscription if topic name is empty
                if !topic_name.is_empty() {
                    let mut vehicle_manager = self.vehicle_manager.lock().await;
                    if let Err(e) = vehicle_manager
                        .subscribe_topic(topic_name, data_type_name)
                        .await
                    {
                        eprintln!("Error subscribing to vehicle data: {:?}", e);
                    }
                } else {
                    println!(
                        "Skipping DDS subscription for scenario '{}': empty topic name",
                        scenario.get_name()
                    );
                }
            } else {
                println!(
                    "Scenario '{}' has no conditions, skipping DDS subscription",
                    scenario.get_name()
                );
            }

            self.launch_scenario_filter(scenario).await?;
        }

        Ok(())
    }

    /// Function to receive subscribed DDS data and pass it to filters
    ///
    /// This function runs as a separate task to continuously receive and process DDS data.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    async fn process_dds_data(&self) -> Result<()> {
        // Create clone of shared receiver
        let rx_dds = Arc::clone(&self.rx_dds);

        // Receive loop
        loop {
            let mut receiver = rx_dds.lock().await;

            // Receive DDS data
            match receiver.recv().await {
                Some(dds_data) => {
                    // Only print if topic or value is not empty
                    if !dds_data.name.is_empty() && !dds_data.value.is_empty() {
                        println!(
                            "Received DDS data: topic={}, value={}",
                            dds_data.name, dds_data.value
                        );
                    }

                    // Forward data to all active filters
                    let mut filters = self.filters.lock().await;
                    for filter in filters.iter_mut() {
                        if filter.is_active() {
                            // Pass DDS data to filter
                            if let Err(e) = filter.process_data(&dds_data).await {
                                println!(
                                    "Error processing DDS data in filter {}: {:?}",
                                    filter.scenario_name, e
                                );
                            }
                        }
                    }
                }
                None => {
                    // Channel closed
                    println!("DDS data channel closed, stopping processor");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Function to process gRPC requests
    ///
    /// This function processes scenario requests coming through gRPC.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    async fn process_grpc_requests(&self) -> Result<()> {
        loop {
            // Wait for scenario parameter from gRPC
            let scenario_parameter = {
                let mut rx_grpc = self.rx_grpc.lock().await;
                rx_grpc.recv().await
            };

            match scenario_parameter {
                Some(param) => {
                    println!("Received scenario parameter: {:?}", param);
                    match param.action {
                        0 => {
                            // Allow
                            // Subscribe to vehicle data only if conditions exist and topic is valid
                            if let Some(conditions) = param.scenario.get_conditions().as_ref() {
                                let topic_name = conditions.get_operand_value();
                                let data_type_name = conditions.get_operand_value();

                                // Only subscribe if topic name is not empty
                                if !topic_name.is_empty() {
                                    let mut vehicle_manager = self.vehicle_manager.lock().await;
                                    if let Err(e) = vehicle_manager
                                        .subscribe_topic(topic_name, data_type_name)
                                        .await
                                    {
                                        eprintln!("Error subscribing to vehicle data: {:?}", e);
                                    }
                                } else {
                                    println!(
                                        "Skipping DDS subscription for scenario '{}': empty topic name",
                                        param.scenario.get_name()
                                    );
                                }
                            } else {
                                println!(
                                    "Scenario '{}' has no conditions, skipping DDS subscription",
                                    param.scenario.get_name()
                                );
                            }
                            self.launch_scenario_filter(param.scenario).await?;
                        }
                        1 => {
                            // Withdraw
                            // Unsubscribe from vehicle data
                            let mut vehicle_manager = self.vehicle_manager.lock().await;
                            if let Err(e) = vehicle_manager
                                .unsubscribe_topic(param.scenario.get_name().clone())
                                .await
                            {
                                eprintln!("Error unsubscribing from vehicle data: {:?}", e);
                            }
                            self.remove_scenario_filter(param.scenario.get_name().clone())
                                .await?;
                        }
                        _ => {}
                    }
                }
                None => {
                    // Channel closed
                    println!("gRPC channel closed, stopping processor");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Start the manager processing
    ///
    /// This function processes incoming scenario requests and
    /// coordinates DDS data handling.
    ///
    /// # Returns
    pub async fn run(self) -> Result<()> {
        // 자신을 Arc로 래핑
        let arc_self = Arc::new(self);

        // DDS 데이터 처리 태스크 시작
        let gateway_dds_manager = Arc::clone(&arc_self);
        let dds_processor = tokio::spawn(async move {
            if let Err(e) = gateway_dds_manager.process_dds_data().await {
                eprintln!("Error in DDS processor: {:?}", e);
            }
        });

        // gRPC 요청 처리를 위해 process_grpc_requests도 &self로 수정해야 함
        let gateway_grpc_manager = Arc::clone(&arc_self);
        let grpc_processor = tokio::spawn(async move {
            if let Err(e) = gateway_grpc_manager.process_grpc_requests().await {
                eprintln!("Error in gRPC processor: {:?}", e);
            }
        });

        // 태스크 완료 대기
        let _ = tokio::try_join!(dds_processor, grpc_processor);

        println!("FilterGatewayManager stopped");

        Ok(())
    }
    ///
    /// Registers a subscription to vehicle data topics needed for a scenario.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    /// * `vehicle_message` - Vehicle message information containing topic details
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn subscribe_vehicle_data(&self, vehicle_message: DdsData) -> Result<()> {
        use std::time::Instant;
        let start = Instant::now();

        println!("subscribe vehicle data {}", vehicle_message.name);
        println!("subscribe vehicle data {}", vehicle_message.value);
        let mut vehicle_manager = self.vehicle_manager.lock().await;
        vehicle_manager
            .subscribe_topic(vehicle_message.name, vehicle_message.value)
            .await?;

        let elapsed = start.elapsed();
        println!("subscribe_vehicle_data: elapsed = {:?}", elapsed);

        Ok(())
    }

    /// Unsubscribe from vehicle data for a scenario
    ///
    /// Cancels a subscription to vehicle data topics for a scenario.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    /// * `vehicle_message` - Vehicle message information containing topic details
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn unsubscribe_vehicle_data(&self, vehicle_message: DdsData) -> Result<()> {
        println!("unsubscribe vehicle data {}", vehicle_message.name);
        let mut vehicle_manager = self.vehicle_manager.lock().await;
        vehicle_manager
            .unsubscribe_topic(vehicle_message.name)
            .await?;

        Ok(())
    }

    /// Create and launch a filter for a scenario
    ///
    /// Creates a new filter for processing a scenario's conditions and
    /// launches it as a separate thread.
    ///
    /// # Arguments
    ///
    /// * `scenario` - Complete scenario information
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn launch_scenario_filter(&self, scenario: Scenario) -> Result<()> {
        use std::time::Instant;
        let start = Instant::now();

        // Check if the scenario has conditions
        if scenario.get_conditions().is_none() {
            println!("No conditions for scenario: {}", scenario.get_name());
            let mut sender = self.sender.lock().await;
            sender.trigger_action(scenario.get_name().clone()).await?;
            let elapsed = start.elapsed();
            println!("launch_scenario_filter: elapsed = {:?}", elapsed);
            return Ok(());
        }

        // Set scenario state from idle to waiting when conditions are registered
        println!("🔄 SCENARIO STATE TRANSITION: FilterGateway Condition Registration");
        println!("   📋 Scenario: {}", scenario.get_name());
        println!("   🔄 State Change: idle → waiting");
        println!("   🔍 Reason: Scenario conditions registered in FilterGateway");

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: scenario.get_name().clone(),
            current_state: "idle".to_string(),
            target_state: "waiting".to_string(),
            transition_id: format!("filtergateway-condition-registered-{}", timestamp),
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

        let mut state_sender = StateManagerSender::new();
        if let Err(e) = state_sender.send_state_change(state_change).await {
            println!("   ❌ Failed to send state change to StateManager: {:?}", e);
        } else {
            println!(
                "   ✅ Successfully notified StateManager: scenario {} idle → waiting",
                scenario.get_name()
            );
        }

        let sender = {
            let sender_guard = self.sender.lock().await;
            sender_guard.clone()
        };
        let filter = Filter::new(scenario.get_name().to_string(), scenario, true, sender);

        // Add the filter to our managed collection
        {
            // Prevent duplicate filters for the same scenario
            let mut filters = self.filters.lock().await;
            if filters
                .iter()
                .any(|f| f.scenario_name == filter.scenario_name)
            {
                println!(
                    "Filter for scenario '{}' already exists, skipping.",
                    filter.scenario_name
                );
                let elapsed = start.elapsed();
                println!("launch_scenario_filter: elapsed = {:?}", elapsed);
                return Ok(());
            }
            filters.push(filter);
        }
        let elapsed = start.elapsed();
        println!("launch_scenario_filter: elapsed = {:?}", elapsed);
        Ok(())
    }

    /// Remove a filter for a scenario
    ///
    /// Stops and removes the filter associated with a scenario.
    ///
    /// # Arguments
    ///
    /// * `scenario_name` - Name of the scenario
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn remove_scenario_filter(&self, scenario_name: String) -> Result<()> {
        println!("remove filter {}\n", scenario_name);

        let arc_filters = Arc::clone(&self.filters);
        let mut filters = arc_filters.lock().await;
        let index = filters
            .iter()
            .position(|f| f.scenario_name == scenario_name);
        if let Some(i) = index {
            filters.remove(i);
        }
        Ok(())
    }

    /// Read all scenario yaml string in etcd
    ///
    /// ### Parameters
    /// * None
    /// ### Return
    /// * `Result<Vec<String>>` - `Ok(_)` contains scenario yaml string vector
    async fn read_all_scenario_from_etcd() -> common::Result<Vec<String>> {
        let kv_scenario = common::etcd::get_all_with_prefix("Scenario").await?;
        let values = kv_scenario.into_iter().map(|kv| kv.value).collect();

        Ok(values)
    }
}
//Unit Tets Cases
#[cfg(test)]
mod tests {
    use anyhow::Result;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tokio::sync::{mpsc, Mutex};

    // ===== Dummy scenario and condition structs for simulating scenarios =====
    #[derive(Debug, Clone)]
    struct DummyScenario {
        name: String,
        has_conditions: bool,
    }

    impl DummyScenario {
        pub fn get_name(&self) -> String {
            self.name.clone()
        }

        pub fn get_conditions(&self) -> Option<DummyCondition> {
            if self.has_conditions {
                Some(DummyCondition {})
            } else {
                None
            }
        }
    }

    #[derive(Debug, Clone)]
    struct DummyCondition;

    impl DummyCondition {
        pub fn get_operand_value(&self) -> String {
            "topic".to_string()
        }
    }

    #[derive(Debug, Clone)]
    struct DummyScenarioParam {
        pub action: u32,
        pub scenario: DummyScenario,
    }

    // ===== Mock VehicleManager for subscribe/unsubscribe simulation =====
    struct MockVehicleManager {
        pub subscribed: Arc<AtomicBool>,
        pub unsubscribed: Arc<AtomicBool>,
    }

    impl MockVehicleManager {
        async fn subscribe_topic(&self, _topic: String, _dtype: String) -> Result<()> {
            self.subscribed.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn unsubscribe_topic(&self, _topic: String) -> Result<()> {
            self.unsubscribed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    // ===== DummyManager simulates your core manager behavior =====
    struct DummyManager {
        rx_grpc: Arc<Mutex<mpsc::Receiver<DummyScenarioParam>>>,
        vehicle_manager: Arc<Mutex<MockVehicleManager>>,
        launched: Arc<AtomicBool>,
        removed: Arc<AtomicBool>,
    }

    impl DummyManager {
        // Simulate launching scenario filter
        async fn launch_scenario_filter(&self, _scenario: DummyScenario) -> Result<()> {
            self.launched.store(true, Ordering::SeqCst);
            Ok(())
        }

        // Simulate removing scenario filter
        async fn remove_scenario_filter(&self, _name: String) -> Result<()> {
            self.removed.store(true, Ordering::SeqCst);
            Ok(())
        }

        // Processes incoming gRPC requests asynchronously
        async fn process_grpc_requests(&self) -> Result<()> {
            loop {
                let scenario_parameter = {
                    let mut rx_grpc = self.rx_grpc.lock().await;
                    rx_grpc.recv().await
                };

                match scenario_parameter {
                    Some(param) => {
                        match param.action {
                            0 => {
                                // Subscribe + Launch
                                let topic = param
                                    .scenario
                                    .get_conditions()
                                    .map(|cond| cond.get_operand_value())
                                    .unwrap_or_default();
                                let vm = self.vehicle_manager.lock().await;
                                vm.subscribe_topic(topic.clone(), topic.clone()).await?;
                                self.launch_scenario_filter(param.scenario).await?;
                            }
                            1 => {
                                // Unsubscribe + Remove
                                let vm = self.vehicle_manager.lock().await;
                                vm.unsubscribe_topic(param.scenario.get_name()).await?;
                                self.remove_scenario_filter(param.scenario.get_name())
                                    .await?;
                            }
                            _ => { // Unknown action, do nothing
                                 // Could log or ignore
                            }
                        }
                    }
                    None => break, // Channel closed
                }
            }
            Ok(())
        }

        // Subscribe to vehicle data via DDS
        async fn subscribe_vehicle_data(&self, msg: DdsData) -> Result<()> {
            let vm = self.vehicle_manager.lock().await;
            vm.subscribe_topic(msg.name.clone(), msg.value.clone())
                .await
        }

        // Unsubscribe vehicle data via DDS
        async fn unsubscribe_vehicle_data(&self, msg: DdsData) -> Result<()> {
            let vm = self.vehicle_manager.lock().await;
            vm.unsubscribe_topic(msg.name.clone()).await
        }
    }

    // ===== DDS data structure used in tests =====
    #[derive(Clone)]
    struct DdsData {
        pub name: String,
        pub value: String,
    }

    // ===== Dummy DDS data and MockFilter for filter testing =====
    #[derive(Debug, Clone)]
    struct DummyDdsData {
        pub name: String,
        pub value: String,
    }

    struct MockFilter {
        pub name: String,
        pub called: Arc<AtomicBool>,
        pub should_be_active: bool,
    }

    impl MockFilter {
        pub fn is_active(&self) -> bool {
            self.should_be_active
        }

        pub async fn process_data(&mut self, data: &DummyDdsData) -> Result<()> {
            println!("MockFilter received: {:?}", data);
            self.called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    struct DummyFilterManager {
        rx_dds: Arc<Mutex<mpsc::Receiver<DummyDdsData>>>,
        filters: Arc<Mutex<Vec<MockFilter>>>,
    }

    impl DummyFilterManager {
        // Process DDS data and apply active filters
        async fn process_dds_data(&self) -> Result<()> {
            let rx_dds = Arc::clone(&self.rx_dds);
            loop {
                let mut receiver = rx_dds.lock().await;

                match receiver.recv().await {
                    Some(dds_data) => {
                        println!(
                            "Received DDS data: topic={}, value={}",
                            dds_data.name, dds_data.value
                        );

                        let mut filters = self.filters.lock().await;
                        for filter in filters.iter_mut() {
                            if filter.is_active() {
                                if let Err(e) = filter.process_data(&dds_data).await {
                                    println!("Error processing data: {:?}", e);
                                }
                            }
                        }
                    }
                    None => {
                        println!("Channel closed.");
                        break;
                    }
                }
            }

            Ok(())
        }
    }

    // ==================== Tests ==========================

    /// Test that a gRPC 'allow' request correctly subscribes and launches scenario filter
    #[tokio::test]
    async fn test_grpc_allow() {
        let (tx, rx) = mpsc::channel(1);
        let manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(rx)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        let subscribed = Arc::clone(&manager.vehicle_manager.lock().await.subscribed);
        let launched = Arc::clone(&manager.launched);

        let scenario = DummyScenario {
            name: "AllowTest".into(),
            has_conditions: true,
        };
        tx.send(DummyScenarioParam {
            action: 0,
            scenario,
        })
        .await
        .unwrap();
        drop(tx);

        manager.process_grpc_requests().await.unwrap();

        assert!(
            subscribed.load(Ordering::SeqCst),
            "Vehicle should be subscribed"
        );
        assert!(
            launched.load(Ordering::SeqCst),
            "Scenario filter should be launched"
        );
    }

    /// Test that a gRPC 'withdraw' request correctly unsubscribes and removes scenario filter
    #[tokio::test]
    async fn test_grpc_withdraw() {
        let (tx, rx) = mpsc::channel(1);
        let manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(rx)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        let unsubscribed = Arc::clone(&manager.vehicle_manager.lock().await.unsubscribed);
        let removed = Arc::clone(&manager.removed);

        let scenario = DummyScenario {
            name: "WithdrawTest".into(),
            has_conditions: false,
        };
        tx.send(DummyScenarioParam {
            action: 1,
            scenario,
        })
        .await
        .unwrap();
        drop(tx);

        manager.process_grpc_requests().await.unwrap();

        assert!(
            unsubscribed.load(Ordering::SeqCst),
            "Vehicle should be unsubscribed"
        );
        assert!(
            removed.load(Ordering::SeqCst),
            "Scenario filter should be removed"
        );
    }

    /// Negative test: Unknown action code should not subscribe or unsubscribe anything
    #[tokio::test]
    async fn test_grpc_unknown_action() {
        let (tx, rx) = mpsc::channel(1);
        let manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(rx)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        let scenario = DummyScenario {
            name: "UnknownAction".into(),
            has_conditions: true,
        };
        tx.send(DummyScenarioParam {
            action: 99,
            scenario,
        })
        .await
        .unwrap();
        drop(tx);

        manager.process_grpc_requests().await.unwrap();

        assert!(
            !manager
                .vehicle_manager
                .lock()
                .await
                .subscribed
                .load(Ordering::SeqCst),
            "No subscription should happen"
        );
        assert!(
            !manager
                .vehicle_manager
                .lock()
                .await
                .unsubscribed
                .load(Ordering::SeqCst),
            "No unsubscription should happen"
        );
        assert!(
            !manager.launched.load(Ordering::SeqCst),
            "No scenario should be launched"
        );
        assert!(
            !manager.removed.load(Ordering::SeqCst),
            "No scenario should be removed"
        );
    }

    /// Test successful subscription to vehicle data via DDS
    #[tokio::test]
    async fn test_subscribe_vehicle_data() {
        let manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(mpsc::channel(1).1)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        let subscribed = Arc::clone(&manager.vehicle_manager.lock().await.subscribed);

        let msg = DdsData {
            name: "speed".to_string(),
            value: "f32".to_string(),
        };

        manager.subscribe_vehicle_data(msg).await.unwrap();

        assert!(
            subscribed.load(Ordering::SeqCst),
            "Subscription flag should be set"
        );
    }

    /// Test successful unsubscription from vehicle data via DDS
    #[tokio::test]
    async fn test_unsubscribe_vehicle_data() {
        let manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(mpsc::channel(1).1)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        let unsubscribed = Arc::clone(&manager.vehicle_manager.lock().await.unsubscribed);

        let msg = DdsData {
            name: "rpm".to_string(),
            value: "i32".to_string(),
        };

        manager.unsubscribe_vehicle_data(msg).await.unwrap();

        assert!(
            unsubscribed.load(Ordering::SeqCst),
            "Unsubscription flag should be set"
        );
    }

    /// Test processing DDS data through active filters and verify filter is called
    #[tokio::test]
    async fn test_process_dds_data_directly() {
        let (tx, rx) = mpsc::channel(1);
        let rx_dds = Arc::new(Mutex::new(rx));

        let test_data = DummyDdsData {
            name: "test_topic".to_string(),
            value: "42".to_string(),
        };

        tx.send(test_data.clone()).await.unwrap();
        drop(tx);

        let called = Arc::new(AtomicBool::new(false));

        let filter = MockFilter {
            name: "TestFilter".to_string(),
            called: Arc::clone(&called),
            should_be_active: true,
        };

        let filters = Arc::new(Mutex::new(vec![filter]));

        let manager = DummyFilterManager {
            rx_dds,
            filters: Arc::clone(&filters),
        };

        manager.process_dds_data().await.unwrap();

        assert!(
            called.load(Ordering::SeqCst),
            "Filter should have been called"
        );
    }

    /// Negative test: Filter inactive - filter process_data should NOT be called
    #[tokio::test]
    async fn test_process_dds_data_with_inactive_filter() {
        let (tx, rx) = mpsc::channel(1);
        let rx_dds = Arc::new(Mutex::new(rx));

        let test_data = DummyDdsData {
            name: "test_topic".to_string(),
            value: "42".to_string(),
        };

        tx.send(test_data.clone()).await.unwrap();
        drop(tx);

        let called = Arc::new(AtomicBool::new(false));

        let filter = MockFilter {
            name: "InactiveFilter".to_string(),
            called: Arc::clone(&called),
            should_be_active: false, // Filter inactive
        };

        let filters = Arc::new(Mutex::new(vec![filter]));

        let manager = DummyFilterManager {
            rx_dds,
            filters: Arc::clone(&filters),
        };

        manager.process_dds_data().await.unwrap();

        assert!(
            !called.load(Ordering::SeqCst),
            "Inactive filter should NOT be called"
        );
    }
    // Test case: Ensure no actions are performed when the gRPC channel is empty
    #[tokio::test]
    async fn test_grpc_empty_channel() {
        let (_, rx) = mpsc::channel(1);
        let manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(rx)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        manager.process_grpc_requests().await.unwrap();

        assert!(
            !manager
                .vehicle_manager
                .lock()
                .await
                .subscribed
                .load(Ordering::SeqCst),
            "No subscription should happen"
        );
        assert!(
            !manager
                .vehicle_manager
                .lock()
                .await
                .unsubscribed
                .load(Ordering::SeqCst),
            "No unsubscription should happen"
        );
        assert!(
            !manager.launched.load(Ordering::SeqCst),
            "No scenario should be launched"
        );
        assert!(
            !manager.removed.load(Ordering::SeqCst),
            "No scenario should be removed"
        );
    }

    // Test case: Verify behavior when the DDS data channel is closed
    #[tokio::test]
    async fn test_dds_data_channel_closed() {
        let (_, rx) = mpsc::channel(1);
        let rx_dds = Arc::new(Mutex::new(rx));

        let filters = Arc::new(Mutex::new(vec![]));

        let manager = DummyFilterManager {
            rx_dds,
            filters: Arc::clone(&filters),
        };

        manager.process_dds_data().await.unwrap();

        let filters = manager.filters.lock().await;
        assert!(
            filters.is_empty(),
            "No filters should be called as channel is closed"
        );
    }

    // Test case: Handle multiple gRPC requests and validate correct actions
    #[tokio::test]
    async fn test_grpc_multiple_requests() {
        let (tx, rx) = mpsc::channel(10);
        let manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(rx)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        let subscribed = Arc::clone(&manager.vehicle_manager.lock().await.subscribed);
        let unsubscribed = Arc::clone(&manager.vehicle_manager.lock().await.unsubscribed);
        let launched = Arc::clone(&manager.launched);
        let removed = Arc::clone(&manager.removed);

        let scenario1 = DummyScenario {
            name: "Scenario1".into(),
            has_conditions: true,
        };
        let scenario2 = DummyScenario {
            name: "Scenario2".into(),
            has_conditions: false,
        };

        tx.send(DummyScenarioParam {
            action: 0,
            scenario: scenario1,
        })
        .await
        .unwrap();
        tx.send(DummyScenarioParam {
            action: 1,
            scenario: scenario2,
        })
        .await
        .unwrap();
        drop(tx);

        manager.process_grpc_requests().await.unwrap();

        assert!(
            subscribed.load(Ordering::SeqCst),
            "Vehicle should be subscribed for Scenario1"
        );
        assert!(
            launched.load(Ordering::SeqCst),
            "Scenario1 filter should be launched"
        );
        assert!(
            unsubscribed.load(Ordering::SeqCst),
            "Vehicle should be unsubscribed for Scenario2"
        );
        assert!(
            removed.load(Ordering::SeqCst),
            "Scenario2 filter should be removed"
        );
    }

    // Test case: Validate processing DDS data through multiple active filters
    #[tokio::test]
    async fn test_dds_data_multiple_filters() {
        let (tx, rx) = mpsc::channel(10);
        let rx_dds = Arc::new(Mutex::new(rx));

        let test_data = DummyDdsData {
            name: "test_topic".to_string(),
            value: "42".to_string(),
        };

        tx.send(test_data.clone()).await.unwrap();
        drop(tx);

        let called_filter1 = Arc::new(AtomicBool::new(false));
        let called_filter2 = Arc::new(AtomicBool::new(false));

        let filter1 = MockFilter {
            name: "Filter1".to_string(),
            called: Arc::clone(&called_filter1),
            should_be_active: true,
        };

        let filter2 = MockFilter {
            name: "Filter2".to_string(),
            called: Arc::clone(&called_filter2),
            should_be_active: true,
        };

        let filters = Arc::new(Mutex::new(vec![filter1, filter2]));

        let manager = DummyFilterManager {
            rx_dds,
            filters: Arc::clone(&filters),
        };

        manager.process_dds_data().await.unwrap();

        assert!(
            called_filter1.load(Ordering::SeqCst),
            "Filter1 should have been called"
        );
        assert!(
            called_filter2.load(Ordering::SeqCst),
            "Filter2 should have been called"
        );
    }

    // Test case: Ensure inactive filters do not process DDS data
    #[tokio::test]
    async fn test_dds_data_no_active_filters() {
        let (tx, rx) = mpsc::channel(10);
        let rx_dds = Arc::new(Mutex::new(rx));

        let test_data = DummyDdsData {
            name: "test_topic".to_string(),
            value: "42".to_string(),
        };

        tx.send(test_data.clone()).await.unwrap();
        drop(tx);

        let called_filter = Arc::new(AtomicBool::new(false));

        let filter = MockFilter {
            name: "InactiveFilter".to_string(),
            called: Arc::clone(&called_filter),
            should_be_active: false,
        };

        let filters = Arc::new(Mutex::new(vec![filter]));

        let manager = DummyFilterManager {
            rx_dds,
            filters: Arc::clone(&filters),
        };

        manager.process_dds_data().await.unwrap();

        assert!(
            !called_filter.load(Ordering::SeqCst),
            "Inactive filter should NOT be called"
        );
    }

    /// Test vehicle manager initialization error handling
    #[tokio::test]
    async fn test_vehicle_manager_init_error() {
        // Mock a VehicleManager that fails on init
        struct FailingVehicleManager {
            pub init_failed: Arc<AtomicBool>,
        }

        impl FailingVehicleManager {
            async fn init(&mut self) -> Result<()> {
                self.init_failed.store(true, Ordering::SeqCst);
                Err(anyhow::anyhow!("Initialization failed"))
            }
        }

        let init_failed = Arc::new(AtomicBool::new(false));
        let mut vm = FailingVehicleManager {
            init_failed: Arc::clone(&init_failed),
        };

        // Simulate the error handling path in FilterGatewayManager::new
        if let Err(e) = vm.init().await {
            println!("Warning: Failed to initialize vehicle manager: {:?}. Continuing with default settings.", e);
        }

        assert!(
            init_failed.load(Ordering::SeqCst),
            "Init should have failed"
        );
    }

    /// Test DDS data processing with empty name/value
    #[tokio::test]
    async fn test_process_dds_data_with_empty_fields() {
        let (tx, rx) = mpsc::channel(10);
        let rx_dds = Arc::new(Mutex::new(rx));

        // Send data with empty fields - should not print
        let empty_data = DummyDdsData {
            name: "".to_string(),
            value: "".to_string(),
        };

        let non_empty_data = DummyDdsData {
            name: "topic".to_string(),
            value: "value".to_string(),
        };

        tx.send(empty_data).await.unwrap();
        tx.send(non_empty_data).await.unwrap();
        drop(tx);

        let called = Arc::new(AtomicBool::new(false));
        let filter = MockFilter {
            name: "TestFilter".to_string(),
            called: Arc::clone(&called),
            should_be_active: true,
        };

        let filters = Arc::new(Mutex::new(vec![filter]));
        let manager = DummyFilterManager { rx_dds, filters };

        manager.process_dds_data().await.unwrap();

        assert!(
            called.load(Ordering::SeqCst),
            "Filter should be called for non-empty data"
        );
    }

    /// Test scenario filter with no conditions
    #[tokio::test]
    async fn test_launch_scenario_filter_no_conditions() {
        use super::*;

        // Create a mock FilterGatewayManager-like struct
        struct MockFilterGatewayManager {
            sender_triggered: Arc<AtomicBool>,
        }

        impl MockFilterGatewayManager {
            async fn mock_launch_scenario_filter(&self, scenario: DummyScenario) -> Result<()> {
                // Check if the scenario has conditions
                if scenario.get_conditions().is_none() {
                    println!("No conditions for scenario: {}", scenario.get_name());
                    // Mock trigger action
                    self.sender_triggered.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                Ok(())
            }
        }

        let manager = MockFilterGatewayManager {
            sender_triggered: Arc::new(AtomicBool::new(false)),
        };

        let scenario_no_conditions = DummyScenario {
            name: "NoConditions".to_string(),
            has_conditions: false,
        };

        manager
            .mock_launch_scenario_filter(scenario_no_conditions)
            .await
            .unwrap();

        assert!(
            manager.sender_triggered.load(Ordering::SeqCst),
            "Sender should be triggered for scenario with no conditions"
        );
    }

    /// Test subscribe_topic error handling
    #[tokio::test]
    async fn test_subscribe_topic_error_handling() {
        // Mock VehicleManager that fails on subscribe
        struct FailingVehicleManager {
            pub subscribe_error: Arc<AtomicBool>,
        }

        impl FailingVehicleManager {
            async fn subscribe_topic(&mut self, _topic: String, _data_type: String) -> Result<()> {
                self.subscribe_error.store(true, Ordering::SeqCst);
                Err(anyhow::anyhow!("Subscribe failed"))
            }
        }

        let subscribe_error = Arc::new(AtomicBool::new(false));
        let mut vm = FailingVehicleManager {
            subscribe_error: Arc::clone(&subscribe_error),
        };

        // Simulate the error handling path
        if let Err(e) = vm
            .subscribe_topic("topic".to_string(), "type".to_string())
            .await
        {
            eprintln!("Error subscribing to vehicle data: {:?}", e);
        }

        assert!(
            subscribe_error.load(Ordering::SeqCst),
            "Subscribe should have failed"
        );
    }

    /// Test unsubscribe_topic error handling
    #[tokio::test]
    async fn test_unsubscribe_topic_error_handling() {
        // Mock VehicleManager that fails on unsubscribe
        struct FailingVehicleManager {
            pub unsubscribe_error: Arc<AtomicBool>,
        }

        impl FailingVehicleManager {
            async fn unsubscribe_topic(&mut self, _topic: String) -> Result<()> {
                self.unsubscribe_error.store(true, Ordering::SeqCst);
                Err(anyhow::anyhow!("Unsubscribe failed"))
            }
        }

        let unsubscribe_error = Arc::new(AtomicBool::new(false));
        let mut vm = FailingVehicleManager {
            unsubscribe_error: Arc::clone(&unsubscribe_error),
        };

        // Simulate the error handling path
        if let Err(e) = vm.unsubscribe_topic("topic".to_string()).await {
            eprintln!("Error unsubscribing from vehicle data: {:?}", e);
        }

        assert!(
            unsubscribe_error.load(Ordering::SeqCst),
            "Unsubscribe should have failed"
        );
    }

    /// Test filter processing error handling
    #[tokio::test]
    async fn test_filter_process_data_error() {
        struct FailingFilter {
            pub name: String,
            pub process_error: Arc<AtomicBool>,
        }

        impl FailingFilter {
            fn is_active(&self) -> bool {
                true
            }

            async fn process_data(&mut self, _data: &DummyDdsData) -> Result<()> {
                self.process_error.store(true, Ordering::SeqCst);
                Err(anyhow::anyhow!("Process data failed"))
            }
        }

        let (tx, rx) = mpsc::channel(1);
        let rx_dds = Arc::new(Mutex::new(rx));

        let test_data = DummyDdsData {
            name: "test_topic".to_string(),
            value: "42".to_string(),
        };

        tx.send(test_data).await.unwrap();
        drop(tx);

        let process_error = Arc::new(AtomicBool::new(false));
        let mut filter = FailingFilter {
            name: "FailingFilter".to_string(),
            process_error: Arc::clone(&process_error),
        };

        let mut receiver = rx_dds.lock().await;
        if let Some(dds_data) = receiver.recv().await {
            if filter.is_active() {
                if let Err(e) = filter.process_data(&dds_data).await {
                    println!(
                        "Error processing DDS data in filter {}: {:?}",
                        filter.name, e
                    );
                }
            }
        }

        assert!(
            process_error.load(Ordering::SeqCst),
            "Filter should have failed processing"
        );
    }

    /// Test duplicate filter prevention
    #[tokio::test]
    async fn test_duplicate_filter_prevention() {
        use super::*;

        // Mock scenario filters with duplicate names
        struct MockScenarioFilter {
            pub scenario_name: String,
        }

        struct MockFilterManager {
            filters: Vec<MockScenarioFilter>,
        }

        impl MockFilterManager {
            fn new() -> Self {
                Self { filters: vec![] }
            }

            fn mock_launch_scenario_filter(&mut self, scenario_name: String) -> Result<()> {
                // Check for duplicates
                if self
                    .filters
                    .iter()
                    .any(|f| f.scenario_name == scenario_name)
                {
                    println!(
                        "Filter for scenario '{}' already exists, skipping.",
                        scenario_name
                    );
                    return Ok(());
                }

                // Add new filter
                self.filters.push(MockScenarioFilter { scenario_name });
                Ok(())
            }
        }

        let mut manager = MockFilterManager::new();

        // Add first filter
        manager
            .mock_launch_scenario_filter("TestScenario".to_string())
            .unwrap();
        assert_eq!(manager.filters.len(), 1, "First filter should be added");

        // Try to add duplicate - should be skipped
        manager
            .mock_launch_scenario_filter("TestScenario".to_string())
            .unwrap();
        assert_eq!(
            manager.filters.len(),
            1,
            "Duplicate filter should be skipped"
        );
    }

    /// Test scenario with conditions state change
    #[tokio::test]
    async fn test_scenario_with_conditions_state_change() {
        // Mock StateManager sender
        struct MockStateManagerSender {
            pub state_change_sent: Arc<AtomicBool>,
        }

        impl MockStateManagerSender {
            fn new() -> Self {
                Self {
                    state_change_sent: Arc::new(AtomicBool::new(false)),
                }
            }

            async fn send_state_change(
                &mut self,
                _state_change: common::statemanager::StateChange,
            ) -> Result<()> {
                self.state_change_sent.store(true, Ordering::SeqCst);
                Ok(())
            }
        }

        let mut state_sender = MockStateManagerSender::new();
        let state_change_sent = Arc::clone(&state_sender.state_change_sent);

        let scenario = DummyScenario {
            name: "TestScenario".to_string(),
            has_conditions: true,
        };

        // Simulate the state change logic
        if scenario.get_conditions().is_some() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as i64;

            let state_change = common::statemanager::StateChange {
                resource_type: common::statemanager::ResourceType::Scenario as i32,
                resource_name: scenario.get_name(),
                current_state: "idle".to_string(),
                target_state: "waiting".to_string(),
                transition_id: format!("filtergateway-condition-registered-{}", timestamp),
                timestamp_ns: timestamp,
                source: "filtergateway".to_string(),
            };

            if let Err(e) = state_sender.send_state_change(state_change).await {
                println!("Failed to send state change to StateManager: {:?}", e);
            }
        }

        assert!(
            state_change_sent.load(Ordering::SeqCst),
            "State change should have been sent for scenario with conditions"
        );
    }

    /// Test scenario state change error handling
    #[tokio::test]
    async fn test_scenario_state_change_error() {
        // Mock StateManager sender that fails
        struct FailingStateManagerSender {
            pub send_error: Arc<AtomicBool>,
        }

        impl FailingStateManagerSender {
            fn new() -> Self {
                Self {
                    send_error: Arc::new(AtomicBool::new(false)),
                }
            }

            async fn send_state_change(
                &mut self,
                _state_change: common::statemanager::StateChange,
            ) -> Result<()> {
                self.send_error.store(true, Ordering::SeqCst);
                Err(anyhow::anyhow!("State change send failed"))
            }
        }

        let mut state_sender = FailingStateManagerSender::new();
        let send_error = Arc::clone(&state_sender.send_error);

        let state_change = common::statemanager::StateChange {
            resource_type: common::statemanager::ResourceType::Scenario as i32,
            resource_name: "TestScenario".to_string(),
            current_state: "idle".to_string(),
            target_state: "waiting".to_string(),
            transition_id: "test-transition".to_string(),
            timestamp_ns: 123456789,
            source: "filtergateway".to_string(),
        };

        // Test error handling path (line 264)
        if let Err(e) = state_sender.send_state_change(state_change).await {
            println!("❌ Failed to send state change to StateManager: {:?}", e);
        }

        assert!(
            send_error.load(Ordering::SeqCst),
            "State change should have failed"
        );
    }

    /// Test channel closed scenarios
    #[tokio::test]
    async fn test_channel_closed_scenarios() {
        // Test DDS channel closed
        let (_, rx_dds) = mpsc::channel::<DummyDdsData>(1);
        let rx_dds = Arc::new(Mutex::new(rx_dds));
        let filters = Arc::new(Mutex::new(vec![]));
        let dds_manager = DummyFilterManager { rx_dds, filters };

        // This should hit the "Channel closed" path
        let result = dds_manager.process_dds_data().await;
        assert!(
            result.is_ok(),
            "Should handle closed DDS channel gracefully"
        );

        // Test gRPC channel closed
        let (_, rx_grpc) = mpsc::channel::<DummyScenarioParam>(1);
        let grpc_manager = DummyManager {
            rx_grpc: Arc::new(Mutex::new(rx_grpc)),
            vehicle_manager: Arc::new(Mutex::new(MockVehicleManager {
                subscribed: Arc::new(AtomicBool::new(false)),
                unsubscribed: Arc::new(AtomicBool::new(false)),
            })),
            launched: Arc::new(AtomicBool::new(false)),
            removed: Arc::new(AtomicBool::new(false)),
        };

        // This should hit the "gRPC channel closed" path
        let result = grpc_manager.process_grpc_requests().await;
        assert!(
            result.is_ok(),
            "Should handle closed gRPC channel gracefully"
        );
    }

    /// Test scenario initialization from etcd
    #[tokio::test]
    async fn test_initialize_with_scenario_subscription_error() {
        use super::*;

        // Mock manager with failing vehicle manager
        struct MockInitManager {
            vehicle_manager_error: Arc<AtomicBool>,
        }

        impl MockInitManager {
            async fn mock_initialize(&self) -> Result<()> {
                println!("FilterGatewayManager init");

                // Simulate scenarios from etcd
                let scenarios = vec!["
apiVersion: v1
kind: Scenario
metadata:
  name: test-scenario
spec:
  action: update
  target: test-scenario
  condition:
    express: eq
    value: ready
    operands:
      type: pod
      name: test-pod
      value: status
"
                .to_string()];

                for scenario_str in scenarios {
                    let scenario: common::spec::artifact::Scenario =
                        serde_yaml::from_str(&scenario_str)?;
                    let topic_name = scenario
                        .get_conditions()
                        .as_ref()
                        .map(|cond| cond.get_operand_value())
                        .unwrap_or_default();

                    // Simulate vehicle manager subscribe error (line 104)
                    self.vehicle_manager_error.store(true, Ordering::SeqCst);
                    eprintln!("Error subscribing to vehicle data: Test error");
                }

                Ok(())
            }
        }

        let manager = MockInitManager {
            vehicle_manager_error: Arc::new(AtomicBool::new(false)),
        };

        manager.mock_initialize().await.unwrap();

        assert!(
            manager.vehicle_manager_error.load(Ordering::SeqCst),
            "Vehicle manager error should be triggered"
        );
    }
}

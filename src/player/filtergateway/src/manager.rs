use crate::filter::Filter;
use crate::grpc::sender::actioncontroller::FilterGatewaySender;
use crate::vehicle::dds::DdsData;
use crate::vehicle::VehicleManager;
use common::spec::artifact::Scenario;
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
pub struct ScenarioParameter {
    /// Name of the scenario
    pub action: i32,
    /// Vehicle message information
    pub scenario: Scenario,
}

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
            let topic_name = scenario
                .get_conditions()
                .as_ref()
                .map(|cond| cond.get_operand_value())
                .unwrap_or_default();
            let data_type_name = scenario
                .get_conditions()
                .as_ref()
                .map(|cond| cond.get_operand_value())
                .unwrap_or_default();
            let mut vehicle_manager = self.vehicle_manager.lock().await;
            if let Err(e) = vehicle_manager
                .subscribe_topic(topic_name, data_type_name)
                .await
            {
                eprintln!("Error subscribing to vehicle data: {:?}", e);
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
                    // Log DDS data reception
                    println!(
                        "Received DDS data: topic={}, value={}",
                        dds_data.name, dds_data.value
                    );

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
                            // Subscribe to vehicle data
                            let topic_name = param
                                .scenario
                                .get_conditions()
                                .as_ref()
                                .map(|cond| cond.get_operand_value())
                                .unwrap_or_default();
                            let data_type_name = param
                                .scenario
                                .get_conditions()
                                .as_ref()
                                .map(|cond| cond.get_operand_value())
                                .unwrap_or_default();
                            let mut vehicle_manager = self.vehicle_manager.lock().await;
                            if let Err(e) = vehicle_manager
                                .subscribe_topic(topic_name, data_type_name)
                                .await
                            {
                                eprintln!("Error subscribing to vehicle data: {:?}", e);
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
        println!("subscribe vehicle data {}", vehicle_message.name);
        println!("subscribe vehicle data {}", vehicle_message.value);
        let mut vehicle_manager = self.vehicle_manager.lock().await;
        vehicle_manager
            .subscribe_topic(vehicle_message.name, vehicle_message.value)
            .await?;

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
        // Check if the scenario has conditions
        if scenario.get_conditions().is_none() {
            println!("No conditions for scenario: {}", scenario.get_name());
            let mut sender = self.sender.lock().await;
            sender.trigger_action(scenario.get_name().clone()).await?;
            return Ok(());
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
                return Ok(());
            }
            filters.push(filter);
        }
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
}

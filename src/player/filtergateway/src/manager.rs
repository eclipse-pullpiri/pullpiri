use crate::filter::Filter;
use crate::grpc::sender::FilterGatewaySender;
use crate::vehicle::dds::DdsData;
use crate::vehicle::{self, VehicleManager};
use common::spec::artifact::Scenario;
use common::{spec::artifact::Artifact, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Manager for FilterGateway
///
/// Responsible for:
/// - Managing scenario filters
/// - Coordinating vehicle data subscriptions
/// - Processing incoming scenario requests
/// 
#[derive(Debug, Clone)]
pub struct ScenarioParameter {
    /// Name of the scenario
    pub action: i32,
    /// Vehicle message information
    pub vehicle_message: DdsData,
}
pub struct FilterGatewayManager {
    /// Receiver for scenario information from gRPC
    rx_grpc: mpsc::Receiver<ScenarioParameter>,
    /// Receiver for DDS data
    rx_dds: Arc<Mutex<mpsc::Receiver<DdsData>>>,
    /// Active filters for scenarios
    filters: Arc<Mutex<Vec<Filter>>>,
    /// gRPC sender for action controller
    sender: Arc<FilterGatewaySender>,
    /// Vehicle manager for handling vehicle data
    vehicle_manager: VehicleManager,
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
        let sender = Arc::new(FilterGatewaySender::new());
        let mut vehicle_manager = VehicleManager::new(tx_dds);
        
        // 오류 처리 개선: unwrap() 대신 명시적 에러 처리
        if let Err(e) = vehicle_manager.init().await {
            println!("Warning: Failed to initialize vehicle manager: {:?}. Continuing with default settings.", e);
            // 계속 진행 (이미 VehicleManager::init()에서 기본값 사용)
        }
        
        Self {
            rx_grpc: rx_grpc,
            rx_dds: Arc::new(Mutex::new(rx_dds)),
            filters: Arc::new(Mutex::new(Vec::new())),
            sender,
            vehicle_manager: vehicle_manager,
        }
    }

    /// Start the manager processing
    ///
    /// This function processes incoming scenario requests and
    /// coordinates DDS data handling.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error result
    pub async fn run(&mut self) -> Result<()> {
        loop {
            // Wait for scenario parameter from gRPC
            let scenario_parameter = self.rx_grpc.recv().await;
            match scenario_parameter {
                Some(param) => {
                    println!("Received scenario parameter: {:?}", param);
                    match param.action {
                        0 => { // Allow
                            // Subscribe to vehicle data
                            self.subscribe_vehicle_data(
                                param.vehicle_message.clone(),
                            )
                            .await?;
                        }
                        1 => {//Withdraw
                            // Unsubscribe from vehicle data
                            self.unsubscribe_vehicle_data(
                                param.vehicle_message.clone(),
                            )
                            .await?;
                        }
                        _ => {}
                    }
                }
                None => break,
            }
        }
        Ok(())
    }

    /// Subscribe to vehicle data for a scenario
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
    pub async fn subscribe_vehicle_data(
        &mut self,
        vehicle_message: DdsData,
    ) -> Result<()> {
        print!("subscribe vehicle data {}\n", vehicle_message.name);
        self.vehicle_manager
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
    pub async fn unsubscribe_vehicle_data(
        &mut self,
        vehicle_message: DdsData,
    ) -> Result<()> {
        print!("unsubscribe vehicle data {}\n", vehicle_message.name);
        self.vehicle_manager
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
            self.sender
                .trigger_action(scenario.get_name().clone())
                .await?;
            return Ok(());
        }

        // Create a new filter for the scenario
        let filter = Filter::new(
            scenario.get_name().to_string(),
            scenario,
            true,
            self.sender.clone(),
        );

        // Add the filter to our managed collection
        {
            let mut filters = self.filters.lock().await;
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
}

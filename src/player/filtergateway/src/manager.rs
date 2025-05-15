use crate::filter::Filter;
use crate::grpc::sender::FilterGatewaySender;
use crate::vehicle::dds::DdsData;
use crate::vehicle::VehicleManager;
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
pub struct FilterGatewayManager {
    /// Receiver for scenario information from gRPC
    rx_grpc: mpsc::Receiver<Scenario>,
    /// Sender for DDS data
    tx_dds: mpsc::Sender<DdsData>,
    /// Receiver for DDS data
    rx_dds: Arc<Mutex<mpsc::Receiver<DdsData>>>,
    /// Active filters for scenarios
    filters: Arc<Mutex<Vec<Filter>>>,
    /// gRPC sender for action controller
    sender: Arc<FilterGatewaySender>,

    vehicle_manager: Arc<Mutex<VehicleManager>>,
}

    /// Creates a new FilterGatewayManager instance
    ///
    /// # Arguments
    ///
    /// * `rx` - Channel receiver for scenario information
    ///
    /// # Returns
    ///
    /// A new FilterGatewayManager instance
impl FilterGatewayManager {
    pub fn new(rx_grpc: mpsc::Receiver<Scenario>, 
        tx_dds: mpsc::Sender<DdsData>, 
        rx_dds: mpsc::Receiver<DdsData>) -> Self {        
        let sender = Arc::new(FilterGatewaySender::new());
        let tx_dds_clone = tx_dds.clone();
        let vehicle_manager = Arc::new(Mutex::new(VehicleManager::new(tx_dds_clone)));
        Self {
            rx_grpc,
            tx_dds,
            rx_dds: Arc::new(Mutex::new(rx_dds)),
            filters: Arc::new(Mutex::new(Vec::new())),
            sender,
            vehicle_manager,
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
        // TODO: Implementation      

        loop {
            tokio::select! {
                // Process incoming scenario requests from gRPC
                Some(scenario) = self.rx_grpc.recv() => {
                    println!("Received scenario: {}", scenario.get_name());

 
                },

                // // Process incoming DDS data
                // dds_data = self.rx_dds.lock().await.recv() => {
                //     if let Some(data) = dds_data {
                //         println!("Received DDS data");

                //         // Process DDS data with active filters
                //         let filters = self.filters.lock().await;
                //         for filter in filters.iter() {
                //             // Here we would process the data with each filter
                //             // Check if the scenario conditions are met
                //             filter.meet_scenario_condition(&data).await?;

                //             println!("Processing DDS data for scenario: {}", filter.scenario_name);
                //         }
                //     } else {
                //         println!("DDS data channel closed");
                //         break;
                //     }
                // },

                else => {
                    println!("All channels closed, exiting");
                    break;
                }
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
        &self,
        scenario_name: String,
        vehicle_message: DdsData,
    ) -> Result<()> {
        // let _ = (scenario_name, vehicle_message); // 사용하지 않는 변수 경고 방지
        //   TODO: Implementation
        println!("Subscribing to vehicle data for scenario: {}", scenario_name);
        // Forward vehicle message to the DDS handler
        let mut vehicle_manager = self.vehicle_manager.lock().await;
        
        vehicle_manager.subscribe_topic(vehicle_message.name.clone(), vehicle_message.name);
        
        println!("Successfully subscribed to vehicle data for scenario: {}", scenario_name);
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
        &self,
        scenario_name: String,
        vehicle_message: DdsData,
    ) -> Result<()> {
      
        println!("Unsubscribing from vehicle data for scenario: {}", scenario_name);
        // Forward vehicle message to the DDS handler to cancel subscription
        if let Err(e) = self.tx_dds.send(vehicle_message).await {
            println!("Failed to send vehicle data unsubscription: {}", e);
            return Err(e.into());
        }
        println!("Successfully unsubscribed from vehicle data for scenario: {}", scenario_name);
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

use crate::filter::Filter;
use crate::grpc::sender::FilterGatewaySender;
use crate::vehicle::dds::DdsData;
use common::{spec::artifact::Artifact, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use common::spec::artifact::Scenario;


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
    pub fn new(rx: mpsc::Receiver<Scenario>) -> Self {
        let (tx_dds, rx_dds) = mpsc::channel::<DdsData>(10);
        let sender = Arc::new(FilterGatewaySender::new());
        Self {
            rx_grpc: rx,
            tx_dds: tx_dds,
            rx_dds: Arc::new(Mutex::new(rx_dds)),
            filters: Arc::new(Mutex::new(Vec::new())),
            sender,
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
                // Some(scenario) = self.rx_grpc.recv() => {
                //     println!("Received scenario: {}", scenario.get_name());
                    
                //     match scenario.get_artifact() {
                //         Artifact::Launch => {
                //             // Launch a new filter for this scenario
                //             self.launch_scenario_filter(scenario).await?;
                //         },
                //         Artifact::Stop => {
                //             // Stop and remove the filter for this scenario
                //             self.remove_scenario_filter(scenario.get_name().to_string()).await?;
                //         },
                //         _ => {
                //             println!("Unhandled scenario artifact type: {:?}", scenario.get_artifact());
                //         }
                //     }
                // },
                
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
        let _ = (scenario_name, vehicle_message); // 사용하지 않는 변수 경고 방지
                                                  // TODO: Implementation
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
        let _ = (scenario_name, vehicle_message); // 사용하지 않는 변수 경고 방지
                                                  // TODO: Implementation
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
        // let _ = scenario; // 사용하지 않는 변수 경고 방지
        // Create a new filter for the scenario
        let filter = Filter::new(
            scenario.get_name().to_string(),
            scenario,
            true,
            self.sender.clone()
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
        let index = filters.iter().position(|f| f.scenario_name == scenario_name);
        if let Some(i) = index {
            filters.remove(i);
        }
        Ok(())
    }
}
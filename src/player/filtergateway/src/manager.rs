use crate::filter::Filter;
use crate::grpc::sender::FilterGatewaySender;
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
use std::sync::Weak;

pub struct FilterGatewayManager {
    /// Receiver for scenario information from gRPC
    rx_grpc: Arc<Mutex<mpsc::Receiver<ScenarioParameter>>>,
    /// Receiver for DDS data
    rx_dds: Arc<Mutex<mpsc::Receiver<DdsData>>>,
    /// Active filters for scenarios
    filters: Arc<Mutex<Vec<Filter>>>,
    /// gRPC sender for action controller
    sender: Arc<FilterGatewaySender>,
    /// Vehicle manager for handling vehicle data
    vehicle_manager: Arc<Mutex<VehicleManager>>,
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
            rx_grpc: Arc::new(Mutex::new(rx_grpc)),
            rx_dds: Arc::new(Mutex::new(rx_dds)),
            filters: Arc::new(Mutex::new(Vec::new())),
            sender,
            vehicle_manager: Arc::new(Mutex::new(vehicle_manager)),
        }
    }

    /// 구독된 DDS 데이터를 수신하고 필터에 전달하는 함수
    /// 
    /// 이 함수는 별도의 태스크로 실행되어 DDS 데이터를 계속 수신하고 처리합니다.
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - 성공 또는 에러 결과
    async fn process_dds_data(&self) -> Result<()> {
        // 공유된 수신자의 클론 생성
        let rx_dds = Arc::clone(&self.rx_dds);
        
        // 수신 루프
        loop {
            let mut receiver = rx_dds.lock().await;
            
            // DDS 데이터 수신
            match receiver.recv().await {
                Some(dds_data) => {
                    // DDS 데이터 수신 로깅
                    println!("Received DDS data: topic={}, value={}", dds_data.name, dds_data.value);
                    
                    // 모든 활성 필터에 데이터 전달
                    let filters = self.filters.lock().await;
                    for filter in filters.iter() {
                        if filter.is_active() {
                            // 필터에 DDS 데이터 전달
                            if let Err(e) = filter.process_data(&dds_data).await {
                                println!("Error processing DDS data in filter {}: {:?}", 
                                         filter.scenario_name, e);
                            }
                        }
                    }
                },
                None => {
                    // 채널 닫힘
                    println!("DDS data channel closed, stopping processor");
                    break;
                }
            }
        }
        
        Ok(())
    }

    /// gRPC 요청을 처리하는 함수
    /// 
    /// 이 함수는 gRPC를 통해 들어오는 시나리오 요청을 처리합니다.
    /// 
    /// # Returns
    /// 
    /// * `Result<()>` - 성공 또는 에러 결과
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
                        0 => { // Allow
                            // Subscribe to vehicle data
                            let topic_name = param.scenario.get_conditions()
                                .as_ref()
                                .map(|cond| cond.get_operand_value())
                                .unwrap_or_default();
                            let data_type_name = param.scenario.get_conditions()
                                .as_ref()
                                .map(|cond| cond.get_operand_value())
                                .unwrap_or_default();
                            let mut vehicle_manager = self.vehicle_manager.lock().await;
                            if let Err(e) = vehicle_manager.subscribe_topic(
                                topic_name,
                                data_type_name
                            ).await {
                                eprintln!("Error subscribing to vehicle data: {:?}", e);
                            }
                            self.launch_scenario_filter(param.scenario).await?;
                        }
                        1 => { // Withdraw
                            // Unsubscribe from vehicle data
                            let mut vehicle_manager = self.vehicle_manager.lock().await;
                            if let Err(e) = vehicle_manager.unsubscribe_topic(
                                param.scenario.get_name().clone()
                            ).await {
                                eprintln!("Error unsubscribing from vehicle data: {:?}", e);
                            }
                            self.remove_scenario_filter(param.scenario.get_name().clone()).await?;
                        }
                        _ => {}
                    }
                }
                None => {
                    // 채널 닫힘
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
    pub async fn subscribe_vehicle_data(
        &self,
        vehicle_message: DdsData,
    ) -> Result<()> {
        print!("subscribe vehicle data {}\n", vehicle_message.name);
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
    pub async fn unsubscribe_vehicle_data(
        &self,
        vehicle_message: DdsData,
    ) -> Result<()> {
        print!("unsubscribe vehicle data {}\n", vehicle_message.name);
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

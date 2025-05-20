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
    pub scenario: Scenario,
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
        scenario: Scenario,
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
        let value_name = condition.get_operand_name();
        let target_value = condition.get_value();
        let express = condition.get_express();

        print!(
            "Checking condition for scenario: {}\nTopic: {}\nTarget Value: {}\nValue Name: {}\nExpression: {}\n",
            self.scenario_name, topic, target_value, value_name, express
        );

        if !data.name.eq(&topic) {
            return Err("data topic does not match".into());
        }

        // fields에서 value_name과 일치하는 key를 찾고, 해당 value와 target_value를 비교
        let field_value = match data.fields.get(&value_name) {
            Some(v) => v,
            None => return Err(format!("field '{}' not found in data.fields", value_name).into()),
        };

        let check: bool = match express.as_str() {
            "eq" => target_value.to_lowercase() == field_value.to_lowercase(),
            "lt" => {
            let target_v = target_value.parse::<f32>().map_err(|_| "target_value parse error")?;
            let current_v = field_value.parse::<f32>().map_err(|_| "field_value parse error")?;
            current_v < target_v
            }
            "le" => {
            let target_v = target_value.parse::<f32>().map_err(|_| "target_value parse error")?;
            let current_v = field_value.parse::<f32>().map_err(|_| "field_value parse error")?;
            current_v <= target_v
            }
            "ge" => {
            let target_v = target_value.parse::<f32>().map_err(|_| "target_value parse error")?;
            let current_v = field_value.parse::<f32>().map_err(|_| "field_value parse error")?;
            current_v >= target_v
            }
            "gt" => {
            let target_v = target_value.parse::<f32>().map_err(|_| "target_value parse error")?;
            let current_v = field_value.parse::<f32>().map_err(|_| "field_value parse error")?;
            current_v > target_v
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

    /// 필터가 활성화되어 있는지 확인
    ///
    /// # Returns
    ///
    /// * `bool` - 필터 활성화 상태
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    
    /// DDS 데이터 처리 및 조건 검사
    ///
    /// 수신된 DDS 데이터를 처리하고 시나리오 조건을 검사합니다.
    /// 조건이 충족되면 액션을 트리거합니다.
    ///
    /// # Arguments
    ///
    /// * `data` - 수신된 DDS 데이터
    ///
    /// # Returns
    ///
    /// * `Result<()>` - 성공 또는 에러 결과
    pub async fn process_data(&self, data: &DdsData) -> Result<()> {
        // 비활성화된 필터는 처리하지 않음
        if !self.is_active {
            return Ok(());
        }

        print!(
            "process data for scenario: {}\nTopic: {}\nTarget Name: {}\nTarget Value: {}\n",
             data.name, data.value, data.fields
        );
        
        // 필터 조건에 맞는 토픽인지 확인
        let condition = match self.scenario.get_conditions() {
            Some(c) => c,
            None => return Ok(()), // 조건이 없는 경우 (이미 처리됨)
        };
        
        let topic = condition.get_operand_value();
        if !data.name.eq(&topic) {
            return Ok(()); // 관련 없는 토픽은 무시
        }
        
        // 조건 검사 수행
        match self.meet_scenario_condition(data).await {
            Ok(_) => {
                println!("Action triggered for scenario: {}", self.scenario_name);
                // 조건 충족 후 필터 비활성화 (한 번만 실행)
                // 필요시 self.is_active = false; 코드 추가
            }
            Err(e) => {
                // 조건 불충족은 정상적인 경우이므로 디버그 수준에서만 기록
                if e.to_string() != "cannot meet condition" {
                    println!("Error checking condition: {:?}", e);
                }
            }
        }
        
        Ok(())
    }
}

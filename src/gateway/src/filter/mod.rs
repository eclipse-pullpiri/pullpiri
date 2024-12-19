// SPDX-License-Identifier: Apache-2.0

use lge_dds::DdsData;
use std::str::FromStr;

#[derive(Debug)]
pub struct Filter {
    pub name: String,
    pub express: String,
    pub target_value: String,
    pub topic: String,
    pub action_key: String,
}

impl Filter {
    pub async fn new(name: &str) -> Self {
        let action_key = name.to_string();
        let conditions = common::etcd::get(&format!("scenario/{name}/condition"))
            .await
            .unwrap();

        if let Ok(condition) =
            serde_yaml::from_str::<common::spec::scenario::Condition>(&conditions)
        {
            Filter {
                name: name.to_string(),
                express: condition.get_express(),
                target_value: condition.get_value(),
                topic: condition.get_operand_value(),
                action_key,
            }
        } else {
            let _ = crate::grpc::sender::send(&action_key).await;
            Filter {
                name: name.to_string(),
                express: "".to_string(),
                target_value: "".to_string(),
                topic: "".to_string(),
                action_key,
            }
        }
    }

    pub async fn check(&mut self, data: DdsData) -> bool {
        println!("{:?}", self);

        if !data.name.eq(&self.topic) {
            return false;
        }

        match self.express.as_str() {
            "eq" => self.target_value.to_lowercase() == data.value.to_lowercase(),
            "lt" => {
                let target_v = f32::from_str(&self.target_value).unwrap();
                let current_v = f32::from_str(&data.value).unwrap();
                target_v < current_v
            }
            "le" => {
                let target_v = f32::from_str(&self.target_value).unwrap();
                let current_v = f32::from_str(&data.value).unwrap();
                target_v <= current_v
            }
            "ge" => {
                let target_v = f32::from_str(&self.target_value).unwrap();
                let current_v = f32::from_str(&data.value).unwrap();
                target_v >= current_v
            }
            "gt" => {
                let target_v = f32::from_str(&self.target_value).unwrap();
                let current_v = f32::from_str(&data.value).unwrap();
                target_v > current_v
            }
            _ => false,
        }
    }
}

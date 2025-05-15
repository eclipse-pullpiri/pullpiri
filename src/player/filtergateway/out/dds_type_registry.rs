// 자동 생성된 DDS 타입 레지스트리
// build.rs에 의해 생성됨

use dust_dds::topic_definition::type_support::{DdsType, DdsSerialize, DdsDeserialize};
use serde::{Deserialize, Serialize};
use super::dds_types::*;
use std::sync::Arc;
use crate::vehicle::dds::listener::GenericTopicListener;
use crate::vehicle::dds::DdsTopicListener;
use tokio::sync::mpsc::Sender;
use crate::vehicle::dds::DdsData;

pub fn create_typed_listener(type_name: &str, topic_name: String, tx: Sender<DdsData>, domain_id: i32) -> Option<Box<dyn DdsTopicListener>> {
    match type_name {
        "BodyLightsHeadLampStatus" => {
            let listener = Box::new(GenericTopicListener::<BodyLightsHeadLamp::BodyLightsHeadLampStatus>::new(
                topic_name,
                type_name.to_string(),
                tx,
                domain_id,
            ));
            Some(listener)
        },
        "BodyTrunkStatus" => {
            let listener = Box::new(GenericTopicListener::<BodyTrunk::BodyTrunkStatus>::new(
                topic_name,
                type_name.to_string(),
                tx,
                domain_id,
            ));
            Some(listener)
        },
        "ADASObstacleDetectionIsWarning" => {
            let listener = Box::new(GenericTopicListener::<ADASObstacleDetection::ADASObstacleDetectionIsWarning>::new(
                topic_name,
                type_name.to_string(),
                tx,
                domain_id,
            ));
            Some(listener)
        },
        _ => None,
    }
}

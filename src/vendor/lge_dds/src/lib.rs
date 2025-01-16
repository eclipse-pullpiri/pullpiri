// SPDX-License-Identifier: Apache-2.0
#![allow(non_snake_case)]

pub mod vehicle_interface;

use tokio::sync::mpsc::Sender;
use vehicle_interface::{
    adas::ADASObstacleDetection::ADASObstacleDetectionIsWarning,
    body::{BodyLightsHeadLamp::BodyLightsHeadLampStatus, BodyTrunk::BodyTrunkStatus},
    cabin::{
        CabinDoor::{CabinLeftDoorStatus, CabinRightDoorStatus},
        CabinWindow::{CabinLeftWindowStatus, CabinRightWindowStatus},
    },
    exterior::Exterior::ExteriorLightIntensity,
    powertrain::{
        PowertrainBattery::PowertrainBatteryChargingChargePortFlapStatus,
        PowertrainTransmission::PowertrainTransmissionCurrentGear,
    },
    receive_dds,
};

#[derive(Debug, Clone)]
pub struct DdsData {
    pub name: String,
    pub value: String,
}

pub trait Piccoloable {
    fn to_piccolo_dds_data(&self) -> DdsData;
    fn topic_name() -> String;
    fn type_name() -> String;
}

pub async fn run(tx: Sender<DdsData>) {
    tokio::spawn(receive_dds::<ADASObstacleDetectionIsWarning>(tx.clone()));
    tokio::spawn(receive_dds::<BodyLightsHeadLampStatus>(tx.clone()));
    tokio::spawn(receive_dds::<BodyTrunkStatus>(tx.clone()));
    tokio::spawn(receive_dds::<CabinLeftDoorStatus>(tx.clone()));
    tokio::spawn(receive_dds::<CabinLeftWindowStatus>(tx.clone()));
    tokio::spawn(receive_dds::<CabinRightDoorStatus>(tx.clone()));
    tokio::spawn(receive_dds::<CabinRightWindowStatus>(tx.clone()));
    tokio::spawn(receive_dds::<ExteriorLightIntensity>(tx.clone()));
    tokio::spawn(receive_dds::<PowertrainTransmissionCurrentGear>(tx.clone()));
    tokio::spawn(receive_dds::<PowertrainBatteryChargingChargePortFlapStatus>(tx.clone()));
}

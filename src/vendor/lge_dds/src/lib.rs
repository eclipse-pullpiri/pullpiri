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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::Path;

    #[test]
    pub fn idl2rs() {
        let src = Path::new("./src/idl");
        let dst = Path::new("./src");

        assert!(copy_dir(src, dst).is_ok());
    }

    pub fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            let file_name = entry_path.file_name().unwrap();
            let mut dest_path = dst.join(file_name);

            if entry_path.is_dir() {
                copy_dir(&entry_path, &dest_path)?;
            } else if entry_path.extension().unwrap() == "idl" {
                println!("{:?}  {:?}", entry_path, dest_path);
                let contents = get_contents_string(&entry_path);
                dest_path.set_extension("rs");
                fs::write(dest_path, contents)?;
            }
        }
        Ok(())
    }

    pub fn get_contents_string(src: &Path) -> String {
        let idl_src = fs::read_to_string(src).unwrap();
        let result = dust_dds_gen::compile_idl(&idl_src).unwrap();
        println!("{result}");
        result
    }
}

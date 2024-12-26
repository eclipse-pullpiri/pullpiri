// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, dust_dds::topic_definition::type_support::DdsType)]
pub struct PowertrainTransmissionCurrentGear {
    pub value: i32,
}

impl crate::Piccoloable for PowertrainTransmissionCurrentGear {
    fn to_piccolo_dds_data(&self) -> crate::DdsData {
        let gear = match self.value {
            // value: 0=Neutral, 1/2/..=Forward, -1/-2/..=Reverse, 126: Parking, 127: Drive
            0 => "N",
            -1 => "R",
            126 => "P",
            127 => "D",
            _ => "unknown",
        };

        crate::DdsData {
            name: String::from("PowertrainTransmissionCurrentGear"),
            value: gear.to_string(),
        }
    }
    fn topic_name() -> String {
        String::from("PowertrainTransmissionCurrentGear")
    }
    fn type_name() -> String {
        String::from("PowertrainTransmissionCurrentGear")
    }
}

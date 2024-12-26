// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, dust_dds::topic_definition::type_support::DdsType)]
pub struct ExteriorLightIntensity {
    pub value: f32,
}

impl crate::Piccoloable for ExteriorLightIntensity {
    fn to_piccolo_dds_data(&self) -> crate::DdsData {
        let day_night = if self.value < 60.0 { "night" } else { "day" };

        crate::DdsData {
            name: String::from("ExteriorLightIntensity"),
            value: day_night.to_string(),
        }
    }
    fn topic_name() -> String {
        String::from("ExteriorLightIntensity")
    }
    fn type_name() -> String {
        String::from("ExteriorLightIntensity")
    }
}

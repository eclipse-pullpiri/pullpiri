#[derive(Debug, dust_dds::topic_definition::type_support::DdsType)]
pub struct BodyLightsHeadLampStatus {
    pub command: i32,
    pub status: i32,
    pub progress: i32,
    pub uistatus: i32,
}

impl crate::Piccoloable for BodyLightsHeadLampStatus {
    fn to_piccolo_dds_data(&self) -> crate::DdsData {
        let lamp = match self.uistatus {
            // uistatus: 0 = IS_ACTING, 1 = HEADLIGHT_IS_OFF, 2 = HEADLIGHT_IS_ON
            0 => "is_acting",
            1 => "off",
            2 => "on",
            _ => "unknown",
        };

        crate::DdsData {
            name: String::from("BodyLightsHeadLampStatus"),
            value: lamp.to_string(),
        }
    }
    fn topic_name() -> String {
        String::from("BodyLightsHeadLampStatus")
    }
    fn type_name() -> String {
        String::from("BodyLightsHeadLampStatus")
    }
}

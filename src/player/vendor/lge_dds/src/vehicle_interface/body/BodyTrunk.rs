#[derive(Debug, dust_dds::topic_definition::type_support::DdsType)]
pub struct BodyTrunkStatus {
    pub command: i32,
    pub status: i32,
    pub progress: i32,
    pub uistatus: i32,
}

impl crate::Piccoloable for BodyTrunkStatus {
    fn to_piccolo_dds_data(&self) -> crate::DdsData {
        let trunk = match self.uistatus {
            // uistatus: 0 = IS_ACTING, 1 = TRUNK_IS_CLOSED, 2 = TRUNK_IS_OPENED
            0 => "is_acting",
            1 => "close",
            2 => "open",
            _ => "unknown",
        };

        crate::DdsData {
            name: String::from("BodyTrunkStatus"),
            value: trunk.to_string(),
        }
    }
    fn topic_name() -> String {
        String::from("BodyTrunkStatus")
    }
    fn type_name() -> String {
        String::from("BodyTrunkStatus")
    }
}

#[derive(Debug, dust_dds::topic_definition::type_support::DdsType)]
pub struct ADASObstacleDetectionIsWarning {
    pub value: bool,
}

impl crate::Piccoloable for ADASObstacleDetectionIsWarning {
    fn to_piccolo_dds_data(&self) -> crate::DdsData {
        let v = if self.value { "sensing" } else { "ignoring" };
        crate::DdsData {
            name: String::from("ADASObstacleDetectionIsWarning"),
            value: v.to_string(),
        }
    }
    fn topic_name() -> String {
        String::from("ADASObstacleDetectionIsWarning")
    }
    fn type_name() -> String {
        String::from("ADASObstacleDetectionIsWarning")
    }
}

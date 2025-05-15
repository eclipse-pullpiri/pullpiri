use serde::{Deserialize, Serialize};
use dust_dds::topic_definition::type_support::{DdsType, DdsSerialize, DdsDeserialize};

#[derive(Debug, Clone, Serialize, Deserialize, DdsType, Default)]
pub struct ADASObstacleDetectionIsWarning {
    pub value: bool,
}

use serde::{Deserialize, Serialize};
use dust_dds::topic_definition::type_support::{DdsType, DdsSerialize, DdsDeserialize};

#[derive(Debug, Clone, Serialize, Deserialize, DdsType, Default)]
pub struct BodyTrunkStatus {
    pub status: i32,
    pub command: i32,
    pub progress: i32,
    pub uistatus: i32,
}

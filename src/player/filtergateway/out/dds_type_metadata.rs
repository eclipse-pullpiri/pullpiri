// 자동 생성된 DDS 타입 메타데이터
use std::collections::HashMap;

pub struct TypeMetadata {
    pub name: String,
    pub module: String,
    pub fields: HashMap<String, String>,
}

pub fn get_type_metadata() -> HashMap<String, TypeMetadata> {
    let mut metadata = HashMap::new();
    let mut fields;
    fields = HashMap::new();
    fields.insert("uistatus".to_string(), "i32".to_string());
    fields.insert("command".to_string(), "i32".to_string());
    fields.insert("status".to_string(), "i32".to_string());
    fields.insert("progress".to_string(), "i32".to_string());
    metadata.insert("BodyLightsHeadLampStatus".to_string(), TypeMetadata {
        name: "BodyLightsHeadLampStatus".to_string(),
        module: "BodyLightsHeadLamp".to_string(),
        fields,
    });
    fields = HashMap::new();
    fields.insert("uistatus".to_string(), "i32".to_string());
    fields.insert("status".to_string(), "i32".to_string());
    fields.insert("command".to_string(), "i32".to_string());
    fields.insert("progress".to_string(), "i32".to_string());
    metadata.insert("BodyTrunkStatus".to_string(), TypeMetadata {
        name: "BodyTrunkStatus".to_string(),
        module: "BodyTrunk".to_string(),
        fields,
    });
    fields = HashMap::new();
    fields.insert("value".to_string(), "bool".to_string());
    metadata.insert("ADASObstacleDetectionIsWarning".to_string(), TypeMetadata {
        name: "ADASObstacleDetectionIsWarning".to_string(),
        module: "ADASObstacleDetection".to_string(),
        fields,
    });
    metadata
}

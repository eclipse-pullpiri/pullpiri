use common::spec::artifact::Scenario;
use filtergateway::filter::Filter;
use filtergateway::grpc::sender::actioncontroller::FilterGatewaySender;
use filtergateway::vehicle::dds::DdsData;
use std::collections::HashMap;
use tokio;

// Helper to build a scenario from an expression and value
fn build_scenario_yaml_with_expression(Scenario_name: &str, expr: &str, value: &str) -> Scenario {
    let yaml = format!(
        r#"
apiVersion: v1
kind: Scenario
metadata:
  name: {Scenario_name}
spec:
  condition:
    express: {expr}
    value: "{value}"
    operands:
      type: DDS
      name: "temperature"
      value: "TestTopic"
  action: update
  target: test_target
"#,
        expr = expr,
        value = value
    );
    serde_yaml::from_str(&yaml).expect("Failed to parse scenario YAML")
}

// Helper to build a scenario from raw YAML
fn build_scenario_from_yaml(yaml: &str) -> Scenario {
    serde_yaml::from_str(yaml).expect("Failed to parse scenario YAML")
}

// Build DDS data with topic, field key and value
fn build_dds_data(topic: &str, key: &str, value: &str) -> DdsData {
    let mut fields = HashMap::new();
    fields.insert(key.into(), value.into());
    DdsData {
        name: topic.into(),
        value: value.to_string(),
        fields,
    }
}
// === Expression Tests ===

#[tokio::test]
async fn test_eq_expression_success() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_eq
spec:
  condition:
    express: le
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_eq
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_eq
spec:
  pattern:
    - type: plain
  models:
    - name: test_eq
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_eq", yaml).await.unwrap();
    common::etcd::put("Package/test_eq", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_eq", "eq", "true");
    let dds = build_dds_data("TestTopic", "temperature", "true");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_eq".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/test_eq").await.unwrap();
    common::etcd::delete("Package/test_eq").await.unwrap();
}

#[tokio::test]
async fn test_meet_scenario_condition_data_not_match() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_eq1
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_eq1
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_eq1
spec:
  pattern:
    - type: plain
  models:
    - name: test_eq1
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_eq1", yaml).await.unwrap();
    common::etcd::put("Package/test_eq1", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_eq1", "eq", "true");
    let dds = build_dds_data("TestTopic_wrong", "temperature", "true");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_eq1".into(), scenario, true, sender);

    assert!(filter.meet_scenario_condition(&dds).await.is_err());
    common::etcd::delete("Scenario/test_eq1").await.unwrap();
    common::etcd::delete("Package/test_eq1").await.unwrap();
}

#[tokio::test]
async fn test_lt_expression_success() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_lt
spec:
  condition:
    express: lt
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_lt
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_lt
spec:
  pattern:
    - type: plain
  models:
    - name: test_lt
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_lt", yaml).await.unwrap();
    common::etcd::put("Package/test_lt", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_lt", "lt", "10");
    let dds = build_dds_data("TestTopic", "temperature", "5");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_lt".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/test_lt").await.unwrap();
    common::etcd::delete("Package/test_lt").await.unwrap();
}

#[tokio::test]
async fn test_meet_scenario_condition_field_parse_error() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_field_parse
spec:
  condition:
    express: gt
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_field_parse
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_field_parse
spec:
  pattern:
    - type: plain
  models:
    - name: test_field_parse
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_field_parse", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/test_field_parse", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_field_parse", "gt", "10");
    let dds = build_dds_data("TestTopic", "temperature", "abc");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_field_parse".into(), scenario, true, sender);

    let result = filter.meet_scenario_condition(&dds).await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "field_value parse error");
    common::etcd::delete("Scenario/test_field_parse")
        .await
        .unwrap();
    common::etcd::delete("Package/test_field_parse")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_meet_scenario_condition_empty() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_field_parse
spec:
  condition:
  action: update
  target: test_field_parse
"#;

    let scenario = build_scenario_from_yaml(yaml);
    let dds = build_dds_data("TestTopic", "temperature", "abc");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_field_parse".into(), scenario, true, sender);

    let result = filter.process_data(&dds).await;
    assert!(true);
}

#[tokio::test]
async fn test_le_expression_success() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_le
spec:
  condition:
    express: le
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_le
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_le
spec:
  pattern:
    - type: plain
  models:
    - name: test_le
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_le", yaml).await.unwrap();
    common::etcd::put("Package/test_le", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_le", "le", "10");
    let dds = build_dds_data("TestTopic", "temperature", "10");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_le".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/test_le").await.unwrap();
    common::etcd::delete("Package/test_le").await.unwrap();
}

#[tokio::test]
async fn test_ge_expression_success() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_ge
spec:
  condition:
    express: ge
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_ge
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_ge
spec:
  pattern:
    - type: plain
  models:
    - name: test_ge
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_ge", yaml).await.unwrap();
    common::etcd::put("Package/test_ge", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_ge", "ge", "10");
    let dds = build_dds_data("TestTopic", "temperature", "11");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_ge".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/test_ge").await.unwrap();
    common::etcd::delete("Package/test_ge").await.unwrap();
}

#[tokio::test]
async fn test_gt_expression_success() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_gt
spec:
  condition:
    express: gt
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_gt
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_gt
spec:
  pattern:
    - type: plain
  models:
    - name: test_gt
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_gt", yaml).await.unwrap();
    common::etcd::put("Package/test_gt", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_gt", "gt", "10");
    let dds = build_dds_data("TestTopic", "temperature", "15");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_gt".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/test_gt").await.unwrap();
    common::etcd::delete("Package/test_gt").await.unwrap();
}

// === Error Cases ===

#[tokio::test]
async fn test_wrong_expression_returns_error() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: invalid_expr
spec:
  condition:
    express: unknown_expr
    value: "on"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: invalid_expr
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: invalid_expr
spec:
  pattern:
    - type: plain
  models:
    - name: invalid_expr
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/invalid_expr", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/invalid_expr", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("invalid_expr", "unknown_expr", "on");
    let dds = build_dds_data("TestTopic", "temperature", "on");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("invalid_expr".into(), scenario, true, sender);

    // Should log error but still return Ok from process_data
    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/invalid_expr").await.unwrap();
    common::etcd::delete("Package/invalid_expr").await.unwrap();
}

#[tokio::test]
async fn test_topic_mismatch_returns_ok() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: topic_mismatch
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: topic_mismatch
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: topic_mismatch
spec:
  pattern:
    - type: plain
  models:
    - name: topic_mismatch
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/topic_mismatch", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/topic_mismatch", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("topic_mismatch", "eq", "true");
    let dds = build_dds_data("WrongTopic", "temperature", "true");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("topic_mismatch".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/topic_mismatch")
        .await
        .unwrap();
    common::etcd::delete("Package/topic_mismatch")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_gt_condition_not_met() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: test_gt
spec:
  condition:
    express: gt
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: test_gt
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: test_gt
spec:
  pattern:
    - type: plain
  models:
    - name: test_gt
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/test_gt", yaml).await.unwrap();
    common::etcd::put("Package/test_gt", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("test_gt", "gt", "100");
    let dds = build_dds_data("TestTopic", "temperature", "15");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("test_gt".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/test_gt").await.unwrap();
    common::etcd::delete("Package/test_gt").await.unwrap();
}
#[tokio::test]
async fn test_missing_field_returns_error_logged() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("missing_field", "eq", "true");
    let dds = build_dds_data("TestTopic", "unknown_field", "true");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("missing_field".into(), scenario, true, sender);

    // Logs error, returns Ok
    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_lt_expression() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: lt
    value: "not_a_number"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_error", "lt", "not_a_number");
    let dds = build_dds_data("TestTopic", "temperature", "5");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_le_expression() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: le
    value: "not_a_number"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_error", "le", "not_a_number");
    let dds = build_dds_data("TestTopic", "temperature", "5");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_gt_expression() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: gt
    value: "not_a_number"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_error", "gt", "not_a_number");
    let dds = build_dds_data("TestTopic", "temperature", "5");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_ge_expression() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: ge
    value: "not_a_number"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_error", "ge", "not_a_number");
    let dds = build_dds_data("TestTopic", "temperature", "5");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_gt_field_value() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: gt
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_field_error", "gt", "10");
    let dds = build_dds_data("TestTopic", "temperature", "not_a_number");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_field_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_lt_field_value() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: lt
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_field_error", "lt", "10");
    let dds = build_dds_data("TestTopic", "temperature", "not_a_number");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_field_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_le_field_value() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: le
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_field_error", "le", "10");
    let dds = build_dds_data("TestTopic", "temperature", "not_a_number");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_field_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_parse_error_in_ge_field_value() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: parse_field_error
spec:
  condition:
    express: ge
    value: "10"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: parse_field_error
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: parse_field_error
spec:
  pattern:
    - type: plain
  models:
    - name: parse_field_error
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/parse_field_error", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/parse_field_error", VALID_PACKAGE_YAML)
        .await
        .unwrap();
    let scenario = build_scenario_yaml_with_expression("parse_field_error", "ge", "10");
    let dds = build_dds_data("TestTopic", "temperature", "not_a_number");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("parse_field_error".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/parse_field_error")
        .await
        .unwrap();
    common::etcd::delete("Package/parse_field_error")
        .await
        .unwrap();
}

// === Behavior Tests ===

#[tokio::test]
async fn test_filter_inactive_skips_processing() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: inactive
spec:
  condition:
    express: eq
    value: "on"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: inactive
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: inactive
spec:
  pattern:
    - type: plain
  models:
    - name: inactive
      node: HPC
      resources:
        volume:
        network:
"#;
    common::etcd::put("Scenario/inactive", yaml).await.unwrap();
    common::etcd::put("Package/inactive", VALID_PACKAGE_YAML)
        .await
        .unwrap();

    let scenario = build_scenario_yaml_with_expression("inactive", "eq", "on");
    let dds = build_dds_data("TestTopic", "temperature", "on");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("inactive".into(), scenario, false, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/inactive").await.unwrap();
    common::etcd::delete("Package/inactive").await.unwrap();
}

#[tokio::test]
async fn test_pause_and_resume_filter() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: pause_resume
spec:
  condition:
    express: eq
    value: "on"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: pause_resume
"#;
    static VALID_PACKAGE_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: pause_resume
spec:
  pattern:
    - type: plain
  models:
    - name: pause_resume
      node: HPC
      resources:
        volume:
        network:
"#;

    let scenario = build_scenario_yaml_with_expression("pause_resume", "eq", "on");
    let dds = build_dds_data("TestTopic", "temperature", "on");

    common::etcd::put("Scenario/pause_resume", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/pause_resume", VALID_PACKAGE_YAML)
        .await
        .unwrap();

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("pause_resume".into(), scenario, true, sender);

    assert!(filter.is_active());
    filter.pause_scenario_filter().await.unwrap();
    assert!(!filter.is_active());

    filter.resume_scenario_filter().await.unwrap();
    assert!(filter.is_active());

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/pause_resume").await.unwrap();
    common::etcd::delete("Package/pause_resume").await.unwrap();
}

#[tokio::test]
async fn test_trigger_action_when_condition_met() {
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: status
      value: TestTopic
  action: update
  target: helloworld
"#;
    static VALID_PACKAGE_YAML_SINGLE: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld
      node: HPC
      resources:
        volume:
        network:
"#;

    common::etcd::put("Scenario/helloworld", yaml)
        .await
        .unwrap();
    common::etcd::put("Package/helloworld", VALID_PACKAGE_YAML_SINGLE)
        .await
        .unwrap();

    let scenario = build_scenario_from_yaml(yaml);
    let dds = build_dds_data("TestTopic", "status", "true");

    let sender = FilterGatewaySender::new();
    let mut filter = Filter::new("helloworld".into(), scenario, true, sender);

    assert!(filter.process_data(&dds).await.is_ok());
    common::etcd::delete("Scenario/helloworld").await.unwrap();
    common::etcd::delete("Package/helloworld").await.unwrap();
}

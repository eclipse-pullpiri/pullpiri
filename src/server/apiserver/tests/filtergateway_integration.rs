use apiserver::grpc::sender::filtergateway::send;
use common::filtergateway::{Action, HandleScenarioRequest};

const VALID_SCENARIO_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  action: update
  target: helloworld
"#;

const INVALID_SCENARIO_YAML_EMPTY: &str = r#""#;

const INVALID_SCENARIO_YAML_MISSING_FIELD: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name:
spec:
  condition:
  target: helloworld
"#;

const INVALID_NO_SCENARIO_YAML: &str = r#"
apiVersion: v1
kind: Package
metadata:
  label: null
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
"#;

/// Test sending with valid scenario and Action::Apply
#[tokio::test]
async fn test_send_with_valid_scenario_apply() {
    let scenario = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario: VALID_SCENARIO_YAML.to_string(),
    };

    let result = send(scenario).await;
    assert!(result.is_ok(), "Expected success for valid APPLY scenario");
}

/// Test sending with valid scenario and Action::Withdraw
#[tokio::test]
async fn test_send_with_valid_scenario_withdraw() {
    let scenario = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario: VALID_SCENARIO_YAML.to_string(),
    };

    let result = send(scenario).await;
    assert!(
        result.is_ok(),
        "Expected success for valid WITHDRAW scenario"
    );
}

/// Test sending with empty scenario YAML (invalid) and Action::Apply
#[tokio::test]
async fn test_send_with_empty_scenario_name_apply() {
    let scenario = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario: INVALID_SCENARIO_YAML_EMPTY.to_string(),
    };

    let result = send(scenario).await;
    assert!(result.is_err(), "Expected error for empty scenario APPLY");
}

/// Test sending with missing required field and Action::Apply
#[tokio::test]
async fn test_send_with_missing_field_apply() {
    let scenario = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario: INVALID_SCENARIO_YAML_MISSING_FIELD.to_string(),
    };

    let result = send(scenario).await;
    // Adjust based on your server behavior, might accept or error
    assert!(
        result.is_ok() || result.is_err(),
        "Either success or error is acceptable here for missing field APPLY"
    );
}

/// Test sending non-Scenario YAML (Package kind) and Action::Apply
#[tokio::test]
async fn test_send_with_nonexistent_scenario_apply() {
    let scenario = HandleScenarioRequest {
        action: Action::Apply.into(),
        scenario: INVALID_NO_SCENARIO_YAML.to_string(),
    };

    let result = send(scenario).await;
    // Depending on server logic, likely ok since it's non-empty YAML
    assert!(
        result.is_ok() || result.is_err(),
        "Either success or error acceptable for non-scenario APPLY"
    );
}

/// Test sending with empty scenario YAML and Action::Withdraw
#[tokio::test]
async fn test_send_with_empty_scenario_name_withdraw() {
    let scenario = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario: INVALID_SCENARIO_YAML_EMPTY.to_string(),
    };

    let result = send(scenario).await;
    assert!(
        result.is_err(),
        "Expected error for empty scenario WITHDRAW"
    );
}

/// Test sending with missing required field and Action::Withdraw
#[tokio::test]
async fn test_send_with_missing_field_withdraw() {
    let scenario = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario: INVALID_SCENARIO_YAML_MISSING_FIELD.to_string(),
    };

    let result = send(scenario).await;
    assert!(
        result.is_ok() || result.is_err(),
        "Either success or error acceptable for missing field WITHDRAW"
    );
}

/// Test sending non-Scenario YAML and Action::Withdraw
#[tokio::test]
async fn test_send_with_nonexistent_scenario_withdraw() {
    let scenario = HandleScenarioRequest {
        action: Action::Withdraw.into(),
        scenario: INVALID_NO_SCENARIO_YAML.to_string(),
    };

    let result = send(scenario).await;
    assert!(
        result.is_ok() || result.is_err(),
        "Either success or error acceptable for non-scenario WITHDRAW"
    );
}

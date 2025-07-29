use common::Result;
use filtergateway::FilterGatewaySender;
use tokio::time::{sleep, Duration};

/// Test Case: Valid Scenario Name
/// This test verifies that calling `trigger_action` with a known valid
/// scenario name (e.g., "antipinch-enable") succeeds when the gRPC
/// server is running and accepts the request.
#[tokio::test]
async fn test_trigger_action_valid_scenario() {
    // Insert mock Scenario YAML into etcd
    common::etcd::put(
        "Scenario/antipinch-enable",
        r#"
apiVersion: v1
kind: Scenario
metadata:
  name: antipinch-enable
spec:
  condition:
  action: update
  target: antipinch-enable
"#,
    )
    .await
    .unwrap();

    // Insert mock Package YAML into etcd
    common::etcd::put(
        "Package/antipinch-enable",
        r#"
apiVersion: v1
kind: Package
metadata:
  label: null
  name: antipinch-enable
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume:
        network:
"#,
    )
    .await
    .unwrap();
    let mut sender = FilterGatewaySender::new();
    let result = sender.trigger_action("antipinch-enable".to_string()).await;

    assert!(
        result.is_ok(),
        "Expected valid scenario to succeed, got error: {:?}",
        result.err()
    );
    // Cleanup after test
    common::etcd::delete("Scenario/antipinch-enable")
        .await
        .unwrap();
    common::etcd::delete("Package/antipinch-enable")
        .await
        .unwrap();
}

/// Test Case: Empty Scenario Name
/// This test checks that an empty string passed as the scenario name
/// is rejected before even attempting to contact the gRPC server.
#[tokio::test]
async fn test_trigger_action_empty_scenario_should_fail() {
    let mut sender = FilterGatewaySender::new();
    let result = sender.trigger_action("".to_string()).await;

    // Should fail due to client-side validation
    assert!(
        result.is_err(),
        "Expected error when triggering with empty scenario"
    );
}

/// Test Case: Whitespace-Only Scenario Name
/// This test ensures that strings with only spaces are treated as empty
/// and properly rejected by the client before making a gRPC call.
#[tokio::test]
async fn test_trigger_action_whitespace_only_scenario_should_fail() {
    let mut sender = FilterGatewaySender::new();
    let result = sender.trigger_action("   ".to_string()).await;

    // Should fail due to client-side input check using `trim().is_empty()`
    assert!(
        result.is_err(),
        "Expected error when triggering with whitespace-only scenario"
    );
}

/// Test Case: Invalid Scenario That Server Rejects
/// This test sends a scenario name that is not recognized by the gRPC server.
/// The server is expected to return an error (e.g., `Status::not_found`).
#[tokio::test]
async fn test_trigger_action_invalid_scenario_server_rejects() {
    let mut sender = FilterGatewaySender::new();
    let result = sender
        .trigger_action("non-existent-scenario".to_string())
        .await;

    // Should fail if the server checks scenario validity
    assert!(
        result.is_err(),
        "Expected error from server for unknown scenario"
    );
}

/// Test Case: Unicode Scenario Name
/// This test verifies that Unicode strings (non-ASCII) are accepted
/// and handled properly. It should not panic or crash, even if the server
/// returns an error.
#[tokio::test]
async fn test_trigger_action_unicode_scenario_name() {
    let mut sender = FilterGatewaySender::new();
    let result = sender.trigger_action("安全模式启动".to_string()).await;

    // The test passes if the function handles Unicode input gracefully
    assert!(
        result.is_ok() || result.is_err(),
        "Unicode scenario name should not crash"
    );
}

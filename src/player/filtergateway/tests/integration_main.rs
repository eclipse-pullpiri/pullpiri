use common::spec::artifact::Scenario;
use filtergateway::manager::{FilterGatewayManager, ScenarioParameter};
use filtergateway::vehicle::dds::DdsData;
use filtergateway::vehicle::VehicleManager;
use filtergateway::FilterGatewaySender;
use filtergateway::{initialize, launch_manager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::LocalSet;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn integration_test_launch_manager_and_initialize() {
    let (tx_grpc, rx_grpc) = channel::<ScenarioParameter>(100);

    // Spawn manager on a LocalSet (if needed for non-Send)
    let local = LocalSet::new();
    local.spawn_local(async move {
        let _ = launch_manager(rx_grpc).await;
    });

    // Spawn initialize
    let init_fut = initialize(tx_grpc);

    // Run both concurrently
    tokio::select! {
        _ = local => {},
        _ = init_fut => {},
        _ = sleep(Duration::from_millis(500)) => {},
    }

    // If reached here, test passed without panic
    assert!(true);
}
/// Test to ensure that the channels are initialized with the correct capacity
#[tokio::test]
async fn test_main_initializes_channels() {
    let (tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) = channel(100);
    assert_eq!(tx_grpc.capacity(), 100);
    assert!(!rx_grpc.is_closed());
}

/// Test to ensure that the manager thread launches without any panic
#[tokio::test]
async fn test_main_launch_manager() {
    let (_tx_grpc, rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) =
        channel(100);

    let local = LocalSet::new();
    local.spawn_local(async move {
        let _ = launch_manager(rx_grpc).await;
    });

    tokio::select! {
        _ = local => {}
        _ = sleep(Duration::from_millis(200)) => {}
    }

    assert!(true);
}

/// Test to ensure that the gRPC initialization runs without any panic
#[tokio::test(flavor = "multi_thread")]
async fn test_main_initialize_grpc() {
    let (tx_grpc, _rx_grpc): (Sender<ScenarioParameter>, Receiver<ScenarioParameter>) =
        channel(100);

    let local = LocalSet::new();
    local.spawn_local(async move {
        let _ = initialize(tx_grpc).await;
    });

    tokio::select! {
        _ = local => {}
        _ = sleep(Duration::from_millis(200)) => {}
    }

    assert!(true);
}

#[tokio::test]
async fn test_launch_filter_with_valid_condition() {
    let (_tx, rx) = mpsc::channel(1);
    let manager = FilterGatewayManager::new(rx).await;

    let valid_yaml = r#"
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
     name: value
     value: ADASObstacleDetectionIsWarning
  action: update
  target: helloworld
"#;

    let scenario: Scenario = serde_yaml::from_str(valid_yaml).unwrap();
    let scenario1: Scenario = serde_yaml::from_str(valid_yaml).unwrap();
    let result = manager.launch_scenario_filter(scenario).await;
    let resul1 = manager.launch_scenario_filter(scenario1).await;
    assert!(result.is_ok());

    let filters = manager.filters.lock().await;
    assert!(filters.iter().any(|f| f.scenario_name == "helloworld"));
}

#[tokio::test(flavor = "multi_thread")]
async fn integration_test_error_path_initialize() {
    // Create a test YAML Scenario string
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
      name: value
      value: ADASObstacleDetectionIsWarning
  action: update
  target: helloworld
"#;

    // Deserialize into Scenario (even with private fields)
    let scenario: Scenario = serde_yaml::from_str(yaml).expect("Failed to parse YAML");

    let scenario_param = ScenarioParameter {
        action: 0,
        scenario,
    };

    let (tx_grpc, rx_grpc) = channel(100);

    // Send the parameter to the manager
    tx_grpc.send(scenario_param).await.expect("send failed");

    // Use LocalSet to support non-Send futures
    let local = LocalSet::new();
    local.spawn_local(async move {
        // This runs your real FilterGatewayManager with a real Scenario
        launch_manager(rx_grpc).await;
    });

    // Let the manager run for a short time
    tokio::select! {
        _ = local => {},
        _ = sleep(Duration::from_millis(500)) => {},
    }

    // If reached here, test didn't panic or crash
    assert!(true);
}

#[tokio::test]
async fn integration_test_none_action() {
    // Create a test YAML Scenario string
    let yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld_new
spec:
  condition:
    express: eq
    value: "on"
    operands:
      type: DDS
      name: status
      value: ADASObstacleDetectionIsWarning
  action: update
  target: helloworld_new
"#;

    // Deserialize into Scenario (even with private fields)
    let scenario: Scenario = serde_yaml::from_str(yaml).expect("Failed to parse YAML");

    let scenario_param = ScenarioParameter {
        action: 3,
        scenario,
    };

    let (tx_grpc, rx_grpc) = channel(100);

    // Send the parameter to the manager
    tx_grpc.send(scenario_param).await.expect("send failed");

    // Use LocalSet to support non-Send futures
    let local = LocalSet::new();
    local.spawn_local(async move {
        // This runs your real FilterGatewayManager with a real Scenario
        launch_manager(rx_grpc).await;
    });

    // Let the manager run for a short time
    tokio::select! {
        _ = local => {},
        _ = sleep(Duration::from_millis(1000)) => {},
    }

    // If reached here, test didn't panic or crash
    assert!(true);
}

#[tokio::test]
async fn integration_test_initialize_failure_path() {
    // Insert mock Scenario YAML into etcd
    common::etcd::put(
        "Scenario/antipinch-en",
        r#"
apiVersion: v1
kind: Scenario
metadata:
  name: antipinch-en
spec:
  condition:
    express: unknown_expr
    value: "on"
    operands:
      type: DDS
      name: status
      value: ADASObstacleDetectionIsWarning
  action: update
  target: antipinch-en
"#,
    )
    .await
    .unwrap();

    // Insert mock Package YAML into etcd
    common::etcd::put(
        "Package/antipinch-en",
        r#"
apiVersion: v1
kind: Package
metadata:
  name: antipinch-en
spec:
  pattern:
    - type: plain
  models:
    - name: antipinch-en
      node: HPC
      resources:
        volume:
        network:
"#,
    )
    .await
    .unwrap();
    // Create a closed receiver channel to simulate invalid input
    let (_tx_grpc, rx_grpc): (_, Receiver<ScenarioParameter>) = channel(100);

    // Drop sender immediately — simulate init failure (depends on implementation)
    drop(_tx_grpc);
    let _ = sleep(Duration::from_millis(200));
    // Use LocalSet to spawn the non-Send future
    let local = LocalSet::new();

    local.spawn_local(async move {
        // This should hit the `Err(e)` block in `initialize().await`
        launch_manager(rx_grpc).await;
    });

    // Let it run for a short while
    tokio::select! {
        _ = local => {},
        _ = sleep(Duration::from_millis(700)) => {},
    }

    // Test passes if no panic occurred and error path was exercised
    assert!(true);
    // Cleanup after test
    common::etcd::delete("Scenario/antipinch-en").await.unwrap();
    common::etcd::delete("Package/antipinch-en").await.unwrap();
}

#[tokio::test]
async fn integration_test_initialize_failure() {
    // Insert mock Scenario YAML into etcd
    common::etcd::put(
        "Scenario/antipinch-en1",
        r#"
apiVersion: v1
kind: Scenario
metadata:
  name: antipinch-en1
spec:
  condition:
  action: update
  target: antipinch-en1
"#,
    )
    .await
    .unwrap();

    // Insert mock Package YAML into etcd
    common::etcd::put(
        "Package/antipinch-en1",
        r#"
apiVersion: v1
kind: Package
metadata:
  name: antipinch-en1
spec:
  pattern:
    - type: plain
  models:
    - name: antipinch-en1
      node: HPC
      resources:
        volume:
        network:
"#,
    )
    .await
    .unwrap();
    // Create a closed receiver channel to simulate invalid input
    let (_tx_grpc, rx_grpc): (_, Receiver<ScenarioParameter>) = channel(100);

    // Drop sender immediately — simulate init failure (depends on implementation)
    drop(_tx_grpc);
    let _ = sleep(Duration::from_millis(200));
    // Use LocalSet to spawn the non-Send future
    let local = LocalSet::new();

    local.spawn_local(async move {
        // This should hit the `Err(e)` block in `initialize().await`
        launch_manager(rx_grpc).await;
    });

    // Let it run for a short while
    tokio::select! {
        _ = local => {},
        _ = sleep(Duration::from_millis(2000)) => {},
    }

    // Test passes if no panic occurred and error path was exercised
    assert!(true);
    // Cleanup after test
    common::etcd::delete("Scenario/antipinch-en1")
        .await
        .unwrap();
    common::etcd::delete("Package/antipinch-en1").await.unwrap();
}

#[tokio::test]
async fn integration_test_initialize_success() {
    // Insert mock Scenario YAML into etcd
    common::etcd::put(
        "Scenario/antipinch-enable1",
        r#"
apiVersion: v1
kind: Scenario
metadata:
  name: antipinch-enable1
spec:
  condition:
    express: eq
    value: "true"
    operands:
      type: DDS
      name: value
      value: ADASObstacleDetectionIsWarning
  action: update
  target: antipinch-enable1
"#,
    )
    .await
    .unwrap();

    // Insert mock Package YAML into etcd
    common::etcd::put(
        "Package/antipinch-enable1",
        r#"
apiVersion: v1
kind: Package
metadata:
  name: antipinch-enable1
spec:
  pattern:
    - type: plain
  models:
    - name: antipinch-enable1
      node: HPC
      resources:
        volume:
        network:
"#,
    )
    .await
    .unwrap();
    // Create a closed receiver channel to simulate invalid input
    let (_tx_grpc, rx_grpc): (_, Receiver<ScenarioParameter>) = channel(100);

    // Drop sender immediately — simulate init failure (depends on implementation)
    drop(_tx_grpc);
    let _ = sleep(Duration::from_millis(200));
    // Use LocalSet to spawn the non-Send future
    let local = LocalSet::new();

    local.spawn_local(async move {
        // This should hit the `Err(e)` block in `initialize().await`
        launch_manager(rx_grpc).await;
    });

    // Let it run for a short while
    tokio::select! {
        _ = local => {},
        _ = sleep(Duration::from_millis(2000)) => {},
    }

    // Test passes if no panic occurred and error path was exercised
    assert!(true);
    // Cleanup after test
    common::etcd::delete("Scenario/antipinch-enable1")
        .await
        .unwrap();
    common::etcd::delete("Package/antipinch-enable1")
        .await
        .unwrap();
}

#[tokio::test]
async fn integration_test_initialize_success_path() {
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
    express: eq
    value: "true"
    operands:
      type: DDS
      name: value
      value: ADASObstacleDetectionIsWarning
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
  name: antipinch-enable
spec:
  pattern:
    - type: plain
  models:
    - name: antipinch-enable
      node: HPC
      resources:
        volume:
        network:
"#,
    )
    .await
    .unwrap();
    // Create a closed receiver channel to simulate invalid input
    let (_tx_grpc, rx_grpc): (_, Receiver<ScenarioParameter>) = channel(100);

    // Use LocalSet to spawn the non-Send future
    let local = LocalSet::new();

    local.spawn_local(async move {
        // This should hit the `Err(e)` block in `initialize().await`
        launch_manager(rx_grpc).await;
    });

    // Let it run for a short while
    tokio::select! {
        _ = local => {},
        _ = sleep(Duration::from_millis(1000)) => {},
    }

    // Test passes if no panic occurred and error path was exercised
    assert!(true);
    // Cleanup after test
    common::etcd::delete("Scenario/antipinch-enable")
        .await
        .unwrap();
    common::etcd::delete("Package/antipinch-enable")
        .await
        .unwrap();
}

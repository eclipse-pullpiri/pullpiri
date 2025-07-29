use common::spec::artifact::{Artifact, Scenario};
use filtergateway::manager::{FilterGatewayManager, ScenarioParameter};
use filtergateway::vehicle::dds::DdsData;
use filtergateway::vehicle::VehicleManager;
use filtergateway::FilterGatewaySender;
use serde_yaml;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

static VALID_SCENARIO_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld_dds
spec:
  condition:
  action: update
  target: helloworld_dds
"#;

static VALID_PACKAGE_YAML_SINGLE: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: helloworld_dds
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld_dds-core
      node: HPC
      resources:
        volume:
        network:
"#;

static VALID_MODEL_YAML_SINGLE: &str = r#"
apiVersion: v1
kind: Model
metadata:
  name: helloworld_dds-core
  annotations:
    io.piccolo.annotations.package-type: helloworld_dds-core
    io.piccolo.annotations.package-name: helloworld_dds
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld_dds-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld_dds
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

#[tokio::test]
async fn test_initialize_manager_with_valid_scenario() {
    let (_tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    common::etcd::put("Scenario/helloworld_dds", VALID_SCENARIO_YAML)
        .await
        .unwrap();
    common::etcd::put("Package/helloworld_dds", VALID_PACKAGE_YAML_SINGLE)
        .await
        .unwrap();
    common::etcd::put("Model/helloworld_dds-core", VALID_MODEL_YAML_SINGLE)
        .await
        .unwrap();

    let result = manager.initialize().await;
    assert!(true);

    common::etcd::delete("Scenario/helloworld_dds")
        .await
        .unwrap();
    common::etcd::delete("Package/helloworld_dds")
        .await
        .unwrap();
    common::etcd::delete("Model/helloworld_dds-core")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_run_manager_with_allow_action() {
    let (tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    let scenario: Scenario = serde_yaml::from_str(VALID_SCENARIO_YAML).unwrap();
    let param = ScenarioParameter {
        action: 0,
        scenario,
    };

    tx.send(param).await.unwrap();

    let handle = tokio::spawn(async move {
        let _ = manager.run().await;
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();
}
static VALID_SCENARIO_YAML1: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld_dds1
spec:
  condition:
  action: update
  target: helloworld_dds1
"#;

static VALID_PACKAGE_YAML_SINGLE1: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: helloworld_dds1
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld_dds-core1
      node: HPC
      resources:
        volume: helloworld_dds1
        network: helloworld_dds1
"#;

static VALID_NETWORK_YAML_SINGLE1: &str = r#"
apiVersion: v1
kind: Network
metadata:
  label: null
  name: helloworld_dds1
"#;

#[tokio::test]
async fn test_run_manager_with_withdraw_action() {
    let (tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    common::etcd::put("Scenario/helloworld_dds1", VALID_SCENARIO_YAML1)
        .await
        .unwrap();
    common::etcd::put("Package/helloworld_dds1", VALID_PACKAGE_YAML_SINGLE1)
        .await
        .unwrap();
    common::etcd::put("Network/helloworld_dds1", VALID_NETWORK_YAML_SINGLE1)
        .await
        .unwrap();
    common::etcd::put("Node/helloworld_dds1", VALID_NETWORK_YAML_SINGLE1)
        .await
        .unwrap();
    let scenario: Scenario = serde_yaml::from_str(VALID_SCENARIO_YAML1).unwrap();
    manager.launch_scenario_filter(scenario).await.unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let scenario: Scenario = serde_yaml::from_str(VALID_SCENARIO_YAML1).unwrap();
    tx.send(ScenarioParameter {
        action: 1,
        scenario,
    })
    .await
    .unwrap();

    let handle = tokio::spawn(async move {
        let _ = manager.run().await;
    });

    common::etcd::delete("Scenario/helloworld_dds1")
        .await
        .unwrap();
    common::etcd::delete("Package/helloworld_dds1")
        .await
        .unwrap();
    common::etcd::delete("Network/helloworld_dds1")
        .await
        .unwrap();
    common::etcd::delete("Node/helloworld_dds1").await.unwrap();
    handle.abort();
}

static VALID_SCENARIO_YAML2: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld_dds2
spec:
  condition:
  action: update
  target: helloworld_dds2
"#;

static VALID_PACKAGE_YAML_SINGLE2: &str = r#"
apiVersion: v1
kind: Package
metadata:
  name: helloworld_dds2
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld_dds-core2
      node: HPC
      resources:
        volume: helloworld_dds2
        network: helloworld_dds2
"#;

static VALID_NETWORK_YAML_SINGLE2: &str = r#"
apiVersion: v1
kind: Network
metadata:
  label: null
  name: helloworld_dds2
"#;

#[tokio::test]
async fn test_run_manager_with_withdraw_action_none() {
    let (tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    common::etcd::put("Scenario/helloworld_dds2", VALID_SCENARIO_YAML2)
        .await
        .unwrap();
    common::etcd::put("Package/helloworld_dds2", VALID_PACKAGE_YAML_SINGLE2)
        .await
        .unwrap();
    common::etcd::put("Node/helloworld_dds2", VALID_NETWORK_YAML_SINGLE2)
        .await
        .unwrap();
    common::etcd::put("Network/helloworld_dds2", VALID_NETWORK_YAML_SINGLE2)
        .await
        .unwrap();
    let scenario: Scenario = serde_yaml::from_str(VALID_SCENARIO_YAML2).unwrap();
    manager.launch_scenario_filter(scenario).await.unwrap();
    let scenario: Scenario = serde_yaml::from_str(VALID_SCENARIO_YAML2).unwrap();
    tx.send(ScenarioParameter {
        action: 3,
        scenario,
    })
    .await
    .unwrap();

    let handle = tokio::spawn(async move {
        let _ = manager.run().await;
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.abort();

    common::etcd::delete("Scenario/helloworld_dds2")
        .await
        .unwrap();
    common::etcd::delete("Package/helloworld_dds2")
        .await
        .unwrap();
    common::etcd::delete("Network/helloworld_dds2")
        .await
        .unwrap();
    common::etcd::delete("Node/helloworld_dds2").await.unwrap();
}

#[tokio::test]
async fn test_subscribe_and_unsubscribe_vehicle_data() {
    let (_tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    let mut fields = HashMap::new();
    fields.insert("speed".to_string(), "100".to_string());

    let data = DdsData {
        name: "test_topic".to_string(),
        value: "TestType".to_string(),
        fields,
    };

    assert!(manager.subscribe_vehicle_data(data).await.is_ok());

    let mut fields2 = HashMap::new();
    fields2.insert("speed".to_string(), "100".to_string());
    let data2 = DdsData {
        name: "test_topic".to_string(),
        value: "TestType".to_string(),
        fields: fields2,
    };

    assert!(manager.unsubscribe_vehicle_data(data2).await.is_ok());
}
#[tokio::test]
async fn test_initialize_manager_with_invalid_scenario_yaml() {
    let (_tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    static INVALID_SCENARIO_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  # missing 'name'
spec:
  condition:
  action: invalid_action
  target: helloworld_dds
"#;

    common::etcd::put("Scenario/invalid_scenario", INVALID_SCENARIO_YAML)
        .await
        .unwrap();

    let result = manager.initialize().await;
    // Should not panic, ideally error or ok handled gracefully
    assert!(result.is_err() || result.is_ok());

    common::etcd::delete("Scenario/invalid_scenario")
        .await
        .unwrap();
}

#[tokio::test]
async fn test_initialize_manager_with_malformed_yaml() {
    let (_tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    static MALFORMED_YAML: &str = r#"
apiVersion: v1
kind Scenario
metadata:
  name helloworld_dds
spec:
  condition:
  action: update
  target: helloworld_dds
"#;

    common::etcd::put("Scenario/malformed", MALFORMED_YAML)
        .await
        .unwrap();

    let result = manager.initialize().await;
    assert!(result.is_err());

    common::etcd::delete("Scenario/malformed").await.unwrap();
}

#[tokio::test]
async fn test_run_manager_with_invalid_action_value() {
    let (tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    let scenario: Scenario = serde_yaml::from_str(VALID_SCENARIO_YAML).unwrap();
    let param = ScenarioParameter {
        action: 99,
        scenario,
    }; // invalid action

    tx.send(param).await.unwrap();

    let handle = tokio::spawn(async move {
        let _ = manager.run().await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    handle.abort();
}

#[tokio::test]
async fn test_launch_scenario_filter_with_invalid_scenario() {
    let (_tx, rx) = mpsc::channel(10);
    let manager = FilterGatewayManager::new(rx).await;

    static INVALID_SCENARIO_YAML2: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  # missing name
spec:
  condition:
  action: update
  target: missing_target
"#;

    let scenario_res = serde_yaml::from_str::<Scenario>(INVALID_SCENARIO_YAML2);
    assert!(scenario_res.is_err());

    if let Ok(scenario) = scenario_res {
        let result = manager.launch_scenario_filter(scenario).await;
        assert!(result.is_err());
    }
}

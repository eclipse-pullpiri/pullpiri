/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Convert string-type artifacts to struct and access etcd

pub mod data;

use common::logd;
use common::spec::artifact::{Artifact, Model, Network, Node, Package, Scenario, Schedule, Volume};
use common::spec::k8s::Pod;

// Artifact kind constants
const KIND_SCENARIO: &str = "Scenario";
const KIND_PACKAGE: &str = "Package";
const KIND_VOLUME: &str = "Volume";
const KIND_NETWORK: &str = "Network";
const KIND_NODE: &str = "Node";
const KIND_MODEL: &str = "Model";
const KIND_SCHEDULE: &str = "Schedule";

// YAML document separator
const YAML_SEPARATOR: &str = "---";

/// Parse artifact kind and name from YAML value
fn parse_artifact_info(value: &serde_yaml::Value) -> Option<(String, String)> {
    let kind = value.get("kind")?.as_str()?;

    let name = match kind {
        KIND_SCENARIO => serde_yaml::from_value::<Scenario>(value.clone())
            .ok()?
            .get_name(),
        KIND_PACKAGE => serde_yaml::from_value::<Package>(value.clone())
            .ok()?
            .get_name(),
        KIND_VOLUME => serde_yaml::from_value::<Volume>(value.clone())
            .ok()?
            .get_name(),
        KIND_NETWORK => serde_yaml::from_value::<Network>(value.clone())
            .ok()?
            .get_name(),
        KIND_NODE => serde_yaml::from_value::<Node>(value.clone())
            .ok()?
            .get_name(),
        KIND_MODEL => serde_yaml::from_value::<Model>(value.clone())
            .ok()?
            .get_name(),
        KIND_SCHEDULE => serde_yaml::from_value::<Schedule>(value.clone())
            .ok()?
            .get_name(),
        _ => return None,
    };

    Some((kind.to_string(), name))
}

/// Send initial state change notification to StateManager
async fn notify_scenario_state(scenario_name: &str, target_state: &str) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64;

    let state_change = common::statemanager::StateChange {
        resource_type: common::statemanager::ResourceType::Scenario as i32,
        resource_name: scenario_name.to_string(),
        current_state: String::new(),
        target_state: target_state.to_string(),
        transition_id: format!("apiserver-scenario-init-{}", timestamp),
        timestamp_ns: timestamp,
        source: "apiserver".to_string(),
    };

    logd!(
        1,
        "🔄 SCENARIO STATE INITIALIZATION: ApiServer Setting Initial State"
    );
    logd!(1, "   📋 Scenario: {}", scenario_name);
    logd!(1, "   🔄 Initial State: → {}", target_state);
    logd!(1, "   📤 Sending StateChange to StateManager");

    let mut state_sender = crate::grpc::sender::statemanager::StateManagerSender::new();
    match state_sender.send_state_change(state_change).await {
        Ok(_) => logd!(
            2,
            "   ✅ Successfully set scenario {} to {} state",
            scenario_name,
            target_state
        ),
        Err(e) => logd!(
            5,
            "   ❌ Failed to send state change to StateManager: {:?}",
            e
        ),
    }
}

/// Send resource YAML to ResourceManager (Network or Volume)
async fn send_resource_to_resourcemanager(artifact_str: &str, kind: &str) {
    use common::resourcemanager::Action;

    logd!(1, "RESOURCE: Sending {} YAML to ResourceManager", kind);

    let mut sender = crate::grpc::sender::resourcemanager::ResourceManagerSender::new();
    match sender.send(artifact_str.to_string(), Action::Apply).await {
        Ok(response) => {
            let resp = response.into_inner();
            if resp.success {
                logd!(1, "   Successfully sent {} resource to ResourceManager", kind);
            } else {
                logd!(5, "   {} resource handling failed: {}", kind, resp.message);
            }
        }
        Err(e) => logd!(5, "   Failed to send {} to ResourceManager: {:?}", kind, e),
    }
}

/// Process and store a single artifact document
async fn process_artifact_document(doc: &str) -> common::Result<Option<(String, String)>> {
    use std::time::Instant;

    let parse_start = Instant::now();
    let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
    let artifact_str = serde_yaml::to_string(&value)?;
    logd!(
        1,
        "process_artifact: YAML parse elapsed = {:?}",
        parse_start.elapsed()
    );

    let (kind, name) = match parse_artifact_info(&value) {
        Some(info) => info,
        None => {
            logd!(5, "Unknown or invalid artifact");
            return Ok(None);
        }
    };

    let key = format!("{}/{}", kind, name);

    let etcd_start = Instant::now();
    data::write_to_etcd(&key, &artifact_str).await?;
    logd!(
        1,
        "process_artifact: etcd write elapsed for {} = {:?}",
        key,
        etcd_start.elapsed()
    );

    // Handle artifact-specific actions
    match kind.as_str() {
        KIND_SCENARIO => {
            notify_scenario_state(&name, "idle").await;
        }
        KIND_NETWORK | KIND_VOLUME => {
            // Send raw YAML to ResourceManager for parsing
            send_resource_to_resourcemanager(&artifact_str, &kind).await;
        }
        _ => {}
    }

    Ok(Some((kind, artifact_str)))
}

/// Apply downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(String, String)` - scenario and package yaml in downloaded artifact
/// ### Description
/// Write artifact in etcd
pub async fn apply(body: &str) -> common::Result<String> {
    use std::time::Instant;
    let total_start = Instant::now();

    let docs: Vec<&str> = body.split(YAML_SEPARATOR).collect();
    let mut scenario_str = String::new();
    let mut package_str = String::new();
    let mut resource_processed = false;

    for doc in docs {
        if let Some((kind, artifact_str)) = process_artifact_document(doc).await? {
            match kind.as_str() {
                KIND_SCENARIO => scenario_str = artifact_str,
                KIND_PACKAGE => package_str = artifact_str,
                KIND_NETWORK | KIND_VOLUME => {
                    resource_processed = true;
                }
                _ => continue,
            }
        }
    }

    logd!(1, "apply: total elapsed = {:?}", total_start.elapsed());

    // If only resource artifacts (Network/Volume) were processed, return success
    if resource_processed && scenario_str.is_empty() && package_str.is_empty() {
        Ok("Resource artifacts processed successfully".to_string())
    } else if scenario_str.is_empty() {
        Err("There is not any scenario in yaml string".into())
    } else if package_str.is_empty() {
        Err("There is not any package in yaml string".into())
    } else {
        save_pod_yaml_from_package(&package_str).await?;
        Ok(scenario_str)
    }
}

/// Delete downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(String)` - scenario yaml in downloaded artifact
/// ### Description
/// Delete scenario yaml only, because other scenario can use a package with same name
pub async fn withdraw(body: &str) -> common::Result<String> {
    let docs: Vec<&str> = body.split(YAML_SEPARATOR).collect();

    for doc in docs {
        let value: serde_yaml::Value = serde_yaml::from_str(doc)?;

        if let Some((kind, name)) = parse_artifact_info(&value) {
            if kind == KIND_SCENARIO {
                let artifact_str = serde_yaml::to_string(&value)?;
                let key = format!("{}/{}", KIND_SCENARIO, name);
                data::delete_at_etcd(&key).await?;
                return Ok(artifact_str);
            }
        }
    }

    Err("There is not any scenario in yaml string".into())
}

/// Load model with optional volume and network resources
async fn load_model_with_resources(
    model_info: &common::spec::artifact::package::ModelInfo,
) -> common::Result<Model> {
    let model_str = common::etcd::get(&format!("{}/{}", KIND_MODEL, model_info.get_name())).await?;
    let mut model: Model = serde_yaml::from_str(&model_str)?;

    // Load volume if specified
    if let Some(volume_name) = model_info.get_resources().get_volume() {
        let volume_str = common::etcd::get(&format!("{}/{}", KIND_VOLUME, volume_name)).await?;
        let _volume: Volume = serde_yaml::from_str(&volume_str)?;
        // TODO: Apply volume configuration with new VolumeSpec structure
    }

    // Load network if specified
    if let Some(network_name) = model_info.get_resources().get_network() {
        let network_str = common::etcd::get(&format!("{}/{}", KIND_NETWORK, network_name)).await?;
        let _network: Network = serde_yaml::from_str(&network_str)?;
        // TODO: Apply network configuration
    }

    Ok(model)
}

/// Save Pod YAML for all models in a package
async fn save_pod_yaml_from_package(package_str: &str) -> common::Result<()> {
    let package: Package = serde_yaml::from_str(package_str)?;
    let mut models = Vec::new();

    for model_info in package.get_models() {
        let model = load_model_with_resources(&model_info).await?;
        models.push(model);
    }

    let pods: Vec<Pod> = models.into_iter().map(Pod::from).collect();

    for pod in pods {
        let pod_yaml = serde_yaml::to_string(&pod)?;
        let key = format!("{}/{}", "Pod", pod.get_name());
        data::write_to_etcd(&key, &pod_yaml).await?;
    }

    Ok(())
}

//UNIT TEST CASES

#[cfg(test)]
mod tests {
    use super::*;

    // -- Test Artifacts --

    /// Valid artifact YAML (Scenario + Package + Model)
    const VALID_ARTIFACT_YAML: &str = r#"
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
---
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

    /// Invalid YAML — missing `action` in Scenario
    const INVALID_YAML_MISSING_ACTION: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
  target: helloworld
---
apiVersion: v1
kind: Package
metadata:
  name: helloworld
spec:
  pattern:
    - type: plain
  models:
    - name: helloworld-core
      node: HPC
      resources:
        volume: []
        network: []
"#;

    /// Invalid YAML — only unknown artifact
    const INVALID_YAML_UNKNOWN_ARTIFACT: &str = r#"
apiVersion: v1
kind: Unknown
metadata:
  name: helloworld
spec:
  dummy: value
"#;

    /// Invalid YAML — empty string
    const INVALID_YAML_EMPTY: &str = "";

    /// Valid Model YAML for helloworld-core (required by Package)
    const VALID_MODEL_YAML: &str = r#"
apiVersion: v1
kind: Model
metadata:
  name: helloworld-core
  annotations:
    io.piccolo.annotations.package-type: helloworld-core
    io.piccolo.annotations.package-name: helloworld
    io.piccolo.annotations.package-network: default
  labels:
    app: helloworld-core
spec:
  hostNetwork: true
  containers:
    - name: helloworld
      image: helloworld:latest
  terminationGracePeriodSeconds: 0
"#;

    // -- apply() tests --

    /// Test apply() with valid artifact YAML (Scenario + Package present)
    #[tokio::test]
    async fn test_apply_valid_artifact() {
        // First, create the required Model that the Package references
        let model_value: serde_yaml::Value = serde_yaml::from_str(VALID_MODEL_YAML).unwrap();
        let model_str = serde_yaml::to_string(&model_value).unwrap();
        data::write_to_etcd("Model/helloworld-core", &model_str)
            .await
            .unwrap();

        let result = apply(VALID_ARTIFACT_YAML).await;

        // Assert: should succeed because both Scenario + Package present and valid
        assert!(
            result.is_ok(),
            "apply() failed with valid artifact: {:?}",
            result.err()
        );

        // Assert: scenario and package strings should not be empty
        let scenario = result.unwrap();
        assert!(!scenario.is_empty(), "Scenario YAML should not be empty");

        // Cleanup: Remove the created Model
        let _ = data::delete_at_etcd("Model/helloworld-core").await;
    }

    /// Test apply() with missing `action` field (invalid Scenario)
    #[tokio::test]
    async fn test_apply_invalid_missing_action() {
        let result = apply(INVALID_YAML_MISSING_ACTION).await;

        // Assert: should fail because Scenario is invalid (missing required field)
        assert!(
            result.is_err(),
            "apply() unexpectedly succeeded with missing action"
        );
    }

    /// Test apply() with unknown artifact (no Scenario, no Package)
    #[tokio::test]
    async fn test_apply_invalid_unknown_artifact() {
        let result = apply(INVALID_YAML_UNKNOWN_ARTIFACT).await;

        // Assert: should fail because no Scenario or Package present
        assert!(
            result.is_err(),
            "apply() unexpectedly succeeded with unknown artifact only"
        );
    }

    /// Test apply() with empty YAML
    #[tokio::test]
    async fn test_apply_invalid_empty_yaml() {
        let result = apply(INVALID_YAML_EMPTY).await;

        // Assert: should fail because YAML is empty
        assert!(
            result.is_err(),
            "apply() unexpectedly succeeded with empty YAML"
        );
    }

    // -- withdraw() tests --

    /// Test withdraw() with valid artifact YAML (Scenario present)
    #[tokio::test]
    async fn test_withdraw_valid_artifact() {
        let result = withdraw(VALID_ARTIFACT_YAML).await;

        // Assert: should succeed because Scenario is present
        assert!(
            result.is_ok(),
            "withdraw() failed with valid artifact: {:?}",
            result.err()
        );

        // Assert: returned scenario YAML should not be empty
        let scenario = result.unwrap();
        assert!(
            !scenario.is_empty(),
            "Returned scenario YAML should not be empty"
        );
    }

    /// Test withdraw() with unknown artifact (no Scenario)
    #[tokio::test]
    async fn test_withdraw_invalid_unknown_artifact() {
        let result = withdraw(INVALID_YAML_UNKNOWN_ARTIFACT).await;

        // Assert: should fail because no Scenario present
        assert!(
            result.is_err(),
            "withdraw() unexpectedly succeeded with unknown artifact"
        );
    }

    /// Test withdraw() with empty YAML
    #[tokio::test]
    async fn test_withdraw_invalid_empty_yaml() {
        let result = withdraw(INVALID_YAML_EMPTY).await;

        // Assert: should fail because YAML is empty
        assert!(
            result.is_err(),
            "withdraw() unexpectedly succeeded with empty YAML"
        );
    }
}

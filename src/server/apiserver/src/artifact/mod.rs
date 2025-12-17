/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Convert string-type artifacts to struct and access etcd

pub mod data;

use common::spec::artifact::Artifact;
use common::spec::artifact::Model;
use common::spec::artifact::Network;
use common::spec::artifact::Node;
use common::spec::artifact::Package;
use common::spec::artifact::Scenario;
use common::spec::artifact::Volume;

/// Apply downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(Vec<String>)` - vector of scenario yaml strings in downloaded artifact
/// ### Description
/// Write artifact in etcd
/// Note: Returns ALL scenarios from the YAML, not just the last one
pub async fn apply(body: &str) -> common::Result<Vec<String>> {
    use std::time::Instant;
    let total_start = Instant::now();

    let docs: Vec<&str> = body.split("---").collect();
    let mut scenario_strs: Vec<String> = Vec::new();
    let mut has_package = false;

    for doc in docs {
        // Skip empty documents
        if doc.trim().is_empty() {
            continue;
        }

        let parse_start = Instant::now();
        let value: serde_yaml::Value = match serde_yaml::from_str(doc) {
            Ok(v) => v,
            Err(e) => {
                println!("apply: failed to parse YAML document: {:?}", e);
                continue;
            }
        };
        let artifact_str = serde_yaml::to_string(&value)?;
        let parse_elapsed = parse_start.elapsed();
        println!("apply: YAML parse elapsed = {:?}", parse_elapsed);

        if let Some(kind) = value.clone().get("kind").and_then(|k| k.as_str()) {
            let name: String = match kind {
                "Scenario" => serde_yaml::from_value::<Scenario>(value)?.get_name(),
                "Package" => serde_yaml::from_value::<Package>(value)?.get_name(),
                "Volume" => serde_yaml::from_value::<Volume>(value)?.get_name(),
                "Network" => serde_yaml::from_value::<Network>(value)?.get_name(),
                "Node" => serde_yaml::from_value::<Node>(value)?.get_name(),
                "Model" => serde_yaml::from_value::<Model>(value)?.get_name(),
                _ => {
                    println!("unknown artifact");
                    continue;
                }
            };
            let key = format!("{}/{}", kind, name);

            let etcd_start = Instant::now();
            data::write_to_etcd(&key, &artifact_str).await?;
            let etcd_elapsed = etcd_start.elapsed();
            println!("apply: etcd write elapsed for {} = {:?}", key, etcd_elapsed);

            match kind {
                "Scenario" => {
                    // Collect ALL scenarios, not just the last one
                    scenario_strs.push(artifact_str);

                    // Set initial scenario state to idle via StateManager
                    println!("🔄 SCENARIO STATE INITIALIZATION: ApiServer Setting Initial State");
                    println!("   📋 Scenario: {}", name);
                    println!("   🔄 Initial State: → idle");
                    println!("   🔍 Reason: New scenario artifact received and stored in etcd");

                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_nanos() as i64;

                    let state_change = common::statemanager::StateChange {
                        resource_type: common::statemanager::ResourceType::Scenario as i32,
                        resource_name: name.clone(),
                        current_state: "".to_string(), // No previous state for new scenario
                        target_state: "idle".to_string(),
                        transition_id: format!("apiserver-scenario-init-{}", timestamp),
                        timestamp_ns: timestamp,
                        source: "apiserver".to_string(),
                    };

                    println!("   📤 Sending StateChange to StateManager:");
                    println!("      • Resource Type: SCENARIO");
                    println!("      • Resource Name: {}", state_change.resource_name);
                    println!("      • Target State: {}", state_change.target_state);
                    println!("      • Transition ID: {}", state_change.transition_id);
                    println!("      • Source: {}", state_change.source);

                    let mut state_sender =
                        crate::grpc::sender::statemanager::StateManagerSender::new();
                    if let Err(e) = state_sender.send_state_change(state_change).await {
                        println!("   ❌ Failed to send state change to StateManager: {:?}", e);
                    } else {
                        println!("   ✅ Successfully set scenario {} to idle state", name);
                    }
                }
                "Package" => has_package = true,
                _ => continue,
            };
        }
    }

    let total_elapsed = total_start.elapsed();
    println!("apply: total elapsed = {:?}", total_elapsed);
    println!("apply: found {} scenarios", scenario_strs.len());

    if scenario_strs.is_empty() {
        Err("There is not any scenario in yaml string".into())
    } else if !has_package {
        Err("There is not any package in yaml string".into())
    } else {
        Ok(scenario_strs)
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
    let docs: Vec<&str> = body.split("---").collect();
    for doc in docs {
        let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
        let artifact_str = serde_yaml::to_string(&value)?;

        if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
            match kind {
                "Scenario" => {
                    let name = serde_yaml::from_value::<Scenario>(value)?.get_name();
                    let key = format!("Scenario/{}", name);
                    data::delete_at_etcd(&key).await?;
                    return Ok(artifact_str);
                }
                _ => {
                    println!("unused artifact");
                }
            }
        }
    }

    Err("There is not any scenario in yaml string".into())
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

    // -- apply() tests --

    /// Test apply() with valid artifact YAML (Scenario + Package present)
    #[tokio::test]
    async fn test_apply_valid_artifact() {
        let result = apply(VALID_ARTIFACT_YAML).await;

        // Assert: should succeed because both Scenario + Package present and valid
        assert!(
            result.is_ok(),
            "apply() failed with valid artifact: {:?}",
            result.err()
        );

        // Assert: scenarios vector should not be empty
        let scenarios = result.unwrap();
        assert!(!scenarios.is_empty(), "Scenarios vector should not be empty");
        assert!(
            !scenarios[0].is_empty(),
            "First scenario YAML should not be empty"
        );
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

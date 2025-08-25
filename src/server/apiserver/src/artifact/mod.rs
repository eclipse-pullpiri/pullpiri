/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Convert string-type artifacts to struct and access etcd

pub mod data;

use common::spec::artifact::{model::ModelState, package::PackageState, scenario::ScenarioState};
use common::spec::artifact::{Artifact, Model, Network, Node, Package, Scenario, Volume};
use std::collections::HashMap;

/// Apply downloaded artifact to etcd
///
/// ### Parametets
/// * `body: &str` - whole yaml string of piccolo artifact
/// ### Returns
/// * `Result(String, String)` - scenario and package yaml in downloaded artifact
/// ### Description
/// Write artifact in etcd
pub async fn apply(body: &str) -> common::Result<String> {
    let docs: Vec<&str> = body.split("---").collect();
    let mut results = HashMap::new();

    for doc in docs {
        if let Err(e) = process_document(doc, &mut results).await {
            println!("Error processing document: {}", e);
            continue;
        }
    }

    // Validate results
    let scenario_str = results.get("Scenario").cloned().unwrap_or_default();
    let package_str = results.get("Package").cloned().unwrap_or_default();

    if scenario_str.is_empty() {
        Err("There is not any scenario in yaml string".into())
    } else if package_str.is_empty() {
        Err("There is not any package in yaml string".into())
    } else {
        Ok(scenario_str)
    }
}

async fn process_document(doc: &str, results: &mut HashMap<String, String>) -> common::Result<()> {
    let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
    let artifact_str = serde_yaml::to_string(&value)?;

    if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
        let name = get_name_by_kind(kind, value.clone())?;
        let updated_artifact = update_artifact_status(kind, artifact_str)?;

        let key = format!("{}/{}", kind, name);
        data::write_to_etcd(&key, &updated_artifact).await?;

        results.insert(kind.to_string(), updated_artifact);
    }

    Ok(())
}

fn get_name_by_kind(kind: &str, value: serde_yaml::Value) -> common::Result<String> {
    match kind {
        "Scenario" => Ok(serde_yaml::from_value::<Scenario>(value)?.get_name()),
        "Package" => Ok(serde_yaml::from_value::<Package>(value)?.get_name()),
        "Volume" => Ok(serde_yaml::from_value::<Volume>(value)?.get_name()),
        "Network" => Ok(serde_yaml::from_value::<Network>(value)?.get_name()),
        "Node" => Ok(serde_yaml::from_value::<Node>(value)?.get_name()),
        "Model" => Ok(serde_yaml::from_value::<Model>(value)?.get_name()),
        _ => Err("Unknown artifact kind".into()),
    }
}

fn update_artifact_status(kind: &str, artifact_str: String) -> common::Result<String> {
    match kind {
        "Scenario" => {
            let mut scenario: Scenario = serde_yaml::from_str(&artifact_str)?;
            scenario.set_status(ScenarioState::Idle);
            Ok(serde_yaml::to_string(&scenario)?)
        }
        "Package" => {
            let mut package: Package = serde_yaml::from_str(&artifact_str)?;
            package.set_status(PackageState::Initializing);
            Ok(serde_yaml::to_string(&package)?)
        }
        "Model" => {
            let mut model: Model = serde_yaml::from_str(&artifact_str)?;
            model.set_status(ModelState::Pending);
            Ok(serde_yaml::to_string(&model)?)
        }
        _ => Ok(artifact_str),
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
    use tokio;

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

        // Assert: scenario and package strings should not be empty
        let scenario = result.unwrap();
        assert!(!scenario.is_empty(), "Scenario YAML should not be empty");
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

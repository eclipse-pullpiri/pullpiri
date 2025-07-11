/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Create Model artifact from given Package information

use common::spec::artifact::{Artifact, Model, Network, Package, Scenario, Volume};

pub async fn yaml_split(body: &str) -> common::Result<(String, Vec<Model>)> {
    let docs: Vec<&str> = body.split("---").collect();
    let mut scenario_str = String::new();
    let mut package_str = String::new();
    let mut models: Vec<Model> = Vec::new();
    //let mut network_str = String::new();

    for doc in docs {
        let value: serde_yaml::Value = serde_yaml::from_str(doc)?;
        let artifact_str = serde_yaml::to_string(&value)?;

        if let Some(kind) = value.clone().get("kind").and_then(|k| k.as_str()) {
            let name: String = match kind {
                "Scenario" => serde_yaml::from_value::<Scenario>(value.clone())?.get_name(),
                "Package" => serde_yaml::from_value::<Package>(value.clone())?.get_name(),
                "Volume" => serde_yaml::from_value::<Volume>(value.clone())?.get_name(),
                "Network" => serde_yaml::from_value::<Network>(value.clone())?.get_name(),
                "Model" => serde_yaml::from_value::<Model>(value.clone())?.get_name(),
                _ => {
                    println!("unknown artifact");
                    continue;
                }
            };

            match kind {
                "Scenario" => scenario_str = artifact_str,
                "Package" => package_str = artifact_str,
                "Model" => {
                    let model = serde_yaml::from_value::<Model>(value)?;
                    models.push(model);
                }
                //"Network" => network_str = artifact_str,
                _ => continue,
            };
        }
    }

    if scenario_str.is_empty() {
        Err("There is not any scenario in yaml string".into())
    } else if package_str.is_empty() {
        //Missing Check is Added for Package
        Err("There is not any package in yaml string".into())
    } else {
        Ok((package_str, models)) //, network_str))
    }
}

/// Get combined `Network`, `Volume`, parsed `Model` information
///
/// ### Parametets
/// * `p: Package` - Package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information
pub async fn get_complete_model(
    p: Package,
    node: String,
    models: Vec<Model>,
) -> common::Result<Vec<Model>> {
    let mut base_models: Vec<Model> = Vec::new();
    let mut model_name: String = String::new();
    for mi in p.get_models() {
        if mi.get_node() == node {
            model_name = mi.get_name();
            for model in models.iter() {
                if model.get_name() == model_name {
                    base_models.push(model.clone());
                } else {
                    println!("Model {} is not for this node {}", model.get_name(), node);
                    continue;
                }
            }
        } else {
            println!("Model {} is not for this node {}", mi.get_name(), node);
            continue;
        }
        //let mut key = format!("Model/{}", mi.get_name());
        //let base_model_str = common::etcd::get(&key).await?;
        //let model: Model = serde_yaml::from_str(&base_model_str)?;

        // if let Some(volume_name) = mi.get_resources().get_volume() {
        //     key = format!("Volume/{}", volume_name);
        //     let volume_str: String = common::etcd::get(&key).await?;
        //     let volume: Volume = serde_yaml::from_str(&volume_str)?;

        //     if let Some(volume_spec) = volume.get_spec() {
        //         model
        //             .get_podspec()
        //             .volumes
        //             .clone_from(volume_spec.get_volume());
        //     }
        // }

        // if let Some(network_name) = mi.get_resources().get_network() {
        //     key = format!("Network/{}", network_name);
        //     let network_str = common::etcd::get(&key).await?;
        //     let network: Network = serde_yaml::from_str(&network_str)?;

        //     if let Some(network_spec) = network.get_spec() {
        //         // TODO
        //     }
        //}
    }
    Ok(base_models)
}

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::Deserialize;
    use serde_yaml::Deserializer;
    /// Helper function to extract a `Package` from a multi-document YAML
    fn extract_package_from_multi_yaml(yaml: &str) -> Option<Package> {
        let deserializer = Deserializer::from_str(yaml);
        for doc in deserializer {
            let maybe_value: Result<serde_yaml::Value, _> = serde_yaml::Value::deserialize(doc);
            if let Ok(value) = maybe_value {
                if let Some(kind) = value.get("kind").and_then(|k| k.as_str()) {
                    if kind == "Package" {
                        let pkg: Result<Package, _> = serde_yaml::from_value(value);
                        return pkg.ok();
                    }
                }
            }
        }
        None
    }
    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: helloworld
spec:
  condition:
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
---
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
      image: helloworld
  terminationGracePeriodSeconds: 0
"#;

    #[tokio::test]
    async fn test_volume_and_network_resolution() {
        // Insert Volume YAML
        let volume_yaml = r#"
apiVersion: v1
kind: Volume
metadata:
  name: test-volume
spec:
  volume:
    - name: data
      emptyDir: {}
"#;
        common::etcd::put("Volume/test-volume", volume_yaml)
            .await
            .unwrap();

        // Insert Network YAML
        let network_yaml = r#"
apiVersion: v1
kind: Network
metadata:
  name: test-network
spec:
  interfaces:
    - name: eth0
      bridge: br0
"#;
        common::etcd::put("Network/test-network", network_yaml)
            .await
            .unwrap();

        // Create a valid Package referencing above Volume and Network
        let pkg_yaml = r#"
apiVersion: v1
kind: Package
metadata:
  name: test
spec:
  pattern:
    - type: plain
  models:
    - name: test-model
      node: node1
      resources:
        volume: test-volume
        network: test-network
"#;

        let model_yaml = r#"
apiVersion: v1
kind: Model
metadata:
  name: test-model
spec:
  containers:
    - name: app
      image: test
"#;

        common::etcd::put("Model/test-model", model_yaml)
            .await
            .unwrap();

        // Deserialize and test
        let package: Package = serde_yaml::from_str(pkg_yaml).unwrap();
        let result = get_complete_model(package).await;

        assert!(result.is_ok());
        let models = result.unwrap();
        assert_eq!(models.len(), 1);
    }

    // Test case for a valid scenario where get_complete_model works correctly
    #[tokio::test]
    async fn test_get_complete_model_success() {
        // Create a dummy package with valid data
        let package = extract_package_from_multi_yaml(VALID_ARTIFACT_YAML);

        // Call get_complete_model and check if it returns Ok
        let result = get_complete_model(package.expect("REASON")).await;

        // If result is an error, print the error for debugging
        assert!(
            result.is_ok(),
            "get_complete_model failed: {:?}",
            result.err()
        );
    }

    // Test case for invalid YAML, ensuring deserialization fails
    #[tokio::test]
    async fn test_get_complete_model_invalid_yaml() {
        // Simulating an invalid YAML format
        let invalid_yaml = "invalid: ::: yaml";

        // Try to parse the invalid YAML
        let result = serde_yaml::from_str::<Package>(invalid_yaml);
        assert!(result.is_err()); // Should fail to parse
    }

    // Test case for missing models field in the Package YAML
    #[tokio::test]
    async fn test_get_complete_model_missing_models() {
        // Define a Package YAML missing the "models" field
        let package_yaml_missing_models = r#"
        apiVersion: v1
        kind: Package
        metadata:
          label: null
          name: antipinch-enable
        spec:
          pattern:
            - type: plain
        "#;

        // Try to deserialize the package
        let package_missing_models: Result<Package, _> =
            serde_yaml::from_str(package_yaml_missing_models);
        assert!(package_missing_models.is_err()); // Should fail due to missing models
    }

    // Test case for missing volume in resources, should cause error in get_complete_model
    #[tokio::test]
    async fn test_get_complete_model_missing_volume() {
        // Define a Package YAML missing the "volume" resource
        let package_yaml_missing_volume = r#"
        apiVersion: v1
        kind: Package
        metadata:
          label: null
          name: antipinch-enable
        spec:
          pattern:
            - type: plain
          models:
            - name: antipinch-enable-core
              node: HPC
              resources:
                network: antipinch-network
        "#;

        let models_yaml = r#"
        apiVersion: v1
        kind: Model
        metadata:
          name: antipinch-enable-core
          annotations:
            io.piccolo.annotations.package-type: test
            io.piccolo.annotations.package-name: test
            io.piccolo.annotations.package-network: test
          labels:
            app: antipinch-enable-core
        spec:
          hostNetwork: true
          containers:
            - name: antipinch-enable-core
              image: localhost/antipinch:1.0
          terminationGracePeriodSeconds: 0
        "#;

        // Try to deserialize the package
        let package_missing_volume: Result<Package, _> =
            serde_yaml::from_str(package_yaml_missing_volume);
        assert!(package_missing_volume.is_ok()); // Package should still parse correctly

        let model: Result<Model, _> = serde_yaml::from_str(models_yaml);
        let m: Vec<Model> = vec![model.unwrap()];

        // Call get_complete_model and check if it returns an error due to missing volume
        let package = package_missing_volume.unwrap();
        let result = get_complete_model(package, "HPC".to_string(), m).await;
        assert!(result.is_err()); // Should fail due to missing volume
    }

    // Test case for missing network in resources, should cause error in get_complete_model
    #[tokio::test]
    async fn test_get_complete_model_missing_network() {
        // Define a Package YAML missing the "network" resource
        let package_yaml_missing_network = r#"
        apiVersion: v1
        kind: Package
        metadata:
          label: null
          name: antipinch-enable
        spec:
          pattern:
            - type: plain
          models:
            - name: antipinch-enable-core
              node: HPC
              resources:
                volume: antipinch-volume
        "#;

        let models_yaml = r#"
        apiVersion: v1
        kind: Model
        metadata:
          name: antipinch-enable-core
          annotations:
            io.piccolo.annotations.package-type: test
            io.piccolo.annotations.package-name: test
            io.piccolo.annotations.package-network: test
          labels:
            app: antipinch-enable-core
        spec:
          hostNetwork: true
          containers:
            - name: antipinch-enable-core
              image: localhost/antipinch:1.0
          terminationGracePeriodSeconds: 0
        "#;

        // Try to deserialize the package
        let package_missing_network: Result<Package, _> =
            serde_yaml::from_str(package_yaml_missing_network);
        assert!(package_missing_network.is_ok()); // Package should still parse correctly

        let model: Result<Model, _> = serde_yaml::from_str(models_yaml);
        let m: Vec<Model> = vec![model.unwrap()];

        // Call get_complete_model and check if it returns an error due to missing network
        let package = package_missing_network.unwrap();
        let result = get_complete_model(package, "HPC".to_string(), m).await;
        assert!(result.is_err()); // Should fail due to missing network
    }
}

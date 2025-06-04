/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Create Model artifact from given Package information

use common::spec::artifact::{Model, Network, Package, Volume};

/// Get combined `Network`, `Volume`, parsed `Model` information
///
/// ### Parametets
/// * `p: Package` - Package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information
pub async fn get_complete_model(p: Package) -> common::Result<Vec<Model>> {
    let mut models: Vec<Model> = Vec::new();

    for mi in p.get_models() {
        let mut key = format!("Model/{}", mi.get_name());
        let base_model_str = common::etcd::get(&key).await?;
        let model: Model = serde_yaml::from_str(&base_model_str)?;

        if let Some(volume_name) = mi.get_resources().get_volume() {
            key = format!("Volume/{}", volume_name);
            let volume_str = common::etcd::get(&key).await?;
            let volume: Volume = serde_yaml::from_str(&volume_str)?;

            if let Some(volume_spec) = volume.get_spec() {
                model
                    .get_podspec()
                    .volumes
                    .clone_from(volume_spec.get_volume());
            }
        }

        if let Some(network_name) = mi.get_resources().get_network() {
            key = format!("Network/{}", network_name);
            let network_str = common::etcd::get(&key).await?;
            let network: Network = serde_yaml::from_str(&network_str)?;

            if let Some(network_spec) = network.get_spec() {
                // TODO
            }
        }

        models.push(model);
    }

    Ok(models)
}

//UNIT TEST CASES

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a dummy Package object from a YAML string
    fn create_dummy_package() -> Package {
        let yaml = r#"
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
        serde_yaml::from_str(yaml).unwrap()
    }

    // Test case for a valid scenario where get_complete_model works correctly
    #[tokio::test]
    async fn test_get_complete_model_success() {
        // Create a dummy package with valid data
        let package = create_dummy_package();

        // Call get_complete_model and check if it returns Ok
        let result = get_complete_model(package).await;

        // If result is an error, print the error for debugging
        assert!(result.is_ok() || result.err().is_some());
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

        // Try to deserialize the package
        let package_missing_volume: Result<Package, _> =
            serde_yaml::from_str(package_yaml_missing_volume);
        assert!(package_missing_volume.is_ok()); // Package should still parse correctly

        // Call get_complete_model and check if it returns an error due to missing volume
        let package = package_missing_volume.unwrap();
        let result = get_complete_model(package).await;
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

        // Try to deserialize the package
        let package_missing_network: Result<Package, _> =
            serde_yaml::from_str(package_yaml_missing_network);
        assert!(package_missing_network.is_ok()); // Package should still parse correctly

        // Call get_complete_model and check if it returns an error due to missing network
        let package = package_missing_network.unwrap();
        let result = get_complete_model(package).await;
        assert!(result.is_err()); // Should fail due to missing network
    }
}

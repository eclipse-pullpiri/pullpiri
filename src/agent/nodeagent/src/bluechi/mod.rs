/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Performs tasks required for Bluechi integration

mod filemaker;
mod parser;

use common::spec::{
    artifact::{Model, Package},
    k8s::Pod,
};

/// Parsing model artifacts and make files about bluechi
///
/// ### Parametets
/// * `package_str` - whole yaml string of package artifact
/// ### Description
/// Get base `Model` information from package spec  
/// Combine `Network`, `Volume`, parsed `Model` information  
/// Convert `Model` to `Pod`  
/// Make `.kube`, `.yaml` files for bluechi  
/// Copy files to the guest node running Bluechi
pub async fn parse(yaml_str: String, nodename: String) -> common::Result<()> {
    let (package_str, models_str) = parser::yaml_split(&yaml_str).await?;
    let package: Package = serde_yaml::from_str(&package_str)?;

    let models: Vec<Model> =
        parser::get_complete_model(package, nodename.clone(), models_str).await?;
    let pods: Vec<Pod> = models.into_iter().map(Pod::from).collect();

    filemaker::make_files_from_pod(pods, nodename).await?;

    // filemaker::delete_symlink_and_reload(&mi.get_name(), &model_node)
    // .await
    // .map_err(|e| {
    //     format!("Failed to delete symlink for '{}': {}", mi.get_name(), e)
    // })?;

    // make_symlink_and_reload(
    // &model_node,
    // &mi.get_name(),
    // &scenario.get_targets(),
    // )
    // .await
    // .map_err(|e| {
    // format!("Failed to create symlink for '{}': {}", mi.get_name(), e)
    // })?;

    //filemaker::copy_to_remote_node(file_names)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::parse;
    use common::Result;

    const VALID_ARTIFACT_YAML: &str = r#"
apiVersion: v1
kind: Scenario
metadata:
  name: hellow1
spec:
  condition:
  action: update
  target: hellow1
---
apiVersion: v1
kind: Package
metadata:
  label: null
  name: hellow1
spec:
  pattern:
    - type: plain
  models:
    - name: hellow1-core
      node: HPC
      resources:
        volume:
        network:
---
apiVersion: v1
kind: Model
metadata:
  name: hellow1-core
  annotations:
    io.piccolo.annotations.package-type: hellow1-core
    io.piccolo.annotations.package-name: hellow1
    io.piccolo.annotations.package-network: default
  labels:
    app: hellow1-core
spec:
  hostNetwork: true
  containers:
    - name: hellow1
      image: hellow1
  terminationGracePeriodSeconds: 0
"#;

    #[tokio::test]
    async fn test_parse_with_valid_artifact_yaml() -> Result<()> {
        let nodename = "HPC".to_string();

        let result = parse(VALID_ARTIFACT_YAML.to_string(), nodename).await;
        assert!(result.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_parse_with_empty_yaml() {
        let yaml_str = "".to_string();
        let nodename = "empty-node".to_string();

        let result = parse(yaml_str, nodename).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_with_invalid_yaml_format() {
        let yaml_str = "invalid_yaml: [::]".to_string();
        let nodename = "invalid-node".to_string();

        let result = parse(yaml_str, nodename).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_with_missing_models_section() {
        let yaml_str = r#"
---
apiVersion: v1
kind: Package
metadata:
  name: example-package
"#
        .to_string();
        let nodename = "missing-models-node".to_string();

        let result = parse(yaml_str, nodename).await;
        assert!(result.is_err());
    }
}

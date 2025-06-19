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
pub async fn parse(package_str: String) -> common::Result<()> {
    let package: Package = serde_yaml::from_str(&package_str)?;

    let models: Vec<Model> = parser::get_complete_model(package).await?;
    let pods: Vec<Pod> = models.into_iter().map(Pod::from).collect();

    let file_names = filemaker::make_files_from_pod(pods).await?;
    filemaker::copy_to_remote_node(file_names)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    // Valid YAML string for testing a Package artifact
    fn valid_package_yaml() -> String {
        r#"
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
                volume: vd-volume
                network: vd-network
                "#
        .to_string()
    }

    // Test case for parsing a valid package YAML
    #[tokio::test]
    async fn test_parse_success() {
        let yaml_str = valid_package_yaml();
        let result = parse(yaml_str).await;
        assert!(result.is_ok(), "parse() failed: {:?}", result.err());
    }

    // Test case for parsing an invalid package YAML (syntax error)
    #[tokio::test]
    async fn test_parse_invalid_yaml_syntax() {
        let invalid_yaml = "invalid: ::: yaml";
        let result = parse(invalid_yaml.to_string()).await;
        assert!(result.is_err(), "parse() unexpectedly succeeded");
    }

    // Test case for parsing a package YAML with missing fields (Missing model)
    #[tokio::test]
    async fn test_parse_missing_model_field() {
        let invalid_yaml = r#"
          apiVersion: v1
          kind: Package
          metadata:
            name: helloworld
          spec:
            pattern:
              - type: plain
        "#;
        let result = parse(invalid_yaml.to_string()).await;
        assert!(
            result.is_err(),
            "parse() unexpectedly succeeded with missing model field"
        );
    }

    // Test case for parsing a package YAML with invalid type in resources (e.g., invalid volume type)
    #[tokio::test]
    async fn test_parse_invalid_field_type_in_resources() {
        let invalid_yaml = r#"
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
                volume: 12345  # Invalid type (should be a string, not an integer)
                network: vd-network
        "#;
        let result = parse(invalid_yaml.to_string()).await;
        assert!(
            result.is_err(),
            "parse() unexpectedly succeeded with invalid field type in resources"
        );
    }

    // Test case for parsing an empty YAML string
    #[tokio::test]
    async fn test_parse_empty_yaml() {
        let empty_yaml = "".to_string();
        let result = parse(empty_yaml).await;
        assert!(
            result.is_err(),
            "parse() unexpectedly succeeded with empty YAML string"
        );
    }
}

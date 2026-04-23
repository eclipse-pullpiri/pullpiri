/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use crate::commands::{print_error, print_info, print_success};
use crate::{Result, SettingsClient};
use clap::Subcommand;
use colored::Colorize;
use std::fs;
use std::path::Path;

#[derive(Subcommand)]
pub enum YamlAction {
    /// Apply YAML artifact to the system
    Apply {
        /// Path to YAML file or '-' for stdin
        file: String,
    },
    /// Withdraw (delete) YAML artifact from the system
    Withdraw {
        /// Path to YAML file or '-' for stdin
        file: String,
    },
}

pub async fn handle(client: &SettingsClient, action: YamlAction) -> Result<()> {
    match action {
        YamlAction::Apply { file } => apply_yaml(client, &file).await,
        YamlAction::Withdraw { file } => withdraw_yaml(client, &file).await,
    }
}

/// Apply YAML artifact
async fn apply_yaml(client: &SettingsClient, file_path: &str) -> Result<()> {
    print_info(&format!("Applying YAML artifact from: {}", file_path));

    let yaml_content = read_yaml_content(file_path)?;

    // Validate that it's a multi-document YAML with required kinds
    validate_yaml_artifact(&yaml_content)?;

    match client.post_yaml("/api/artifact", &yaml_content).await {
        Ok(response) => {
            if let Some(message) = response.get("message") {
                println!("{}", message.as_str().unwrap_or("Applied successfully"));
            }

            if let Some(applied) = response.get("applied") {
                if let Some(array) = applied.as_array() {
                    println!("\nApplied resources:");
                    for (i, resource) in array.iter().enumerate() {
                        if let Some(kind) = resource.get("kind") {
                            if let Some(name) = resource.get("name") {
                                println!(
                                    "  {}. {} - {}",
                                    i + 1,
                                    kind.as_str().unwrap_or("Unknown"),
                                    name.as_str().unwrap_or("Unknown")
                                );
                            }
                        }
                    }
                }
            }

            print_success("YAML artifact applied successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to apply YAML artifact: {}", e));
            return Err(e.into());
        }
    }

    Ok(())
}

/// Withdraw YAML artifact
async fn withdraw_yaml(client: &SettingsClient, file_path: &str) -> Result<()> {
    print_info(&format!("Withdrawing YAML artifact from: {}", file_path));

    let yaml_content = read_yaml_content(file_path)?;

    // Validate that it's a multi-document YAML with required kinds
    validate_yaml_artifact(&yaml_content)?;

    match client.delete_yaml("/api/artifact", &yaml_content).await {
        Ok(response) => {
            if let Some(message) = response.get("message") {
                println!("{}", message.as_str().unwrap_or("Withdrawn successfully"));
            }

            if let Some(withdrawn) = response.get("withdrawn") {
                if let Some(array) = withdrawn.as_array() {
                    println!("\nWithdrawn resources:");
                    for (i, resource) in array.iter().enumerate() {
                        if let Some(kind) = resource.get("kind") {
                            if let Some(name) = resource.get("name") {
                                println!(
                                    "  {}. {} - {}",
                                    i + 1,
                                    kind.as_str().unwrap_or("Unknown"),
                                    name.as_str().unwrap_or("Unknown")
                                );
                            }
                        }
                    }
                }
            }

            print_success("YAML artifact withdrawn successfully");
        }
        Err(e) => {
            print_error(&format!("Failed to withdraw YAML artifact: {}", e));
            return Err(e.into());
        }
    }

    Ok(())
}

/// Read YAML content from file or stdin
fn read_yaml_content(file_path: &str) -> Result<String> {
    if file_path == "-" {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        Ok(buffer)
    } else {
        if !Path::new(file_path).exists() {
            return Err(crate::CliError::Custom(format!("File not found: {}", file_path)).into());
        }
        let content = fs::read_to_string(file_path)?;
        Ok(content)
    }
}

/// Validate YAML artifact structure
fn validate_yaml_artifact(yaml_content: &str) -> Result<()> {
    // Check if it contains required document separators
    if !yaml_content.contains("---") {
        print_info("Single document YAML detected - this may work for simple scenarios");
        return Ok(());
    }

    // Split documents and check for required kinds
    let documents: Vec<&str> = yaml_content.split("---").collect();
    let mut found_kinds = std::collections::HashSet::new();

    for doc in documents {
        let doc = doc.trim();
        if doc.is_empty() {
            continue;
        }

        // Look for 'kind:' line
        for line in doc.lines() {
            let line = line.trim();
            if line.starts_with("kind:") {
                if let Some(kind) = line.split(':').nth(1) {
                    found_kinds.insert(kind.trim().to_string());
                }
                break;
            }
        }
    }

    // Warn about missing kinds but don't fail
    let required_kinds = vec!["Scenario", "Package", "Model"];
    let missing_kinds: Vec<&str> = required_kinds
        .iter()
        .filter(|&&kind| !found_kinds.contains(kind))
        .copied()
        .collect();

    if !missing_kinds.is_empty() {
        println!(
            "{} Warning: Missing recommended kinds: {}",
            "⚠".yellow().bold(),
            missing_kinds.join(", ")
        );
        println!(
            "   The API Server expects Scenario, Package, and Model kinds for proper operation."
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_yaml_content_file_not_found() {
        let result = read_yaml_content("/nonexistent/path/file.yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_yaml_artifact_single_document() {
        let yaml = "apiVersion: v1\nkind: Scenario\nmetadata:\n  name: test";
        let result = validate_yaml_artifact(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_yaml_artifact_multi_document() {
        let yaml = r#"---
apiVersion: v1
kind: Scenario
metadata:
  name: test
---
apiVersion: v1
kind: Package
metadata:
  name: pkg
---
apiVersion: v1
kind: Model
metadata:
  name: model
"#;
        let result = validate_yaml_artifact(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_yaml_artifact_missing_kinds() {
        // This should succeed but print a warning (we can't easily test stdout)
        let yaml = r#"---
apiVersion: v1
kind: Scenario
metadata:
  name: test
"#;
        let result = validate_yaml_artifact(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_yaml_artifact_empty() {
        let result = validate_yaml_artifact("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_yaml_artifact_only_separators() {
        let yaml = "---\n---\n---";
        let result = validate_yaml_artifact(yaml);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_yaml_artifact_no_kind_field() {
        let yaml = r#"---
apiVersion: v1
metadata:
  name: test
"#;
        let result = validate_yaml_artifact(yaml);
        assert!(result.is_ok());
    }
}

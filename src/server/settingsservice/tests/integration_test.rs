// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for Settings Service

use serde_json::json;
use settingsservice::settings_utils::yaml;

#[tokio::test]
async fn test_yaml_utilities() {
    // Test YAML parsing
    let yaml_content = r#"
name: test-config
version: 1
nested:
  value: 42
  array:
    - item1
    - item2
"#;

    let parsed = yaml::parse_yaml(yaml_content).expect("Failed to parse YAML");
    
    // Test path retrieval
    assert_eq!(
        yaml::get_path(&parsed, "name").unwrap(),
        &json!("test-config")
    );
    assert_eq!(
        yaml::get_path(&parsed, "nested.value").unwrap(),
        &json!(42)
    );

    // Test YAML serialization
    let serialized = yaml::to_yaml_string(&parsed).expect("Failed to serialize YAML");
    assert!(serialized.contains("name: test-config"));
}

#[tokio::test]
async fn test_yaml_merge() {
    let mut base = json!({
        "name": "base",
        "config": {
            "value": 1,
            "unchanged": "keep"
        }
    });

    let overlay = json!({
        "name": "overlay",
        "config": {
            "value": 2,
            "new": "added"
        }
    });

    yaml::merge_yaml(&mut base, &overlay).expect("Failed to merge YAML");

    assert_eq!(base["name"], json!("overlay"));
    assert_eq!(base["config"]["value"], json!(2));
    assert_eq!(base["config"]["unchanged"], json!("keep"));
    assert_eq!(base["config"]["new"], json!("added"));
}

#[tokio::test] 
async fn test_yaml_path_operations() {
    let mut value = json!({
        "level1": {
            "level2": {
                "value": "original"
            }
        }
    });

    // Test path setting
    yaml::set_path(&mut value, "level1.level2.value", json!("modified"))
        .expect("Failed to set path");
    
    assert_eq!(
        yaml::get_path(&value, "level1.level2.value").unwrap(),
        &json!("modified")
    );

    // Test creating new path
    yaml::set_path(&mut value, "level1.new_key", json!("new_value"))
        .expect("Failed to set new path");
    
    assert_eq!(
        yaml::get_path(&value, "level1.new_key").unwrap(),
        &json!("new_value")
    );
}

#[test]
fn test_error_types() {
    use settingsservice::settings_utils::error::{SettingsError, StorageError};

    // Test error creation and formatting
    let config_error = SettingsError::Config("test error".to_string());
    assert_eq!(format!("{}", config_error), "Configuration error: test error");

    let storage_error = StorageError::ConnectionFailed("connection failed".to_string());
    let settings_error = SettingsError::Storage(storage_error);
    assert_eq!(format!("{}", settings_error), "Storage error: ETCD connection failed: connection failed");
}

#[test]
fn test_logging_initialization() {
    use settingsservice::settings_utils::logging;

    // Test that logging can be initialized without errors
    let result = logging::init_logging("debug");
    assert!(result.is_ok(), "Logging initialization should succeed");
}
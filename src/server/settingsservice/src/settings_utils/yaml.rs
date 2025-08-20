// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! YAML processing utilities

use anyhow::Result;
use serde_json::Value;
use crate::settings_utils::error::SettingsError;

/// Parse YAML content into a JSON Value
pub fn parse_yaml(content: &str) -> Result<Value, SettingsError> {
    serde_yaml::from_str(content)
        .map_err(|e| SettingsError::Config(format!("YAML parse error: {}", e)))
}

/// Serialize a JSON Value to YAML string
pub fn to_yaml_string(value: &Value) -> Result<String, SettingsError> {
    serde_yaml::to_string(value)
        .map_err(|e| SettingsError::Config(format!("YAML serialization error: {}", e)))
}

/// Merge two YAML values, with overlay taking precedence
pub fn merge_yaml(base: &mut Value, overlay: &Value) -> Result<(), SettingsError> {
    match (base.clone(), overlay) {
        (Value::Object(mut base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(base_value) = base_map.get_mut(key) {
                    merge_yaml(base_value, value)?;
                } else {
                    base_map.insert(key.clone(), value.clone());
                }
            }
            *base = Value::Object(base_map);
        }
        _ => {
            *base = overlay.clone();
        }
    }
    Ok(())
}

/// Get a value at the specified path
pub fn get_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;
    
    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            _ => return None,
        }
    }
    
    Some(current)
}

/// Set a value at the specified path
pub fn set_path(value: &mut Value, path: &str, new_value: Value) -> Result<(), SettingsError> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err(SettingsError::Config("Empty path".to_string()));
    }

    let mut current = value;
    
    // Navigate to the parent
    for part in &parts[..parts.len() - 1] {
        match current {
            Value::Object(map) => {
                current = map.entry(part.to_string()).or_insert(Value::Object(Default::default()));
            }
            _ => {
                return Err(SettingsError::Config(format!(
                    "Cannot navigate path '{}': not an object",
                    path
                )));
            }
        }
    }

    // Set the final value
    match current {
        Value::Object(map) => {
            map.insert(parts[parts.len() - 1].to_string(), new_value);
            Ok(())
        }
        _ => Err(SettingsError::Config(format!(
            "Cannot set path '{}': parent is not an object",
            path
        ))),
    }
}
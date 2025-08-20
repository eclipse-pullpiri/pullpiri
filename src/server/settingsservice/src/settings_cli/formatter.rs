// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Output formatting utilities for CLI

use serde_json::Value;

/// Output format options
#[derive(Debug, Clone)]
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
    Text,
}

/// Format JSON value as pretty JSON string
pub fn format_json(value: &Value, pretty: bool) -> String {
    if pretty {
        serde_json::to_string_pretty(value).unwrap_or_default()
    } else {
        serde_json::to_string(value).unwrap_or_default()
    }
}

/// Format JSON value as YAML string
pub fn format_yaml(value: &Value) -> String {
    serde_yaml::to_string(value).unwrap_or_default()
}

/// Format data as a simple table (placeholder implementation)
pub fn format_table<T: serde::Serialize>(items: &[T], _headers: &[&str]) -> String {
    // Simple implementation - convert to JSON for now
    if let Ok(json_value) = serde_json::to_value(items) {
        format_json(&json_value, true)
    } else {
        "Error formatting table".to_string()
    }
}
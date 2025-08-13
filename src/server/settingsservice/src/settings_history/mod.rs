// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration history management module

use crate::settings_config::{Config, ConfigManager};
use crate::settings_storage::{Storage, history_key};
use crate::settings_utils::error::SettingsError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// History entry for configuration changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub config_path: String,
    pub version: u64,
    pub timestamp: DateTime<Utc>,
    pub author: String,
    pub comment: Option<String>,
    pub action: ChangeAction,
    pub change_summary: String,
}

/// Type of change action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeAction {
    Create,
    Update,
    Delete,
    Rollback,
}

/// Difference between two configuration values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    pub path: String,
    pub operation: DiffOperation,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
}

/// Type of diff operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffOperation {
    Add,
    Remove,
    Change,
}

/// History manager for tracking configuration changes
pub struct HistoryManager {
    storage: Box<dyn Storage>,
}

impl HistoryManager {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self { storage }
    }

    /// Record a configuration change
    pub async fn record_change(
        &mut self,
        config_path: &str,
        old_config: Option<&Config>,
        new_config: &Config,
        action: ChangeAction,
    ) -> Result<u64, SettingsError> {
        let version = new_config.metadata.version;
        
        debug!("Recording change for {} version {}", config_path, version);

        // Calculate change summary
        let change_summary = if let Some(old) = old_config {
            Self::calculate_change_summary(&old.content, &new_config.content)
        } else {
            "Initial creation".to_string()
        };

        let history_entry = HistoryEntry {
            config_path: config_path.to_string(),
            version,
            timestamp: new_config.metadata.modified_at,
            author: new_config.metadata.author.clone(),
            comment: new_config.metadata.comment.clone(),
            action,
            change_summary,
        };

        // Store the full config as well for rollback purposes
        let history_data = serde_json::json!({
            "entry": history_entry,
            "config": new_config
        });

        let key = history_key(config_path, version);
        self.storage.put_json(&key, &history_data).await?;

        info!("Recorded history entry for {} version {}", config_path, version);
        Ok(version)
    }

    /// Get history entries for a configuration
    pub async fn list_history(
        &mut self,
        config_path: &str,
        limit: Option<usize>,
    ) -> Result<Vec<HistoryEntry>, SettingsError> {
        debug!("Listing history for: {}", config_path);

        let prefix = format!(
            "{}{}/",
            crate::settings_storage::KeyPrefixes::HISTORY,
            config_path
        );

        let entries = self.storage.list(&prefix).await?;
        let mut history_entries = Vec::new();

        for (key, value) in entries {
            match serde_json::from_str::<serde_json::Value>(&value) {
                Ok(data) => {
                    if let Some(entry_data) = data.get("entry") {
                        match serde_json::from_value::<HistoryEntry>(entry_data.clone()) {
                            Ok(entry) => history_entries.push(entry),
                            Err(e) => warn!("Failed to parse history entry from {}: {}", key, e),
                        }
                    }
                }
                Err(e) => warn!("Failed to parse history data from {}: {}", key, e),
            }
        }

        // Sort by version (descending)
        history_entries.sort_by(|a, b| b.version.cmp(&a.version));

        // Apply limit if specified
        if let Some(limit) = limit {
            history_entries.truncate(limit);
        }

        Ok(history_entries)
    }

    /// Get configuration at a specific version
    pub async fn get_version(
        &mut self,
        config_path: &str,
        version: u64,
    ) -> Result<Config, SettingsError> {
        debug!("Getting version {} of {}", version, config_path);

        let key = history_key(config_path, version);
        if let Some(history_data) = self.storage.get_json(&key).await? {
            if let Some(config_data) = history_data.get("config") {
                let config: Config = serde_json::from_value(config_data.clone())
                    .map_err(|e| SettingsError::History(format!("Failed to deserialize config: {}", e)))?;
                
                Ok(config)
            } else {
                Err(SettingsError::History(
                    "Config data not found in history entry".to_string(),
                ))
            }
        } else {
            Err(SettingsError::History(format!(
                "Version {} not found for config {}",
                version, config_path
            )))
        }
    }

    /// Rollback to a specific version
    pub async fn rollback_to_version(
        &mut self,
        config_path: &str,
        target_version: u64,
        config_manager: &mut ConfigManager,
        author: &str,
        comment: Option<String>,
    ) -> Result<Config, SettingsError> {
        info!("Rolling back {} to version {}", config_path, target_version);

        // Get the target version config
        let target_config = self.get_version(config_path, target_version).await?;

        // Get current config for history
        let current_config = config_manager.load_config(config_path).await?;

        // Create new config with rollback content
        let rollback_config = config_manager
            .update_config(
                config_path,
                target_config.content,
                author,
                comment.or_else(|| Some(format!("Rollback to version {}", target_version))),
                None, // Don't record history during rollback - we'll do it manually
            )
            .await?;

        // Record rollback in history
        self.record_change(
            config_path,
            Some(&current_config),
            &rollback_config,
            ChangeAction::Rollback,
        )
        .await?;

        Ok(rollback_config)
    }

    /// Calculate differences between two configurations
    pub fn calculate_diff(old_config: &Value, new_config: &Value) -> Vec<DiffEntry> {
        let mut diffs = Vec::new();
        Self::calculate_diff_recursive("", old_config, new_config, &mut diffs);
        diffs
    }

    /// Recursive function to calculate differences
    fn calculate_diff_recursive(
        path: &str,
        old: &Value,
        new: &Value,
        diffs: &mut Vec<DiffEntry>,
    ) {
        match (old, new) {
            (Value::Object(old_map), Value::Object(new_map)) => {
                // Check for removed and changed keys
                for (key, old_value) in old_map {
                    let current_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };

                    if let Some(new_value) = new_map.get(key) {
                        if old_value != new_value {
                            Self::calculate_diff_recursive(&current_path, old_value, new_value, diffs);
                        }
                    } else {
                        diffs.push(DiffEntry {
                            path: current_path,
                            operation: DiffOperation::Remove,
                            old_value: Some(old_value.clone()),
                            new_value: None,
                        });
                    }
                }

                // Check for added keys
                for (key, new_value) in new_map {
                    if !old_map.contains_key(key) {
                        let current_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{}.{}", path, key)
                        };

                        diffs.push(DiffEntry {
                            path: current_path,
                            operation: DiffOperation::Add,
                            old_value: None,
                            new_value: Some(new_value.clone()),
                        });
                    }
                }
            }
            _ => {
                if old != new {
                    diffs.push(DiffEntry {
                        path: path.to_string(),
                        operation: DiffOperation::Change,
                        old_value: Some(old.clone()),
                        new_value: Some(new.clone()),
                    });
                }
            }
        }
    }

    /// Format differences as a human-readable string
    pub fn format_diff(diffs: &[DiffEntry]) -> String {
        let mut output = Vec::new();

        for diff in diffs {
            match &diff.operation {
                DiffOperation::Add => {
                    output.push(format!(
                        "+ {}: {}",
                        diff.path,
                        diff.new_value.as_ref().unwrap_or(&Value::Null)
                    ));
                }
                DiffOperation::Remove => {
                    output.push(format!(
                        "- {}: {}",
                        diff.path,
                        diff.old_value.as_ref().unwrap_or(&Value::Null)
                    ));
                }
                DiffOperation::Change => {
                    output.push(format!(
                        "~ {}: {} -> {}",
                        diff.path,
                        diff.old_value.as_ref().unwrap_or(&Value::Null),
                        diff.new_value.as_ref().unwrap_or(&Value::Null)
                    ));
                }
            }
        }

        output.join("\n")
    }

    /// Calculate a brief change summary
    fn calculate_change_summary(old_config: &Value, new_config: &Value) -> String {
        let diffs = Self::calculate_diff(old_config, new_config);
        
        if diffs.is_empty() {
            return "No changes detected".to_string();
        }

        let adds = diffs.iter().filter(|d| matches!(d.operation, DiffOperation::Add)).count();
        let removes = diffs.iter().filter(|d| matches!(d.operation, DiffOperation::Remove)).count();
        let changes = diffs.iter().filter(|d| matches!(d.operation, DiffOperation::Change)).count();

        let mut summary_parts = Vec::new();
        
        if adds > 0 {
            summary_parts.push(format!("{} addition{}", adds, if adds == 1 { "" } else { "s" }));
        }
        if removes > 0 {
            summary_parts.push(format!("{} removal{}", removes, if removes == 1 { "" } else { "s" }));
        }
        if changes > 0 {
            summary_parts.push(format!("{} change{}", changes, if changes == 1 { "" } else { "s" }));
        }

        summary_parts.join(", ")
    }
}
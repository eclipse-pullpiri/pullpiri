// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration history management module

use crate::settings_config::{Config, ConfigManager};
use crate::settings_storage::{history_key, Storage};
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

        info!(
            "Recorded history entry for {} version {}",
            config_path, version
        );
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
                let config: Config = serde_json::from_value(config_data.clone()).map_err(|e| {
                    SettingsError::History(format!("Failed to deserialize config: {}", e))
                })?;

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
    fn calculate_diff_recursive(path: &str, old: &Value, new: &Value, diffs: &mut Vec<DiffEntry>) {
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
                            Self::calculate_diff_recursive(
                                &current_path,
                                old_value,
                                new_value,
                                diffs,
                            );
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

        let adds = diffs
            .iter()
            .filter(|d| matches!(d.operation, DiffOperation::Add))
            .count();
        let removes = diffs
            .iter()
            .filter(|d| matches!(d.operation, DiffOperation::Remove))
            .count();
        let changes = diffs
            .iter()
            .filter(|d| matches!(d.operation, DiffOperation::Change))
            .count();

        let mut summary_parts = Vec::new();

        if adds > 0 {
            summary_parts.push(format!(
                "{} addition{}",
                adds,
                if adds == 1 { "" } else { "s" }
            ));
        }
        if removes > 0 {
            summary_parts.push(format!(
                "{} removal{}",
                removes,
                if removes == 1 { "" } else { "s" }
            ));
        }
        if changes > 0 {
            summary_parts.push(format!(
                "{} change{}",
                changes,
                if changes == 1 { "" } else { "s" }
            ));
        }

        summary_parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_config::{Config, ConfigManager, ConfigMetadata};
    use crate::settings_storage::Storage;
    use crate::settings_utils::error::StorageError;
    use async_trait::async_trait;
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;
    use tokio;

    /// Mock storage implementation for testing
    #[derive(Default)]
    pub struct MockStorage {
        data: HashMap<String, String>,
        get_json_results: HashMap<String, Option<Value>>,
        list_results: HashMap<String, HashMap<String, String>>,
        should_fail: bool,
        fail_message: String,
    }

    impl MockStorage {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_failure(message: &str) -> Self {
            Self {
                data: HashMap::new(),
                get_json_results: HashMap::new(),
                list_results: HashMap::new(),
                should_fail: true,
                fail_message: message.to_string(),
            }
        }

        pub fn set_get_json_result(&mut self, key: String, result: Option<Value>) {
            self.get_json_results.insert(key, result);
        }

        pub fn set_list_result(&mut self, prefix: String, result: HashMap<String, String>) {
            self.list_results.insert(prefix, result);
        }
    }

    #[async_trait]
    impl Storage for MockStorage {
        async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            Ok(self.data.get(key).cloned())
        }

        async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            self.data.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            Ok(self.data.remove(key).is_some())
        }

        async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            if let Some(result) = self.list_results.get(prefix) {
                Ok(result.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            } else {
                let result = self
                    .data
                    .iter()
                    .filter(|(k, _)| k.starts_with(prefix))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Ok(result)
            }
        }

        async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            if let Some(result) = self.get_json_results.get(key) {
                Ok(result.clone())
            } else {
                match self.get(key).await? {
                    Some(value) => {
                        let json = serde_json::from_str(&value).map_err(|e| {
                            StorageError::SerializationError(format!("JSON parse error: {}", e))
                        })?;
                        Ok(Some(json))
                    }
                    None => Ok(None),
                }
            }
        }

        async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
            if self.should_fail {
                return Err(StorageError::OperationFailed(self.fail_message.clone()));
            }
            let json_str = serde_json::to_string(value).map_err(|e| {
                StorageError::SerializationError(format!("JSON serialize error: {}", e))
            })?;
            self.put(key, &json_str).await
        }
    }

    /// Create test configuration metadata
    fn create_test_metadata() -> ConfigMetadata {
        let now = Utc::now();
        ConfigMetadata {
            version: 1,
            created_at: now,
            modified_at: now,
            author: "test_user".to_string(),
            comment: Some("Test configuration".to_string()),
            schema_type: "user".to_string(),
        }
    }

    /// Create test configuration
    fn create_test_config() -> Config {
        Config {
            path: "/test/config".to_string(),
            content: json!({
                "name": "John Doe",
                "age": 30,
                "email": "john.doe@example.com"
            }),
            metadata: create_test_metadata(),
        }
    }

    /// Create test history entry
    fn create_test_history_entry() -> HistoryEntry {
        HistoryEntry {
            config_path: "/test/config".to_string(),
            version: 1,
            timestamp: Utc::now(),
            author: "test_user".to_string(),
            comment: Some("Test change".to_string()),
            action: ChangeAction::Update,
            change_summary: "1 change".to_string(),
        }
    }

    #[test]
    fn test_change_action_variants() {
        let create = ChangeAction::Create;
        let update = ChangeAction::Update;
        let delete = ChangeAction::Delete;
        let rollback = ChangeAction::Rollback;

        // Test pattern matching
        match create {
            ChangeAction::Create => (),
            _ => panic!("Expected Create variant"),
        }

        match update {
            ChangeAction::Update => (),
            _ => panic!("Expected Update variant"),
        }

        match delete {
            ChangeAction::Delete => (),
            _ => panic!("Expected Delete variant"),
        }

        match rollback {
            ChangeAction::Rollback => (),
            _ => panic!("Expected Rollback variant"),
        }
    }

    #[test]
    fn test_diff_operation_variants() {
        let add = DiffOperation::Add;
        let remove = DiffOperation::Remove;
        let change = DiffOperation::Change;

        // Test pattern matching
        match add {
            DiffOperation::Add => (),
            _ => panic!("Expected Add variant"),
        }

        match remove {
            DiffOperation::Remove => (),
            _ => panic!("Expected Remove variant"),
        }

        match change {
            DiffOperation::Change => (),
            _ => panic!("Expected Change variant"),
        }
    }

    #[test]
    fn test_history_entry_creation() {
        let entry = create_test_history_entry();

        assert_eq!(entry.config_path, "/test/config");
        assert_eq!(entry.version, 1);
        assert_eq!(entry.author, "test_user");
        assert_eq!(entry.comment, Some("Test change".to_string()));
        assert_eq!(entry.change_summary, "1 change");

        match entry.action {
            ChangeAction::Update => (),
            _ => panic!("Expected Update action"),
        }
    }

    #[test]
    fn test_diff_entry_creation() {
        let diff_entry = DiffEntry {
            path: "name".to_string(),
            operation: DiffOperation::Change,
            old_value: Some(json!("John")),
            new_value: Some(json!("Jane")),
        };

        assert_eq!(diff_entry.path, "name");
        assert_eq!(diff_entry.old_value, Some(json!("John")));
        assert_eq!(diff_entry.new_value, Some(json!("Jane")));

        match diff_entry.operation {
            DiffOperation::Change => (),
            _ => panic!("Expected Change operation"),
        }
    }

    #[test]
    fn test_calculate_diff_simple_change() {
        let old_config = json!({
            "name": "John",
            "age": 30
        });

        let new_config = json!({
            "name": "Jane",
            "age": 30
        });

        let diffs = HistoryManager::calculate_diff(&old_config, &new_config);
        assert_eq!(diffs.len(), 1);

        let diff = &diffs[0];
        assert_eq!(diff.path, "name");
        assert!(matches!(diff.operation, DiffOperation::Change));
        assert_eq!(diff.old_value, Some(json!("John")));
        assert_eq!(diff.new_value, Some(json!("Jane")));
    }

    #[test]
    fn test_calculate_diff_add_remove() {
        let old_config = json!({
            "name": "John",
            "age": 30,
            "city": "New York"
        });

        let new_config = json!({
            "name": "John",
            "age": 31,
            "email": "john@example.com"
        });

        let diffs = HistoryManager::calculate_diff(&old_config, &new_config);
        assert_eq!(diffs.len(), 3);

        // Find specific diffs
        let change_diff = diffs.iter().find(|d| d.path == "age").unwrap();
        assert!(matches!(change_diff.operation, DiffOperation::Change));
        assert_eq!(change_diff.old_value, Some(json!(30)));
        assert_eq!(change_diff.new_value, Some(json!(31)));

        let remove_diff = diffs.iter().find(|d| d.path == "city").unwrap();
        assert!(matches!(remove_diff.operation, DiffOperation::Remove));
        assert_eq!(remove_diff.old_value, Some(json!("New York")));
        assert_eq!(remove_diff.new_value, None);

        let add_diff = diffs.iter().find(|d| d.path == "email").unwrap();
        assert!(matches!(add_diff.operation, DiffOperation::Add));
        assert_eq!(add_diff.old_value, None);
        assert_eq!(add_diff.new_value, Some(json!("john@example.com")));
    }

    #[test]
    fn test_calculate_diff_nested_objects() {
        let old_config = json!({
            "user": {
                "name": "John",
                "profile": {
                    "age": 30
                }
            }
        });

        let new_config = json!({
            "user": {
                "name": "Jane",
                "profile": {
                    "age": 31
                }
            }
        });

        let diffs = HistoryManager::calculate_diff(&old_config, &new_config);
        assert_eq!(diffs.len(), 2);

        let name_diff = diffs.iter().find(|d| d.path == "user.name").unwrap();
        assert!(matches!(name_diff.operation, DiffOperation::Change));

        let age_diff = diffs.iter().find(|d| d.path == "user.profile.age").unwrap();
        assert!(matches!(age_diff.operation, DiffOperation::Change));
    }

    #[test]
    fn test_calculate_diff_no_changes() {
        let config = json!({
            "name": "John",
            "age": 30
        });

        let diffs = HistoryManager::calculate_diff(&config, &config);
        assert!(diffs.is_empty());
    }

    #[test]
    fn test_format_diff() {
        let diffs = vec![
            DiffEntry {
                path: "name".to_string(),
                operation: DiffOperation::Change,
                old_value: Some(json!("John")),
                new_value: Some(json!("Jane")),
            },
            DiffEntry {
                path: "email".to_string(),
                operation: DiffOperation::Add,
                old_value: None,
                new_value: Some(json!("jane@example.com")),
            },
            DiffEntry {
                path: "city".to_string(),
                operation: DiffOperation::Remove,
                old_value: Some(json!("New York")),
                new_value: None,
            },
        ];

        let formatted = HistoryManager::format_diff(&diffs);

        assert!(formatted.contains("~ name: \"John\" -> \"Jane\""));
        assert!(formatted.contains("+ email: \"jane@example.com\""));
        assert!(formatted.contains("- city: \"New York\""));
    }

    #[test]
    fn test_calculate_change_summary() {
        let old_config = json!({
            "name": "John",
            "age": 30,
            "city": "New York"
        });

        let new_config = json!({
            "name": "Jane",
            "age": 30,
            "email": "jane@example.com"
        });

        let summary = HistoryManager::calculate_change_summary(&old_config, &new_config);

        // Should contain information about additions, removals, and changes
        assert!(summary.contains("addition"));
        assert!(summary.contains("removal"));
        assert!(summary.contains("change"));
    }

    #[test]
    fn test_calculate_change_summary_no_changes() {
        let config = json!({
            "name": "John",
            "age": 30
        });

        let summary = HistoryManager::calculate_change_summary(&config, &config);
        assert_eq!(summary, "No changes detected");
    }

    #[test]
    fn test_calculate_change_summary_single_change() {
        let old_config = json!({
            "name": "John"
        });

        let new_config = json!({
            "name": "Jane"
        });

        let summary = HistoryManager::calculate_change_summary(&old_config, &new_config);
        assert_eq!(summary, "1 change");
    }

    #[test]
    fn test_calculate_change_summary_multiple_changes() {
        let old_config = json!({
            "name": "John",
            "age": 30
        });

        let new_config = json!({
            "name": "Jane",
            "age": 31
        });

        let summary = HistoryManager::calculate_change_summary(&old_config, &new_config);
        assert_eq!(summary, "2 changes");
    }

    #[tokio::test]
    async fn test_history_manager_new() {
        let storage = Box::new(MockStorage::new());
        let manager = HistoryManager::new(storage);

        // Basic verification that manager is created
        // Can't easily test internal state without more complex mocking
        // Manager should be successfully created without panicking
    }

    #[tokio::test]
    async fn test_record_change_create() {
        let mut storage = MockStorage::new();
        let mut manager = HistoryManager::new(Box::new(storage));

        let config = create_test_config();
        let result = manager
            .record_change("/test/config", None, &config, ChangeAction::Create)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_record_change_update() {
        let mut storage = MockStorage::new();
        let mut manager = HistoryManager::new(Box::new(storage));

        let old_config = create_test_config();
        let mut new_config = create_test_config();
        new_config.metadata.version = 2;
        new_config.content = json!({
            "name": "Jane Doe",
            "age": 31,
            "email": "jane.doe@example.com"
        });

        let result = manager
            .record_change(
                "/test/config",
                Some(&old_config),
                &new_config,
                ChangeAction::Update,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_record_change_with_storage_failure() {
        let storage = MockStorage::with_failure("Storage error");
        let mut manager = HistoryManager::new(Box::new(storage));

        let config = create_test_config();
        let result = manager
            .record_change("/test/config", None, &config, ChangeAction::Create)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_history_empty() {
        let mut storage = MockStorage::new();
        storage.set_list_result(
            "/piccolo/settings/history//test/config/".to_string(),
            HashMap::new(),
        );

        let mut manager = HistoryManager::new(Box::new(storage));

        let result = manager.list_history("/test/config", None).await;
        assert!(result.is_ok());

        let history = result.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_list_history_with_entries() {
        let mut storage = MockStorage::new();

        let entry1 = HistoryEntry {
            config_path: "/test/config".to_string(),
            version: 1,
            timestamp: Utc::now(),
            author: "user1".to_string(),
            comment: Some("First version".to_string()),
            action: ChangeAction::Create,
            change_summary: "Initial creation".to_string(),
        };

        let entry2 = HistoryEntry {
            config_path: "/test/config".to_string(),
            version: 2,
            timestamp: Utc::now(),
            author: "user2".to_string(),
            comment: Some("Second version".to_string()),
            action: ChangeAction::Update,
            change_summary: "1 change".to_string(),
        };

        let history_data1 = json!({
            "entry": entry1,
            "config": create_test_config()
        });

        let history_data2 = json!({
            "entry": entry2,
            "config": create_test_config()
        });

        let mut list_result = HashMap::new();
        list_result.insert(
            "key1".to_string(),
            serde_json::to_string(&history_data1).unwrap(),
        );
        list_result.insert(
            "key2".to_string(),
            serde_json::to_string(&history_data2).unwrap(),
        );

        storage.set_list_result(
            "/piccolo/settings/history//test/config/".to_string(),
            list_result,
        );

        let mut manager = HistoryManager::new(Box::new(storage));

        let result = manager.list_history("/test/config", None).await;
        assert!(result.is_ok());

        let history = result.unwrap();
        assert_eq!(history.len(), 2);

        // Should be sorted by version descending (version 2 first)
        assert_eq!(history[0].version, 2);
        assert_eq!(history[1].version, 1);
    }

    #[tokio::test]
    async fn test_list_history_with_limit() {
        let mut storage = MockStorage::new();

        // Create multiple entries
        let mut list_result = HashMap::new();
        for i in 1..=5 {
            let entry = HistoryEntry {
                config_path: "/test/config".to_string(),
                version: i,
                timestamp: Utc::now(),
                author: format!("user{}", i),
                comment: Some(format!("Version {}", i)),
                action: ChangeAction::Update,
                change_summary: "1 change".to_string(),
            };

            let history_data = json!({
                "entry": entry,
                "config": create_test_config()
            });

            list_result.insert(
                format!("key{}", i),
                serde_json::to_string(&history_data).unwrap(),
            );
        }

        storage.set_list_result(
            "/piccolo/settings/history//test/config/".to_string(),
            list_result,
        );

        let mut manager = HistoryManager::new(Box::new(storage));

        let result = manager.list_history("/test/config", Some(3)).await;
        assert!(result.is_ok());

        let history = result.unwrap();
        assert_eq!(history.len(), 3); // Limited to 3 entries
    }

    #[tokio::test]
    async fn test_get_version_success() {
        let mut storage = MockStorage::new();

        let config = create_test_config();
        let history_data = json!({
            "entry": create_test_history_entry(),
            "config": config
        });

        let key = history_key("/test/config", 1);
        storage.set_get_json_result(key, Some(history_data));

        let mut manager = HistoryManager::new(Box::new(storage));

        let result = manager.get_version("/test/config", 1).await;
        assert!(result.is_ok());

        let retrieved_config = result.unwrap();
        assert_eq!(retrieved_config.path, "/test/config");
        assert_eq!(retrieved_config.metadata.version, 1);
    }

    #[tokio::test]
    async fn test_get_version_not_found() {
        let mut storage = MockStorage::new();

        let key = history_key("/test/config", 999);
        storage.set_get_json_result(key, None);

        let mut manager = HistoryManager::new(Box::new(storage));

        let result = manager.get_version("/test/config", 999).await;
        assert!(result.is_err());

        if let Err(SettingsError::History(msg)) = result {
            assert!(msg.contains("Version 999 not found"));
        } else {
            panic!("Expected History error");
        }
    }

    #[tokio::test]
    async fn test_get_version_invalid_data() {
        let mut storage = MockStorage::new();

        // History data without config field
        let invalid_history_data = json!({
            "entry": create_test_history_entry()
            // Missing "config" field
        });

        let key = history_key("/test/config", 1);
        storage.set_get_json_result(key, Some(invalid_history_data));

        let mut manager = HistoryManager::new(Box::new(storage));

        let result = manager.get_version("/test/config", 1).await;
        assert!(result.is_err());

        if let Err(SettingsError::History(msg)) = result {
            assert!(msg.contains("Config data not found"));
        } else {
            panic!("Expected History error");
        }
    }

    #[test]
    fn test_history_entry_debug_and_clone() {
        let entry = create_test_history_entry();

        // Test Debug trait
        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("HistoryEntry"));
        assert!(debug_str.contains("/test/config"));

        // Test Clone trait
        let cloned_entry = entry.clone();
        assert_eq!(entry.config_path, cloned_entry.config_path);
        assert_eq!(entry.version, cloned_entry.version);
        assert_eq!(entry.author, cloned_entry.author);
        assert_eq!(entry.comment, cloned_entry.comment);
        assert_eq!(entry.change_summary, cloned_entry.change_summary);
    }

    #[test]
    fn test_diff_entry_debug_and_clone() {
        let diff = DiffEntry {
            path: "test.path".to_string(),
            operation: DiffOperation::Change,
            old_value: Some(json!("old")),
            new_value: Some(json!("new")),
        };

        // Test Debug trait
        let debug_str = format!("{:?}", diff);
        assert!(debug_str.contains("DiffEntry"));
        assert!(debug_str.contains("test.path"));

        // Test Clone trait
        let cloned_diff = diff.clone();
        assert_eq!(diff.path, cloned_diff.path);
        assert_eq!(diff.old_value, cloned_diff.old_value);
        assert_eq!(diff.new_value, cloned_diff.new_value);
    }

    #[test]
    fn test_change_action_serialization() {
        let actions = vec![
            ChangeAction::Create,
            ChangeAction::Update,
            ChangeAction::Delete,
            ChangeAction::Rollback,
        ];

        for action in actions {
            let serialized = serde_json::to_string(&action).unwrap();
            let deserialized: ChangeAction = serde_json::from_str(&serialized).unwrap();

            // Verify round-trip serialization works
            match (action, deserialized) {
                (ChangeAction::Create, ChangeAction::Create) => (),
                (ChangeAction::Update, ChangeAction::Update) => (),
                (ChangeAction::Delete, ChangeAction::Delete) => (),
                (ChangeAction::Rollback, ChangeAction::Rollback) => (),
                _ => panic!("Serialization round-trip failed"),
            }
        }
    }

    #[test]
    fn test_diff_operation_serialization() {
        let operations = vec![
            DiffOperation::Add,
            DiffOperation::Remove,
            DiffOperation::Change,
        ];

        for operation in operations {
            let serialized = serde_json::to_string(&operation).unwrap();
            let deserialized: DiffOperation = serde_json::from_str(&serialized).unwrap();

            // Verify round-trip serialization works
            match (operation, deserialized) {
                (DiffOperation::Add, DiffOperation::Add) => (),
                (DiffOperation::Remove, DiffOperation::Remove) => (),
                (DiffOperation::Change, DiffOperation::Change) => (),
                _ => panic!("Serialization round-trip failed"),
            }
        }
    }
}

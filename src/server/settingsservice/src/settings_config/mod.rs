// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration management module

use crate::settings_history::{ChangeAction, HistoryManager};
use crate::settings_storage::{config_key, schema_key, Storage};
use crate::settings_utils::error::{SettingsError, ValidationError};
use crate::settings_utils::yaml;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonschema::{JSONSchema, ValidationError as JsonSchemaError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub author: String,
    pub comment: Option<String>,
    pub schema_type: String,
}

/// Configuration with content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub path: String,
    pub content: Value,
    pub metadata: ConfigMetadata,
}

/// Configuration summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSummary {
    pub path: String,
    pub schema_type: String,
    pub version: u64,
    pub modified_at: DateTime<Utc>,
    pub author: String,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationErrorDetail>,
    pub warnings: Vec<ValidationErrorDetail>,
}

/// Validation error detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorDetail {
    pub path: String,
    pub message: String,
    pub severity: ValidationSeverity,
}

/// Validation error severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Schema validator for configuration validation
pub struct SchemaValidator {
    schemas: HashMap<String, JSONSchema>,
}

impl SchemaValidator {
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Load a schema for a specific type
    pub fn load_schema(&mut self, schema_type: &str, schema: &Value) -> Result<(), SettingsError> {
        let compiled_schema = JSONSchema::compile(schema)
            .map_err(|e| SettingsError::Validation(format!("Schema compilation failed: {}", e)))?;

        self.schemas
            .insert(schema_type.to_string(), compiled_schema);
        debug!("Loaded schema for type: {}", schema_type);
        Ok(())
    }

    /// Validate configuration against schema
    pub fn validate(&self, schema_type: &str, data: &Value) -> ValidationResult {
        if let Some(schema) = self.schemas.get(schema_type) {
            let validation_result = schema.validate(data);

            match validation_result {
                Ok(_) => ValidationResult {
                    is_valid: true,
                    errors: Vec::new(),
                    warnings: Vec::new(),
                },
                Err(errors) => {
                    let error_details: Vec<ValidationErrorDetail> = errors
                        .map(|error| ValidationErrorDetail {
                            path: error.instance_path.to_string(),
                            message: error.to_string(),
                            severity: ValidationSeverity::Error,
                        })
                        .collect();

                    ValidationResult {
                        is_valid: false,
                        errors: error_details,
                        warnings: Vec::new(),
                    }
                }
            }
        } else {
            ValidationResult {
                is_valid: false,
                errors: vec![ValidationErrorDetail {
                    path: "".to_string(),
                    message: format!("Schema '{}' not found", schema_type),
                    severity: ValidationSeverity::Error,
                }],
                warnings: Vec::new(),
            }
        }
    }
}

/// Configuration manager
pub struct ConfigManager {
    storage: Box<dyn Storage>,
    validator: SchemaValidator,
}

impl ConfigManager {
    pub fn new(storage: Box<dyn Storage>) -> Self {
        Self {
            storage,
            validator: SchemaValidator::new(),
        }
    }

    /// Load configuration by path
    pub async fn load_config(&mut self, config_path: &str) -> Result<Config, SettingsError> {
        debug!("Loading config: {}", config_path);

        let key = config_key(config_path);
        if let Some(config_data) = self.storage.get_json(&key).await? {
            let config: Config = serde_json::from_value(config_data).map_err(|e| {
                SettingsError::Config(format!("Failed to deserialize config: {}", e))
            })?;

            Ok(config)
        } else {
            Err(SettingsError::Config(format!(
                "Configuration not found: {}",
                config_path
            )))
        }
    }

    /// Save configuration
    pub async fn save_config(&mut self, config: &Config) -> Result<(), SettingsError> {
        info!(
            "Saving config: {} (version {})",
            config.path, config.metadata.version
        );

        let key = config_key(&config.path);
        let config_value = serde_json::to_value(config)
            .map_err(|e| SettingsError::Config(format!("Failed to serialize config: {}", e)))?;

        self.storage.put_json(&key, &config_value).await?;
        Ok(())
    }

    /// Create new configuration
    pub async fn create_config(
        &mut self,
        path: &str,
        content: Value,
        schema_type: &str,
        author: &str,
        comment: Option<String>,
        history_manager: Option<&mut HistoryManager>,
    ) -> Result<Config, SettingsError> {
        // Check if config already exists
        let key = config_key(path);
        if self.storage.get(&key).await?.is_some() {
            return Err(SettingsError::Config(format!(
                "Configuration already exists: {}",
                path
            )));
        }

        let now = Utc::now();
        let config = Config {
            path: path.to_string(),
            content,
            metadata: ConfigMetadata {
                version: 1,
                created_at: now,
                modified_at: now,
                author: author.to_string(),
                comment,
                schema_type: schema_type.to_string(),
            },
        };

        self.save_config(&config).await?;

        // Record history
        if let Some(history_manager) = history_manager {
            history_manager
                .record_change(path, None, &config, ChangeAction::Create)
                .await?;
        }

        Ok(config)
    }

    /// Update configuration
    pub async fn update_config(
        &mut self,
        path: &str,
        content: Value,
        author: &str,
        comment: Option<String>,
        history_manager: Option<&mut HistoryManager>,
    ) -> Result<Config, SettingsError> {
        let old_config = self.load_config(path).await?;

        let mut config = old_config.clone();
        config.content = content;
        config.metadata.version += 1;
        config.metadata.modified_at = Utc::now();
        config.metadata.author = author.to_string();
        config.metadata.comment = comment;

        self.save_config(&config).await?;

        // Record history
        if let Some(history_manager) = history_manager {
            history_manager
                .record_change(path, Some(&old_config), &config, ChangeAction::Update)
                .await?;
        }

        Ok(config)
    }

    /// Delete configuration
    pub async fn delete_config(
        &mut self,
        config_path: &str,
        history_manager: Option<&mut HistoryManager>,
    ) -> Result<(), SettingsError> {
        info!("Deleting config: {}", config_path);

        // Load existing config before deletion for history
        let old_config = if let Ok(config) = self.load_config(config_path).await {
            Some(config)
        } else {
            None
        };

        let key = config_key(config_path);
        if !self.storage.delete(&key).await? {
            warn!("Configuration not found for deletion: {}", config_path);
        }

        // Record history if we had a config to delete
        if let (Some(old_config), Some(history_manager)) = (old_config, history_manager) {
            // Create a tombstone config for history
            let mut deleted_config = old_config.clone();
            deleted_config.metadata.modified_at = Utc::now();
            history_manager
                .record_change(
                    config_path,
                    Some(&old_config),
                    &deleted_config,
                    ChangeAction::Delete,
                )
                .await?;
        }

        Ok(())
    }

    /// List configurations with optional prefix filter
    pub async fn list_configs(
        &mut self,
        prefix: Option<&str>,
    ) -> Result<Vec<ConfigSummary>, SettingsError> {
        debug!("Listing configs with prefix: {:?}", prefix);

        let search_prefix = format!(
            "{}{}",
            crate::settings_storage::KeyPrefixes::CONFIG,
            prefix.unwrap_or("")
        );

        let configs = self.storage.list(&search_prefix).await?;
        let mut summaries = Vec::new();

        for (key, value) in configs {
            match serde_json::from_str::<Config>(&value) {
                Ok(config) => {
                    summaries.push(ConfigSummary {
                        path: config.path,
                        schema_type: config.metadata.schema_type,
                        version: config.metadata.version,
                        modified_at: config.metadata.modified_at,
                        author: config.metadata.author,
                    });
                }
                Err(e) => {
                    warn!("Failed to parse config from key {}: {}", key, e);
                }
            }
        }

        Ok(summaries)
    }

    /// Validate configuration against schema
    pub async fn validate_config(
        &mut self,
        config: &Config,
    ) -> Result<ValidationResult, SettingsError> {
        debug!(
            "Validating config: {} with schema: {}",
            config.path, config.metadata.schema_type
        );

        // Load schema if not already loaded
        if !self
            .validator
            .schemas
            .contains_key(&config.metadata.schema_type)
        {
            self.load_schema(&config.metadata.schema_type).await?;
        }

        Ok(self
            .validator
            .validate(&config.metadata.schema_type, &config.content))
    }

    /// Load schema from storage
    async fn load_schema(&mut self, schema_type: &str) -> Result<(), SettingsError> {
        let key = schema_key(schema_type);
        if let Some(schema_data) = self.storage.get_json(&key).await? {
            self.validator.load_schema(schema_type, &schema_data)?;
        } else {
            warn!("Schema not found: {}", schema_type);
            // Create a basic schema that accepts any valid JSON
            let basic_schema = serde_json::json!({
                "type": "object"
            });
            self.validator.load_schema(schema_type, &basic_schema)?;
        }
        Ok(())
    }

    /// Create or update a schema
    pub async fn save_schema(
        &mut self,
        schema_type: &str,
        schema: &Value,
    ) -> Result<(), SettingsError> {
        info!("Saving schema: {}", schema_type);

        let key = schema_key(schema_type);
        self.storage.put_json(&key, schema).await?;

        // Update the validator
        self.validator.load_schema(schema_type, schema)?;

        Ok(())
    }

    /// Get schema by type
    pub async fn get_schema(&mut self, schema_type: &str) -> Result<Value, SettingsError> {
        let key = schema_key(schema_type);
        if let Some(schema) = self.storage.get_json(&key).await? {
            Ok(schema)
        } else {
            Err(SettingsError::Config(format!(
                "Schema not found: {}",
                schema_type
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_history::HistoryManager;
    use crate::settings_utils::error::{SettingsError, StorageError};
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use tokio;

    /// Mock storage implementation for testing
    #[derive(Default)]
    pub struct MockStorage {
        data: HashMap<String, String>,
        get_results: HashMap<String, Option<String>>,
        get_json_results: HashMap<String, Option<Value>>,
        delete_results: HashMap<String, bool>,
        list_results: HashMap<String, HashMap<String, String>>,
    }

    impl MockStorage {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn set_get_result(&mut self, key: String, result: Option<String>) {
            self.get_results.insert(key, result);
        }

        pub fn set_get_json_result(&mut self, key: String, result: Option<Value>) {
            self.get_json_results.insert(key, result);
        }

        pub fn set_delete_result(&mut self, key: String, result: bool) {
            self.delete_results.insert(key, result);
        }

        pub fn set_list_result(&mut self, prefix: String, result: HashMap<String, String>) {
            self.list_results.insert(prefix, result);
        }
    }

    #[async_trait]
    impl Storage for MockStorage {
        async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
            if let Some(result) = self.get_results.get(key) {
                Ok(result.clone())
            } else {
                Ok(self.data.get(key).cloned())
            }
        }

        async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
            self.data.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
            if let Some(result) = self.delete_results.get(key) {
                Ok(*result)
            } else {
                Ok(self.data.remove(key).is_some())
            }
        }

        async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
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
            let json_str = serde_json::to_string(value).map_err(|e| {
                StorageError::SerializationError(format!("JSON serialize error: {}", e))
            })?;
            self.put(key, &json_str).await
        }
    }

    /// Create a test schema for validation
    fn create_test_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "minLength": 1
                },
                "age": {
                    "type": "integer",
                    "minimum": 0
                },
                "email": {
                    "type": "string",
                    "format": "email"
                }
            },
            "required": ["name", "age"],
            "additionalProperties": false
        })
    }

    /// Create a valid test configuration data
    fn create_valid_config_data() -> Value {
        json!({
            "name": "John Doe",
            "age": 30,
            "email": "john.doe@example.com"
        })
    }

    /// Create an invalid test configuration data
    fn create_invalid_config_data() -> Value {
        json!({
            "name": "",
            "age": -5,
            "email": "invalid-email"
        })
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
            content: create_valid_config_data(),
            metadata: create_test_metadata(),
        }
    }

    #[test]
    fn test_config_metadata_creation() {
        let metadata = create_test_metadata();

        assert_eq!(metadata.version, 1);
        assert_eq!(metadata.author, "test_user");
        assert_eq!(metadata.comment, Some("Test configuration".to_string()));
        assert_eq!(metadata.schema_type, "user");
    }

    #[test]
    fn test_config_creation() {
        let config = create_test_config();

        assert_eq!(config.path, "/test/config");
        assert_eq!(config.metadata.version, 1);
        assert_eq!(config.metadata.author, "test_user");
        assert_eq!(config.metadata.schema_type, "user");

        // Verify content structure
        let content = &config.content;
        assert_eq!(content["name"], "John Doe");
        assert_eq!(content["age"], 30);
        assert_eq!(content["email"], "john.doe@example.com");
    }

    #[test]
    fn test_config_summary_creation() {
        let config = create_test_config();
        let summary = ConfigSummary {
            path: config.path.clone(),
            schema_type: config.metadata.schema_type.clone(),
            version: config.metadata.version,
            modified_at: config.metadata.modified_at,
            author: config.metadata.author.clone(),
        };

        assert_eq!(summary.path, "/test/config");
        assert_eq!(summary.schema_type, "user");
        assert_eq!(summary.version, 1);
        assert_eq!(summary.author, "test_user");
    }

    #[test]
    fn test_validation_error_detail() {
        let error_detail = ValidationErrorDetail {
            path: "/name".to_string(),
            message: "String too short".to_string(),
            severity: ValidationSeverity::Error,
        };

        assert_eq!(error_detail.path, "/name");
        assert_eq!(error_detail.message, "String too short");
        matches!(error_detail.severity, ValidationSeverity::Error);
    }

    #[test]
    fn test_schema_validator_new() {
        let validator = SchemaValidator::new();
        assert!(validator.schemas.is_empty());
    }

    #[test]
    fn test_schema_validator_load_schema() {
        let mut validator = SchemaValidator::new();
        let schema = create_test_schema();

        let result = validator.load_schema("user", &schema);
        assert!(result.is_ok());
        assert!(validator.schemas.contains_key("user"));
    }

    #[test]
    fn test_schema_validator_load_invalid_schema() {
        let mut validator = SchemaValidator::new();
        let invalid_schema = json!({
            "type": "invalid_type"
        });

        let result = validator.load_schema("user", &invalid_schema);
        assert!(result.is_err());

        if let Err(SettingsError::Validation(msg)) = result {
            assert!(msg.contains("Schema compilation failed"));
        } else {
            panic!("Expected ValidationError");
        }
    }

    #[test]
    fn test_schema_validator_validate_valid_data() {
        let mut validator = SchemaValidator::new();
        let schema = create_test_schema();
        validator.load_schema("user", &schema).unwrap();

        let valid_data = create_valid_config_data();
        let result = validator.validate("user", &valid_data);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_schema_validator_validate_invalid_data() {
        let mut validator = SchemaValidator::new();
        let schema = create_test_schema();
        validator.load_schema("user", &schema).unwrap();

        let invalid_data = create_invalid_config_data();
        let result = validator.validate("user", &invalid_data);

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());

        // Should have multiple validation errors for invalid data
        let error_count = result.errors.len();
        assert!(error_count > 0);
    }

    #[test]
    fn test_schema_validator_validate_unknown_schema() {
        let validator = SchemaValidator::new();
        let data = create_valid_config_data();
        let result = validator.validate("unknown_schema", &data);

        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0]
            .message
            .contains("Schema 'unknown_schema' not found"));
        assert!(matches!(
            result.errors[0].severity,
            ValidationSeverity::Error
        ));
    }

    #[tokio::test]
    async fn test_config_manager_new() {
        let storage = Box::new(MockStorage::new());
        let manager = ConfigManager::new(storage);

        // Basic verification that manager is created
        assert!(manager.validator.schemas.is_empty());
    }

    #[tokio::test]
    async fn test_config_manager_create_config() {
        let mut storage = MockStorage::new();
        let config_data = create_valid_config_data();
        let config_path = "/test/config";

        // Ensure config doesn't exist initially
        storage.set_get_result(config_key(config_path), None);

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager
            .create_config(
                config_path,
                config_data.clone(),
                "user",
                "test_author",
                Some("Test comment".to_string()),
                None,
            )
            .await;

        assert!(result.is_ok());
        let config = result.unwrap();
        assert_eq!(config.path, config_path);
        assert_eq!(config.content, config_data);
        assert_eq!(config.metadata.version, 1);
        assert_eq!(config.metadata.author, "test_author");
        assert_eq!(config.metadata.schema_type, "user");
    }

    #[tokio::test]
    async fn test_config_manager_create_config_already_exists() {
        let mut storage = MockStorage::new();
        let config_path = "/test/config";

        // Set up storage to return existing config
        let existing_config = create_test_config();
        let existing_config_json = serde_json::to_string(&existing_config).unwrap();
        storage.set_get_result(config_key(config_path), Some(existing_config_json));

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager
            .create_config(
                config_path,
                create_valid_config_data(),
                "user",
                "test_author",
                None,
                None,
            )
            .await;

        assert!(result.is_err());
        if let Err(SettingsError::Config(msg)) = result {
            assert!(msg.contains("Configuration already exists"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[tokio::test]
    async fn test_config_manager_load_config() {
        let mut storage = MockStorage::new();
        let config = create_test_config();
        let config_json = serde_json::to_value(&config).unwrap();

        storage.set_get_json_result(config_key(&config.path), Some(config_json));

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager.load_config(&config.path).await;
        assert!(result.is_ok());

        let loaded_config = result.unwrap();
        assert_eq!(loaded_config.path, config.path);
        assert_eq!(loaded_config.content, config.content);
        assert_eq!(loaded_config.metadata.version, config.metadata.version);
    }

    #[tokio::test]
    async fn test_config_manager_load_config_not_found() {
        let mut storage = MockStorage::new();
        let config_path = "/nonexistent/config";

        storage.set_get_json_result(config_key(config_path), None);

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager.load_config(config_path).await;
        assert!(result.is_err());

        if let Err(SettingsError::Config(msg)) = result {
            assert!(msg.contains("Configuration not found"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[tokio::test]
    async fn test_config_manager_update_config() {
        let mut storage = MockStorage::new();
        let mut config = create_test_config();
        let config_json = serde_json::to_value(&config).unwrap();

        storage.set_get_json_result(config_key(&config.path), Some(config_json));

        let mut manager = ConfigManager::new(Box::new(storage));

        let new_content = json!({
            "name": "Jane Doe",
            "age": 25,
            "email": "jane.doe@example.com"
        });

        let result = manager
            .update_config(
                &config.path,
                new_content.clone(),
                "new_author",
                Some("Updated config".to_string()),
                None,
            )
            .await;

        assert!(result.is_ok());
        let updated_config = result.unwrap();

        assert_eq!(updated_config.content, new_content);
        assert_eq!(updated_config.metadata.version, 2); // Version should be incremented
        assert_eq!(updated_config.metadata.author, "new_author");
        assert_eq!(
            updated_config.metadata.comment,
            Some("Updated config".to_string())
        );
    }

    #[tokio::test]
    async fn test_config_manager_delete_config() {
        let mut storage = MockStorage::new();
        let config = create_test_config();
        let config_json = serde_json::to_value(&config).unwrap();

        storage.set_get_json_result(config_key(&config.path), Some(config_json));
        storage.set_delete_result(config_key(&config.path), true);

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager.delete_config(&config.path, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_manager_save_and_get_schema() {
        let mut storage = MockStorage::new();
        let schema = create_test_schema();
        let schema_type = "user";

        storage.set_get_json_result(schema_key(schema_type), Some(schema.clone()));

        let mut manager = ConfigManager::new(Box::new(storage));

        // Test save schema
        let save_result = manager.save_schema(schema_type, &schema).await;
        assert!(save_result.is_ok());

        // Test get schema
        let get_result = manager.get_schema(schema_type).await;
        assert!(get_result.is_ok());

        let retrieved_schema = get_result.unwrap();
        assert_eq!(retrieved_schema, schema);
    }

    #[tokio::test]
    async fn test_config_manager_get_schema_not_found() {
        let mut storage = MockStorage::new();
        let schema_type = "nonexistent";

        storage.set_get_json_result(schema_key(schema_type), None);

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager.get_schema(schema_type).await;
        assert!(result.is_err());

        if let Err(SettingsError::Config(msg)) = result {
            assert!(msg.contains("Schema not found"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[tokio::test]
    async fn test_config_manager_validate_config() {
        let mut storage = MockStorage::new();
        let schema = create_test_schema();
        let config = create_test_config();

        // Set up schema in storage
        storage.set_get_json_result(schema_key(&config.metadata.schema_type), Some(schema));

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager.validate_config(&config).await;
        assert!(result.is_ok());

        let validation_result = result.unwrap();
        assert!(validation_result.is_valid);
        assert!(validation_result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_config_manager_validate_config_invalid_data() {
        let mut storage = MockStorage::new();
        let schema = create_test_schema();
        let mut config = create_test_config();
        config.content = create_invalid_config_data();

        // Set up schema in storage
        storage.set_get_json_result(schema_key(&config.metadata.schema_type), Some(schema));

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager.validate_config(&config).await;
        assert!(result.is_ok());

        let validation_result = result.unwrap();
        assert!(!validation_result.is_valid);
        assert!(!validation_result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_config_manager_list_configs() {
        let mut storage = MockStorage::new();
        let config1 = create_test_config();
        let mut config2 = create_test_config();
        config2.path = "/test/config2".to_string();
        config2.metadata.version = 2;

        let mut configs_map = HashMap::new();
        configs_map.insert(
            config_key(&config1.path),
            serde_json::to_string(&config1).unwrap(),
        );
        configs_map.insert(
            config_key(&config2.path),
            serde_json::to_string(&config2).unwrap(),
        );

        storage.set_list_result(
            format!("{}test", crate::settings_storage::KeyPrefixes::CONFIG),
            configs_map,
        );

        let mut manager = ConfigManager::new(Box::new(storage));

        let result = manager.list_configs(Some("test")).await;
        assert!(result.is_ok());

        let summaries = result.unwrap();
        assert_eq!(summaries.len(), 2);

        // Verify summary contents
        let summary1 = summaries.iter().find(|s| s.path == config1.path).unwrap();
        assert_eq!(summary1.version, config1.metadata.version);
        assert_eq!(summary1.schema_type, config1.metadata.schema_type);
        assert_eq!(summary1.author, config1.metadata.author);

        let summary2 = summaries.iter().find(|s| s.path == config2.path).unwrap();
        assert_eq!(summary2.version, config2.metadata.version);
        assert_eq!(summary2.schema_type, config2.metadata.schema_type);
    }

    #[test]
    fn test_validation_severity_variants() {
        // Test all variants can be created
        let error = ValidationSeverity::Error;
        let warning = ValidationSeverity::Warning;
        let info = ValidationSeverity::Info;

        // Basic match testing
        match error {
            ValidationSeverity::Error => (),
            _ => panic!("Expected Error variant"),
        }

        match warning {
            ValidationSeverity::Warning => (),
            _ => panic!("Expected Warning variant"),
        }

        match info {
            ValidationSeverity::Info => (),
            _ => panic!("Expected Info variant"),
        }
    }

    #[test]
    fn test_validation_result_with_warnings() {
        let result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: vec![ValidationErrorDetail {
                path: "/optional_field".to_string(),
                message: "Field is deprecated".to_string(),
                severity: ValidationSeverity::Warning,
            }],
        };

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].message, "Field is deprecated");
    }

    #[test]
    fn test_validation_result_with_mixed_severities() {
        let result = ValidationResult {
            is_valid: false,
            errors: vec![ValidationErrorDetail {
                path: "/required_field".to_string(),
                message: "Field is required".to_string(),
                severity: ValidationSeverity::Error,
            }],
            warnings: vec![ValidationErrorDetail {
                path: "/optional_field".to_string(),
                message: "Field is deprecated".to_string(),
                severity: ValidationSeverity::Warning,
            }],
        };

        assert!(!result.is_valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.warnings.len(), 1);

        // Verify error details
        assert!(matches!(
            result.errors[0].severity,
            ValidationSeverity::Error
        ));
        assert!(matches!(
            result.warnings[0].severity,
            ValidationSeverity::Warning
        ));
    }
}

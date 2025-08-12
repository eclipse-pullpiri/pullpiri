// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Configuration management module

use crate::settings_storage::{Storage, config_key, schema_key};
use crate::settings_utils::error::{SettingsError, ValidationError};
use crate::settings_utils::yaml;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jsonschema::{JSONSchema, ValidationError as JsonSchemaError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, info, warn, error};
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
        
        self.schemas.insert(schema_type.to_string(), compiled_schema);
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
            let config: Config = serde_json::from_value(config_data)
                .map_err(|e| SettingsError::Config(format!("Failed to deserialize config: {}", e)))?;
            
            Ok(config)
        } else {
            Err(SettingsError::Config(format!("Configuration not found: {}", config_path)))
        }
    }

    /// Save configuration
    pub async fn save_config(&mut self, config: &Config) -> Result<(), SettingsError> {
        info!("Saving config: {} (version {})", config.path, config.metadata.version);
        
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
    ) -> Result<Config, SettingsError> {
        // Check if config already exists
        let key = config_key(path);
        if self.storage.get(&key).await?.is_some() {
            return Err(SettingsError::Config(format!("Configuration already exists: {}", path)));
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
        Ok(config)
    }

    /// Update configuration
    pub async fn update_config(
        &mut self,
        path: &str,
        content: Value,
        author: &str,
        comment: Option<String>,
    ) -> Result<Config, SettingsError> {
        let mut config = self.load_config(path).await?;
        
        config.content = content;
        config.metadata.version += 1;
        config.metadata.modified_at = Utc::now();
        config.metadata.author = author.to_string();
        config.metadata.comment = comment;

        self.save_config(&config).await?;
        Ok(config)
    }

    /// Delete configuration
    pub async fn delete_config(&mut self, config_path: &str) -> Result<(), SettingsError> {
        info!("Deleting config: {}", config_path);
        
        let key = config_key(config_path);
        if !self.storage.delete(&key).await? {
            warn!("Configuration not found for deletion: {}", config_path);
        }
        
        Ok(())
    }

    /// List configurations with optional prefix filter
    pub async fn list_configs(&mut self, prefix: Option<&str>) -> Result<Vec<ConfigSummary>, SettingsError> {
        debug!("Listing configs with prefix: {:?}", prefix);
        
        let search_prefix = format!("{}{}", 
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
    pub async fn validate_config(&mut self, config: &Config) -> Result<ValidationResult, SettingsError> {
        debug!("Validating config: {} with schema: {}", config.path, config.metadata.schema_type);
        
        // Load schema if not already loaded
        if !self.validator.schemas.contains_key(&config.metadata.schema_type) {
            self.load_schema(&config.metadata.schema_type).await?;
        }

        Ok(self.validator.validate(&config.metadata.schema_type, &config.content))
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
    pub async fn save_schema(&mut self, schema_type: &str, schema: &Value) -> Result<(), SettingsError> {
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
            Err(SettingsError::Config(format!("Schema not found: {}", schema_type)))
        }
    }
}
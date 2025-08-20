// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! ETCD storage client module

use crate::settings_utils::error::{SettingsError, StorageError};
use async_trait::async_trait;
use etcd_client::{Client, ConnectOptions, DeleteOptions, GetOptions, PutOptions};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, warn};

/// ETCD client wrapper for Settings Service
pub struct EtcdClient {
    client: Client,
}

impl EtcdClient {
    /// Create a new ETCD client
    pub async fn new(endpoints: Vec<String>) -> Result<Self, StorageError> {
        debug!("Connecting to ETCD endpoints: {:?}", endpoints);

        let client = Client::connect(endpoints, Some(ConnectOptions::new()))
            .await
            .map_err(|e| {
                StorageError::ConnectionFailed(format!("Failed to connect to ETCD: {}", e))
            })?;

        Ok(Self { client })
    }

    /// Get a value by key
    pub async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
        debug!("Getting key: {}", key);

        let resp =
            self.client.get(key, None).await.map_err(|e| {
                StorageError::OperationFailed(format!("Get operation failed: {}", e))
            })?;

        if let Some(kv) = resp.kvs().first() {
            let value = String::from_utf8(kv.value().to_vec())
                .map_err(|e| StorageError::SerializationError(format!("Invalid UTF-8: {}", e)))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Put a key-value pair
    pub async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        debug!("Putting key: {}, value length: {}", key, value.len());

        self.client
            .put(key, value, None)
            .await
            .map_err(|e| StorageError::OperationFailed(format!("Put operation failed: {}", e)))?;

        Ok(())
    }

    /// Delete a key
    pub async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
        debug!("Deleting key: {}", key);

        let resp = self.client.delete(key, None).await.map_err(|e| {
            StorageError::OperationFailed(format!("Delete operation failed: {}", e))
        })?;

        Ok(resp.deleted() > 0)
    }

    /// List keys with a prefix
    pub async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
        debug!("Listing keys with prefix: {}", prefix);

        let get_options = Some(GetOptions::new().with_prefix());
        let resp =
            self.client.get(prefix, get_options).await.map_err(|e| {
                StorageError::OperationFailed(format!("List operation failed: {}", e))
            })?;

        let mut results = Vec::new();
        for kv in resp.kvs() {
            let key = String::from_utf8(kv.key().to_vec()).map_err(|e| {
                StorageError::SerializationError(format!("Invalid UTF-8 in key: {}", e))
            })?;
            let value = String::from_utf8(kv.value().to_vec()).map_err(|e| {
                StorageError::SerializationError(format!("Invalid UTF-8 in value: {}", e))
            })?;
            results.push((key, value));
        }

        Ok(results)
    }

    /// Get JSON value by key
    pub async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError> {
        if let Some(value_str) = self.get(key).await? {
            let value: Value = serde_json::from_str(&value_str).map_err(|e| {
                StorageError::SerializationError(format!("JSON parse error: {}", e))
            })?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Put JSON value by key
    pub async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
        let value_str = serde_json::to_string(value).map_err(|e| {
            StorageError::SerializationError(format!("JSON serialize error: {}", e))
        })?;

        self.put(key, &value_str).await
    }
}

/// Storage interface trait for dependency injection and testing
#[async_trait]
pub trait Storage: Send + Sync {
    async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError>;
    async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError>;
    async fn delete(&mut self, key: &str) -> Result<bool, StorageError>;
    async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError>;
    async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError>;
    async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError>;
}

#[async_trait]
impl Storage for EtcdClient {
    async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
        self.get(key).await
    }

    async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        self.put(key, value).await
    }

    async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
        self.delete(key).await
    }

    async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
        self.list(prefix).await
    }

    async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError> {
        self.get_json(key).await
    }

    async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
        self.put_json(key, value).await
    }
}

/// Key prefixes for different data types
pub struct KeyPrefixes;

impl KeyPrefixes {
    pub const CONFIG: &'static str = "/piccolo/settings/configs/";
    pub const HISTORY: &'static str = "/piccolo/settings/history/";
    pub const METRICS: &'static str = "/piccolo/metrics/";
    pub const FILTERS: &'static str = "/piccolo/settings/filters/";
    pub const SCHEMAS: &'static str = "/piccolo/settings/schemas/";
}

/// Helper functions for key management
pub fn config_key(path: &str) -> String {
    format!("{}{}", KeyPrefixes::CONFIG, path)
}

pub fn history_key(config_path: &str, version: u64) -> String {
    format!("{}{}/v{}", KeyPrefixes::HISTORY, config_path, version)
}

pub fn metrics_key(resource_type: &str, resource_id: &str) -> String {
    format!("{}{}/{}", KeyPrefixes::METRICS, resource_type, resource_id)
}

pub fn filter_key(filter_id: &str) -> String {
    format!("{}{}", KeyPrefixes::FILTERS, filter_id)
}

pub fn schema_key(schema_type: &str) -> String {
    format!("{}{}", KeyPrefixes::SCHEMAS, schema_type)
}

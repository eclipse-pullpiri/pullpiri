/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Persistency Service Client
//!
//! This module provides a client interface to the persistency service,
//! replacing direct RocksDB/etcd usage with gRPC calls to the persistency service.

use crate::persistency_proto::{
    persistency_service_client::PersistencyServiceClient,
    BatchPutRequest, DeleteRequest, GetByPrefixRequest, GetRequest,
    HealthRequest, KeyKvsValue, KvsValue, ListKeysRequest, PutRequest,
};
use tonic::transport::{Channel, Error as TonicError};
use tonic::Status;

/// Key-Value pair for compatibility with existing code
#[derive(Debug, Clone)]
pub struct KV {
    pub key: String,
    pub value: String,
}

/// Client for the persistency service
pub struct PersistencyClient {
    client: PersistencyServiceClient<Channel>,
}

/// Custom error type for persistency operations
#[derive(Debug)]
pub enum PersistencyError {
    Transport(TonicError),
    Grpc(Status),
    Conversion(String),
    NotFound,
    InvalidArgs(String),
}

impl From<TonicError> for PersistencyError {
    fn from(err: TonicError) -> Self {
        PersistencyError::Transport(err)
    }
}

impl From<Status> for PersistencyError {
    fn from(err: Status) -> Self {
        PersistencyError::Grpc(err)
    }
}

impl std::fmt::Display for PersistencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistencyError::Transport(e) => write!(f, "Transport error: {}", e),
            PersistencyError::Grpc(e) => write!(f, "gRPC error: {}", e),
            PersistencyError::Conversion(e) => write!(f, "Conversion error: {}", e),
            PersistencyError::NotFound => write!(f, "Key not found"),
            PersistencyError::InvalidArgs(e) => write!(f, "Invalid arguments: {}", e),
        }
    }
}

impl std::error::Error for PersistencyError {}

impl PersistencyClient {
    /// Create a new persistency client
    pub async fn new() -> Result<Self, PersistencyError> {
        let endpoint = crate::persistency_proto::connect_server();
        let client = PersistencyServiceClient::connect(endpoint).await?;

        Ok(Self { client })
    }

    /// Helper function to convert string to KvsValue
    fn string_to_kvs_value(value: &str) -> KvsValue {
        KvsValue {
            value: Some(crate::persistency_proto::kvs_value::Value::StringValue(
                value.to_string(),
            )),
        }
    }

    /// Helper function to convert KvsValue to string
    fn kvs_value_to_string(value: &KvsValue) -> Result<String, PersistencyError> {
        match &value.value {
            Some(crate::persistency_proto::kvs_value::Value::StringValue(s)) => Ok(s.clone()),
            Some(crate::persistency_proto::kvs_value::Value::I32Value(v)) => Ok(v.to_string()),
            Some(crate::persistency_proto::kvs_value::Value::U32Value(v)) => Ok(v.to_string()),
            Some(crate::persistency_proto::kvs_value::Value::I64Value(v)) => Ok(v.to_string()),
            Some(crate::persistency_proto::kvs_value::Value::U64Value(v)) => Ok(v.to_string()),
            Some(crate::persistency_proto::kvs_value::Value::F64Value(v)) => Ok(v.to_string()),
            Some(crate::persistency_proto::kvs_value::Value::BooleanValue(v)) => {
                Ok(v.to_string())
            }
            Some(crate::persistency_proto::kvs_value::Value::NullValue(_)) => {
                Ok("null".to_string())
            }
            Some(crate::persistency_proto::kvs_value::Value::ArrayValue(_)) => Err(
                PersistencyError::Conversion(
                    "Complex types not supported in string conversion".to_string(),
                ),
            ),
            Some(crate::persistency_proto::kvs_value::Value::ObjectValue(_)) => Err(
                PersistencyError::Conversion(
                    "Complex types not supported in string conversion".to_string(),
                ),
            ),
            None => Err(PersistencyError::Conversion("Empty value".to_string())),
        }
    }

    /// Put a key-value pair
    pub async fn put(&mut self, key: &str, value: &str) -> Result<(), PersistencyError> {
        let request = PutRequest {
            key: key.to_string(),
            value: Some(Self::string_to_kvs_value(value)),
        };

        let response = self.client.put(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(())
        } else {
            Err(PersistencyError::InvalidArgs(response.error))
        }
    }

    /// Get a value by key
    pub async fn get(&mut self, key: &str) -> Result<String, PersistencyError> {
        let request = GetRequest {
            key: key.to_string(),
        };

        let response = self.client.get(request).await?;
        let response = response.into_inner();

        if response.success {
            if let Some(value) = response.value {
                Self::kvs_value_to_string(&value)
            } else {
                Err(PersistencyError::NotFound)
            }
        } else {
            Err(PersistencyError::NotFound)
        }
    }

    /// Get all key-value pairs with a given prefix
    pub async fn get_all_with_prefix(
        &mut self,
        prefix: &str,
    ) -> Result<Vec<KV>, PersistencyError> {
        let request = GetByPrefixRequest {
            prefix: prefix.to_string(),
            limit: 0, // 0 means no limit
        };

        let response = self.client.get_by_prefix(request).await?;
        let response = response.into_inner();

        if response.error.is_empty() {
            let mut kv_pairs = Vec::new();
            for pair in response.pairs {
                if let Some(value) = pair.value {
                    match Self::kvs_value_to_string(&value) {
                        Ok(value_str) => {
                            kv_pairs.push(KV {
                                key: pair.key,
                                value: value_str,
                            });
                        }
                        Err(_) => {
                            // Skip complex values that can't be converted to strings
                            continue;
                        }
                    }
                }
            }
            Ok(kv_pairs)
        } else {
            Err(PersistencyError::InvalidArgs(response.error))
        }
    }

    /// Delete a key
    pub async fn delete(&mut self, key: &str) -> Result<(), PersistencyError> {
        let request = DeleteRequest {
            key: key.to_string(),
        };

        let response = self.client.delete(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(())
        } else {
            Err(PersistencyError::InvalidArgs(response.error))
        }
    }

    /// Delete all keys with a given prefix
    pub async fn delete_all_with_prefix(
        &mut self,
        prefix: &str,
    ) -> Result<(), PersistencyError> {
        // First get all keys with the prefix
        let kv_pairs = self.get_all_with_prefix(prefix).await?;

        // Then delete each key individually
        for kv in kv_pairs {
            self.delete(&kv.key).await?;
        }

        Ok(())
    }

    /// Batch put operation to store multiple key-value pairs
    pub async fn batch_put(
        &mut self,
        items: Vec<(String, String)>,
    ) -> Result<(), PersistencyError> {
        let pairs: Vec<KeyKvsValue> = items
            .into_iter()
            .map(|(key, value)| KeyKvsValue {
                key,
                value: Some(Self::string_to_kvs_value(&value)),
            })
            .collect();

        let request = BatchPutRequest { pairs };

        let response = self.client.batch_put(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(())
        } else {
            Err(PersistencyError::InvalidArgs(response.error))
        }
    }

    /// Health check for the persistency service
    pub async fn health_check(&mut self) -> Result<bool, PersistencyError> {
        let request = HealthRequest {};

        let response = self.client.health(request).await?;
        let response = response.into_inner();

        Ok(response.status == "healthy")
    }

    /// List keys with optional prefix filter
    pub async fn list_keys(
        &mut self,
        prefix: &str,
        limit: i32,
    ) -> Result<Vec<String>, PersistencyError> {
        let request = ListKeysRequest {
            prefix: prefix.to_string(),
            limit,
        };

        let response = self.client.list_keys(request).await?;
        let response = response.into_inner();

        if response.error.is_empty() {
            Ok(response.keys)
        } else {
            Err(PersistencyError::InvalidArgs(response.error))
        }
    }

    /// Reset all data (for testing/development)
    pub async fn reset(&mut self) -> Result<(), PersistencyError> {
        let request = crate::persistency_proto::ResetRequest {};
        let response = self.client.reset(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(())
        } else {
            Err(PersistencyError::InvalidArgs(response.error))
        }
    }

    /// Flush data to persistent storage
    pub async fn flush(&mut self) -> Result<(), PersistencyError> {
        let request = crate::persistency_proto::FlushRequest {};
        let response = self.client.flush(request).await?;
        let response = response.into_inner();

        if response.success {
            Ok(())
        } else {
            Err(PersistencyError::InvalidArgs(response.error))
        }
    }
}
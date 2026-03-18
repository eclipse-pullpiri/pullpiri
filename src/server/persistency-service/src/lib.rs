/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Persistency Service - A gRPC service wrapper for the rust_kvs library
//!
//! This service provides a centralized persistency backend for all Pullpiri components,
//! replacing PERSISTENCY usage. It wraps the rust_kvs library and exposes it through gRPC.

use common::persistency_proto::{
    persistency_service_server::{PersistencyService, PersistencyServiceServer},
    BatchPutRequest, BatchPutResponse, DeleteRequest, DeleteResponse, FlushRequest, FlushResponse,
    GetByPrefixRequest, GetByPrefixResponse, GetRequest, GetResponse, HealthRequest,
    HealthResponse, KeyExistsRequest, KeyExistsResponse, KeyKvsValue, KvsArray, KvsObject,
    KvsValue, ListKeysRequest, ListKeysResponse, NullValue, PutRequest, PutResponse, ResetRequest,
    ResetResponse,
};
use rust_kvs::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

/// Persistency Service Implementation
pub struct PersistencyServiceImpl {
    kvs: Arc<RwLock<Kvs>>,
}

impl PersistencyServiceImpl {
    /// Create a new persistency service instance
    pub fn new() -> Result<Self, ErrorCode> {
        info!("Initializing persistency service with rust_kvs");

        // Log current working directory where files will be created
        let current_dir =
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        info!("Storage files will be created in: {:?}", current_dir);

        let kvs = KvsBuilder::new(InstanceId(0)).build()?;

        info!("Persistency service initialized successfully");

        Ok(Self {
            kvs: Arc::new(RwLock::new(kvs)),
        })
    }

    /// Convert rust_kvs::KvsValue to protobuf KvsValue
    fn kvs_value_to_proto(value: &rust_kvs::kvs_value::KvsValue) -> KvsValue {
        use rust_kvs::kvs_value::KvsValue as RustKvsValue;

        match value {
            RustKvsValue::I32(v) => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::I32Value(*v)),
            },
            RustKvsValue::U32(v) => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::U32Value(*v)),
            },
            RustKvsValue::I64(v) => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::I64Value(*v)),
            },
            RustKvsValue::U64(v) => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::U64Value(*v)),
            },
            RustKvsValue::F64(v) => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::F64Value(*v)),
            },
            RustKvsValue::Boolean(v) => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::BooleanValue(
                    *v,
                )),
            },
            RustKvsValue::String(v) => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::StringValue(
                    v.clone(),
                )),
            },
            RustKvsValue::Null => KvsValue {
                value: Some(common::persistency_proto::kvs_value::Value::NullValue(
                    NullValue {},
                )),
            },
            RustKvsValue::Array(v) => {
                let values = v.iter().map(Self::kvs_value_to_proto).collect();
                KvsValue {
                    value: Some(common::persistency_proto::kvs_value::Value::ArrayValue(
                        KvsArray { values },
                    )),
                }
            }
            RustKvsValue::Object(v) => {
                let values = v
                    .iter()
                    .map(|(k, v)| (k.clone(), Self::kvs_value_to_proto(v)))
                    .collect();
                KvsValue {
                    value: Some(
                        common::persistency_proto::kvs_value::Value::ObjectValue(KvsObject {
                            values,
                        }),
                    ),
                }
            }
        }
    }

    /// Convert protobuf KvsValue to rust_kvs::KvsValue
    fn proto_to_kvs_value(value: &KvsValue) -> Result<rust_kvs::kvs_value::KvsValue, String> {
        use common::persistency_proto::kvs_value::Value;
        use rust_kvs::kvs_value::KvsValue as RustKvsValue;

        match &value.value {
            Some(Value::I32Value(v)) => Ok(RustKvsValue::I32(*v)),
            Some(Value::U32Value(v)) => Ok(RustKvsValue::U32(*v)),
            Some(Value::I64Value(v)) => Ok(RustKvsValue::I64(*v)),
            Some(Value::U64Value(v)) => Ok(RustKvsValue::U64(*v)),
            Some(Value::F64Value(v)) => Ok(RustKvsValue::F64(*v)),
            Some(Value::BooleanValue(v)) => Ok(RustKvsValue::Boolean(*v)),
            Some(Value::StringValue(v)) => Ok(RustKvsValue::String(v.clone())),
            Some(Value::NullValue(_)) => Ok(RustKvsValue::Null),
            Some(Value::ArrayValue(arr)) => {
                let mut values = Vec::new();
                for proto_val in &arr.values {
                    values.push(Self::proto_to_kvs_value(proto_val)?);
                }
                Ok(RustKvsValue::Array(values))
            }
            Some(Value::ObjectValue(obj)) => {
                let mut values = HashMap::new();
                for (key, proto_val) in &obj.values {
                    values.insert(key.clone(), Self::proto_to_kvs_value(proto_val)?);
                }
                Ok(RustKvsValue::Object(values))
            }
            None => Err("KvsValue has no value set".to_string()),
        }
    }
}

#[tonic::async_trait]
impl PersistencyService for PersistencyServiceImpl {
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        // Implementation of health check
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            database_path: "persistency_store".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn put(&self, request: Request<PutRequest>) -> Result<Response<PutResponse>, Status> {
        let req = request.into_inner();
        debug!("Put request for key: {}", req.key);

        let kvs = self.kvs.read().await;

        match req.value {
            Some(proto_value) => match Self::proto_to_kvs_value(&proto_value) {
                Ok(rust_value) => match kvs.set_value(&req.key, rust_value) {
                    Ok(_) => {
                        debug!("Successfully set value for key: {}", req.key);

                        if let Err(e) = kvs.flush() {
                            warn!("Failed to flush after setting key {}: {:?}", req.key, e);
                        } else {
                            debug!(
                                "Flushed data to storage files after setting key: {}",
                                req.key
                            );
                        }

                        Ok(Response::new(PutResponse {
                            success: true,
                            error: String::new(),
                        }))
                    }
                    Err(e) => {
                        error!("Failed to set value for key {}: {:?}", req.key, e);
                        Ok(Response::new(PutResponse {
                            success: false,
                            error: format!("Failed to set value: {:?}", e),
                        }))
                    }
                },
                Err(e) => {
                    error!("Failed to convert protobuf value: {}", e);
                    Ok(Response::new(PutResponse {
                        success: false,
                        error: format!("Value conversion error: {}", e),
                    }))
                }
            },
            None => {
                error!("Put request missing value for key: {}", req.key);
                Ok(Response::new(PutResponse {
                    success: false,
                    error: "Missing value in request".to_string(),
                }))
            }
        }
    }

    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let req = request.into_inner();
        debug!("Get request for key: {}", req.key);

        let kvs = self.kvs.read().await;

        match kvs.get_value(&req.key) {
            Ok(rust_value) => {
                let proto_value = Self::kvs_value_to_proto(&rust_value);
                debug!("Successfully retrieved value for key: {}", req.key);
                Ok(Response::new(GetResponse {
                    success: true,
                    value: Some(proto_value),
                    error: String::new(),
                }))
            }
            Err(e) => {
                warn!("Failed to get value for key {}: {:?}", req.key, e);
                Ok(Response::new(GetResponse {
                    success: false,
                    value: None,
                    error: format!("Key not found: {:?}", e),
                }))
            }
        }
    }

    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let req = request.into_inner();
        debug!("Delete request for key: {}", req.key);

        let kvs = self.kvs.read().await;

        match kvs.remove_key(&req.key) {
            Ok(_) => {
                debug!("Successfully removed key: {}", req.key);
                Ok(Response::new(DeleteResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to remove key {}: {:?}", req.key, e);
                Ok(Response::new(DeleteResponse {
                    success: false,
                    error: format!("Failed to remove key: {:?}", e),
                }))
            }
        }
    }

    async fn batch_put(
        &self,
        request: Request<BatchPutRequest>,
    ) -> Result<Response<BatchPutResponse>, Status> {
        let req = request.into_inner();
        debug!("BatchPut request for {} pairs", req.pairs.len());

        let kvs = self.kvs.read().await;
        let mut processed = 0;

        for pair in req.pairs {
            if let Some(proto_value) = pair.value {
                match Self::proto_to_kvs_value(&proto_value) {
                    Ok(rust_value) => match kvs.set_value(&pair.key, rust_value) {
                        Ok(_) => processed += 1,
                        Err(e) => {
                            warn!("Failed to set value for key {} in batch: {:?}", pair.key, e);
                        }
                    },
                    Err(_) => continue,
                }
            }
        }

        if processed > 0 {
            if let Err(e) = kvs.flush() {
                warn!("Failed to flush after batch put: {:?}", e);
            }
        }

        Ok(Response::new(BatchPutResponse {
            success: true,
            processed_count: processed as i32,
            error: String::new(),
        }))
    }

    async fn get_by_prefix(
        &self,
        request: Request<GetByPrefixRequest>,
    ) -> Result<Response<GetByPrefixResponse>, Status> {
        let req = request.into_inner();
        debug!("GetByPrefix request for prefix: {}", req.prefix);

        let kvs = self.kvs.read().await;

        match kvs.get_all_keys() {
            Ok(all_keys) => {
                let mut results = Vec::new();
                let limit = if req.limit > 0 {
                    req.limit as usize
                } else {
                    usize::MAX
                };

                for key in all_keys {
                    if results.len() >= limit {
                        break;
                    }
                    if key.starts_with(&req.prefix) {
                        match kvs.get_value(&key) {
                            Ok(rust_value) => {
                                let proto_value = Self::kvs_value_to_proto(&rust_value);
                                results.push(KeyKvsValue {
                                    key,
                                    value: Some(proto_value),
                                });
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to get value for key {} during prefix search: {:?}",
                                    key, e
                                );
                            }
                        }
                    }
                }

                let count = results.len() as i32;
                debug!(
                    "Successfully retrieved {} keys with prefix '{}'",
                    count, req.prefix
                );
                Ok(Response::new(GetByPrefixResponse {
                    pairs: results,
                    total_count: count,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to get keys for prefix search: {:?}", e);
                Ok(Response::new(GetByPrefixResponse {
                    pairs: vec![],
                    total_count: 0,
                    error: format!("Failed to get keys: {:?}", e),
                }))
            }
        }
    }

    async fn list_keys(
        &self,
        request: Request<ListKeysRequest>,
    ) -> Result<Response<ListKeysResponse>, Status> {
        let req = request.into_inner();
        debug!("ListKeys request");

        let kvs = self.kvs.read().await;

        match kvs.get_all_keys() {
            Ok(all_keys) => {
                let mut keys = Vec::new();
                let limit = if req.limit > 0 {
                    req.limit as usize
                } else {
                    usize::MAX
                };

                for key in all_keys {
                    if keys.len() >= limit {
                        break;
                    }
                    if req.prefix.is_empty() || key.starts_with(&req.prefix) {
                        keys.push(key);
                    }
                }

                let count = keys.len() as i32;
                debug!("Successfully retrieved {} keys", count);
                Ok(Response::new(ListKeysResponse {
                    keys,
                    total_count: count,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to list keys: {:?}", e);
                Ok(Response::new(ListKeysResponse {
                    keys: vec![],
                    total_count: 0,
                    error: format!("Failed to list keys: {:?}", e),
                }))
            }
        }
    }

    async fn key_exists(
        &self,
        request: Request<KeyExistsRequest>,
    ) -> Result<Response<KeyExistsResponse>, Status> {
        let req = request.into_inner();
        debug!("KeyExists request for key: {}", req.key);

        let kvs = self.kvs.read().await;

        match kvs.key_exists(&req.key) {
            Ok(exists) => {
                debug!("Key {} exists: {}", req.key, exists);
                Ok(Response::new(KeyExistsResponse {
                    success: true,
                    exists,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to check if key {} exists: {:?}", req.key, e);
                Ok(Response::new(KeyExistsResponse {
                    success: false,
                    exists: false,
                    error: format!("Failed to check key existence: {:?}", e),
                }))
            }
        }
    }

    async fn reset(
        &self,
        _request: Request<ResetRequest>,
    ) -> Result<Response<ResetResponse>, Status> {
        debug!("Reset request");

        let kvs = self.kvs.read().await;

        match kvs.reset() {
            Ok(_) => {
                info!("Successfully reset KVS");
                Ok(Response::new(ResetResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to reset KVS: {:?}", e);
                Ok(Response::new(ResetResponse {
                    success: false,
                    error: format!("Failed to reset: {:?}", e),
                }))
            }
        }
    }

    async fn flush(
        &self,
        _request: Request<FlushRequest>,
    ) -> Result<Response<FlushResponse>, Status> {
        debug!("Flush request");

        let kvs = self.kvs.read().await;

        match kvs.flush() {
            Ok(_) => {
                debug!("Successfully flushed KVS");
                Ok(Response::new(FlushResponse {
                    success: true,
                    error: String::new(),
                }))
            }
            Err(e) => {
                error!("Failed to flush KVS: {:?}", e);
                Ok(Response::new(FlushResponse {
                    success: false,
                    error: format!("Failed to flush: {:?}", e),
                }))
            }
        }
    }
}
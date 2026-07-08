/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::logd;
use crate::rocksdbservice::{
    rocks_db_service_client::RocksDbServiceClient, BatchPutRequest, DeleteRequest,
    GetByPrefixRequest, GetRequest, HealthRequest, KeyValue, PutRequest,
};

lazy_static::lazy_static! {
    static ref ROCKSDB_SERVICE_URL: String = {
        std::env::var("ROCKSDB_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:47007".to_string())
    };
}

const DEV: bool = false;

/// Put a key-value pair into the gRPC RocksDB service
pub async fn put(key: &str, value: &str) -> Result<(), String> {
    if DEV {
        logd!(
            1,
            "[RocksDB] Putting key '{}' to service: {}",
            key,
            *ROCKSDB_SERVICE_URL
        );
    }

    match RocksDbServiceClient::connect(ROCKSDB_SERVICE_URL.clone()).await {
        Ok(mut client) => {
            let request = tonic::Request::new(PutRequest {
                key: key.to_string(),
                value: value.to_string(),
            });

            match client.put(request).await {
                Ok(response) => {
                    let put_response = response.into_inner();
                    if put_response.success {
                        Ok(())
                    } else {
                        let error_msg = put_response.error;
                        logd!(5, "[RocksDB] Put failed: {}", error_msg);
                        Err(error_msg)
                    }
                }
                Err(e) => {
                    let error_msg = format!("gRPC request failed: {}", e);
                    logd!(5, "[RocksDB] {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create client: {}", e);
            logd!(5, "[RocksDB] {}", error_msg);
            Err(error_msg)
        }
    }
}

/// Get a value by key from the gRPC RocksDB service
pub async fn get(key: &str) -> Result<String, String> {
    if DEV {
        logd!(
            1,
            "[RocksDB] Getting key '{}' from service: {}",
            key,
            *ROCKSDB_SERVICE_URL
        );
    }

    match RocksDbServiceClient::connect(ROCKSDB_SERVICE_URL.clone()).await {
        Ok(mut client) => {
            let request = tonic::Request::new(GetRequest {
                key: key.to_string(),
            });

            match client.get(request).await {
                Ok(response) => {
                    let get_response = response.into_inner();
                    if get_response.success {
                        if DEV {
                            logd!(
                                1,
                                "[RocksDB] Successfully retrieved key: {} (value length: {})",
                                key,
                                get_response.value.len()
                            );
                        }
                        Ok(get_response.value)
                    } else {
                        logd!(5, "[RocksDB] Key not found: {}", key);
                        Err("Key not found".to_string())
                    }
                }
                Err(e) => {
                    let error_msg = format!("gRPC request failed: {}", e);
                    logd!(5, "[RocksDB] {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create client: {}", e);
            logd!(5, "[RocksDB] {}", error_msg);
            Err(error_msg)
        }
    }
}

/// Get all key-value pairs with the specified prefix using gRPC RocksDB service
pub async fn get_all_with_prefix(prefix: &str) -> Result<Vec<(String, String)>, String> {
    if DEV {
        logd!(
            1,
            "[RocksDB] Getting all keys with prefix '{}' from service: {}",
            prefix,
            *ROCKSDB_SERVICE_URL
        );
    }

    match RocksDbServiceClient::connect(ROCKSDB_SERVICE_URL.clone()).await {
        Ok(mut client) => {
            let request = tonic::Request::new(GetByPrefixRequest {
                prefix: prefix.to_string(),
                limit: 0, // 0 means no limit
            });

            match client.get_by_prefix(request).await {
                Ok(response) => {
                    let get_response = response.into_inner();
                    if get_response.error.is_empty() {
                        let result: Vec<(String, String)> = get_response
                            .pairs
                            .into_iter()
                            .map(|kv| (kv.key, kv.value))
                            .collect();
                        if DEV {
                            logd!(
                                1,
                                "[RocksDB] Successfully retrieved {} keys with prefix '{}'",
                                result.len(),
                                prefix
                            );
                        }
                        Ok(result)
                    } else {
                        logd!(5, "[RocksDB] Error from service: {}", get_response.error);
                        Err(get_response.error)
                    }
                }
                Err(e) => {
                    let error_msg = format!("gRPC request failed: {}", e);
                    logd!(5, "[RocksDB] {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create client: {}", e);
            logd!(5, "[RocksDB] {}", error_msg);
            Err(error_msg)
        }
    }
}

/// Delete a key from the gRPC RocksDB service
pub async fn delete(key: &str) -> Result<(), String> {
    if DEV {
        logd!(
            1,
            "[RocksDB] Deleting key '{}' from service: {}",
            key,
            *ROCKSDB_SERVICE_URL
        );
    }

    match RocksDbServiceClient::connect(ROCKSDB_SERVICE_URL.clone()).await {
        Ok(mut client) => {
            let request = tonic::Request::new(DeleteRequest {
                key: key.to_string(),
            });

            match client.delete(request).await {
                Ok(response) => {
                    let delete_response = response.into_inner();
                    if delete_response.success {
                        if DEV {
                            logd!(1, "[RocksDB] Successfully deleted key: {}", key);
                        }
                        Ok(())
                    } else {
                        let error_msg = delete_response.error;
                        logd!(5, "[RocksDB] Delete failed: {}", error_msg);
                        Err(error_msg)
                    }
                }
                Err(e) => {
                    let error_msg = format!("gRPC request failed: {}", e);
                    logd!(5, "[RocksDB] {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create client: {}", e);
            logd!(5, "[RocksDB] {}", error_msg);
            Err(error_msg)
        }
    }
}

/// Batch put operation to store multiple key-value pairs using gRPC RocksDB service
pub async fn batch_put(items: Vec<(String, String)>) -> Result<(), String> {
    if DEV {
        logd!(
            1,
            "[RocksDB] Batch putting {} items to service: {}",
            items.len(),
            *ROCKSDB_SERVICE_URL
        );
    }

    match RocksDbServiceClient::connect(ROCKSDB_SERVICE_URL.clone()).await {
        Ok(mut client) => {
            let pairs: Vec<KeyValue> = items
                .into_iter()
                .map(|(key, value)| KeyValue { key, value })
                .collect();

            let request = tonic::Request::new(BatchPutRequest { pairs });

            match client.batch_put(request).await {
                Ok(response) => {
                    let batch_response = response.into_inner();
                    if batch_response.success {
                        if DEV {
                            logd!(
                                1,
                                "[RocksDB] Successfully stored {} items in batch",
                                batch_response.processed_count
                            );
                        }
                        Ok(())
                    } else {
                        let error_msg = batch_response.error;
                        logd!(5, "[RocksDB] Batch put failed: {}", error_msg);
                        Err(error_msg)
                    }
                }
                Err(e) => {
                    let error_msg = format!("gRPC request failed: {}", e);
                    logd!(5, "[RocksDB] {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create client: {}", e);
            logd!(5, "[RocksDB] {}", error_msg);
            Err(error_msg)
        }
    }
}

/// Health check for the gRPC RocksDB service
pub async fn health_check() -> Result<bool, String> {
    if DEV {
        logd!(
            1,
            "[RocksDB] Health check for service: {}",
            *ROCKSDB_SERVICE_URL
        );
    }

    match RocksDbServiceClient::connect(ROCKSDB_SERVICE_URL.clone()).await {
        Ok(mut client) => {
            let request = tonic::Request::new(HealthRequest {});

            match client.health(request).await {
                Ok(response) => {
                    let health_response = response.into_inner();
                    let is_healthy = health_response.status == "healthy";
                    if DEV {
                        logd!(
                            1,
                            "[RocksDB] Health check result: {}",
                            health_response.status
                        );
                    }
                    Ok(is_healthy)
                }
                Err(e) => {
                    let error_msg = format!("Health check failed: {}", e);
                    logd!(5, "[RocksDB] {}", error_msg);
                    Err(error_msg)
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Failed to create client: {}", e);
            logd!(5, "[RocksDB] {}", error_msg);
            Err(error_msg)
        }
    }
}

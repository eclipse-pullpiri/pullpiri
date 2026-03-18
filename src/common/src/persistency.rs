/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Persistency Service Interface
//!
//! This module provides a high-level interface to the persistency service,
//! offering the same API as the etcd module for drop-in replacement.
//! Functions return `Result<T, String>` to match the existing etcd interface.

use crate::logd;
use crate::persistency_client::PersistencyClient;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEV: bool = false;

/// Lazy static client instance for global access with retry logic
static CLIENT: tokio::sync::OnceCell<Arc<Mutex<PersistencyClient>>> =
    tokio::sync::OnceCell::const_new();

/// Get or initialize the global persistency client
async fn get_client() -> Result<Arc<Mutex<PersistencyClient>>, String> {
    const MAX_RETRIES: u32 = 10;
    const RETRY_DELAY_MS: u64 = 1000;

    CLIENT
        .get_or_try_init(|| async {
            let mut attempt = 0;
            let mut last_error = None;

            while attempt < MAX_RETRIES {
                match PersistencyClient::new().await {
                    Ok(client) => {
                        return Ok(Arc::new(Mutex::new(client)));
                    }
                    Err(err) => {
                        logd!(
                            5,
                            "[Persistency] Failed to connect (attempt {}/{}): {}",
                            attempt + 1,
                            MAX_RETRIES,
                            err
                        );
                        last_error = Some(format!("{}", err));
                        attempt += 1;

                        if attempt < MAX_RETRIES {
                            tokio::time::sleep(tokio::time::Duration::from_millis(RETRY_DELAY_MS))
                                .await;
                        }
                    }
                }
            }

            Err(last_error.unwrap_or_else(|| {
                "Failed to connect to persistency service after multiple attempts".to_string()
            }))
        })
        .await
        .map(|client| client.clone())
}

/// Put a key-value pair into the persistency service
pub async fn put(key: &str, value: &str) -> Result<(), String> {
    if DEV {
        logd!(1, "[Persistency] Putting key '{}'", key);
    }

    let client = get_client().await?;
    let mut client = client.lock().await;
    client.put(key, value).await.map_err(|e| format!("{}", e))
}

/// Get a value by key from the persistency service
pub async fn get(key: &str) -> Result<String, String> {
    if DEV {
        logd!(1, "[Persistency] Getting key '{}'", key);
    }

    let client = get_client().await?;
    let mut client = client.lock().await;
    client.get(key).await.map_err(|e| format!("{}", e))
}

/// Get all key-value pairs with the specified prefix
pub async fn get_all_with_prefix(prefix: &str) -> Result<Vec<(String, String)>, String> {
    if DEV {
        logd!(
            1,
            "[Persistency] Getting all keys with prefix '{}'",
            prefix
        );
    }

    let client = get_client().await?;
    let mut client = client.lock().await;

    let kv_pairs = client
        .get_all_with_prefix(prefix)
        .await
        .map_err(|e| format!("{}", e))?;

    let result: Vec<(String, String)> = kv_pairs
        .into_iter()
        .map(|kv| (kv.key, kv.value))
        .collect();

    if DEV {
        logd!(
            1,
            "[Persistency] Successfully retrieved {} keys with prefix '{}'",
            result.len(),
            prefix
        );
    }

    Ok(result)
}

/// Delete a key from the persistency service
pub async fn delete(key: &str) -> Result<(), String> {
    if DEV {
        logd!(1, "[Persistency] Deleting key '{}'", key);
    }

    let client = get_client().await?;
    let mut client = client.lock().await;
    client.delete(key).await.map_err(|e| format!("{}", e))
}

/// Batch put operation to store multiple key-value pairs
pub async fn batch_put(items: Vec<(String, String)>) -> Result<(), String> {
    if DEV {
        logd!(1, "[Persistency] Batch putting {} items", items.len());
    }

    let client = get_client().await?;
    let mut client = client.lock().await;
    client
        .batch_put(items)
        .await
        .map_err(|e| format!("{}", e))
}

/// Health check for the persistency service
pub async fn health_check() -> Result<bool, String> {
    if DEV {
        logd!(1, "[Persistency] Health check");
    }

    let client = get_client().await?;
    let mut client = client.lock().await;
    client.health_check().await.map_err(|e| format!("{}", e))
}

/// Delete all keys with a given prefix
pub async fn delete_all_with_prefix(prefix: &str) -> Result<(), String> {
    if DEV {
        logd!(
            1,
            "[Persistency] Deleting all keys with prefix '{}'",
            prefix
        );
    }

    let client = get_client().await?;
    let mut client = client.lock().await;
    client
        .delete_all_with_prefix(prefix)
        .await
        .map_err(|e| format!("{}", e))
}

// Keep the server configuration functions for compatibility
pub fn open_server() -> String {
    crate::persistency_proto::open_server()
}

//Unit Test Cases
#[cfg(test)]
mod tests {
    use super::*;

    // Test constants
    const TEST_KEY: &str = "unit_test_key";
    const TEST_VALUE: &str = "unit_test_value";
    const TEST_PREFIX: &str = "unit_test_";

    #[tokio::test]
    async fn test_put_and_get() {
        let result = put(TEST_KEY, TEST_VALUE).await;
        if result.is_ok() {
            let get_result = get(TEST_KEY).await;
            if let Ok(value) = get_result {
                assert_eq!(value, TEST_VALUE);
            }
        }
        let _ = delete(TEST_KEY).await;
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let result = get("nonexistent_key_12345").await;
        assert!(result.is_err(), "Expected error for nonexistent key");
    }

    #[tokio::test]
    async fn test_get_all_with_prefix() {
        let _ = put(&format!("{}key1", TEST_PREFIX), "value1").await;
        let _ = put(&format!("{}key2", TEST_PREFIX), "value2").await;
        let _ = put("other_key", "other_value").await;

        let result = get_all_with_prefix(TEST_PREFIX).await;
        if let Ok(kvs) = result {
            assert!(kvs.len() >= 2, "Expected at least 2 keys with prefix");
            for (key, _) in &kvs {
                assert!(key.starts_with(TEST_PREFIX), "Key should start with prefix");
            }
        }

        let _ = delete(&format!("{}key1", TEST_PREFIX)).await;
        let _ = delete(&format!("{}key2", TEST_PREFIX)).await;
        let _ = delete("other_key").await;
    }

    #[tokio::test]
    async fn test_delete() {
        let _ = put(TEST_KEY, TEST_VALUE).await;

        let delete_result = delete(TEST_KEY).await;
        if delete_result.is_ok() {
            let get_result = get(TEST_KEY).await;
            assert!(get_result.is_err(), "Key should not exist after deletion");
        }
    }

    #[tokio::test]
    async fn test_batch_put() {
        let items = vec![
            (format!("{}batch1", TEST_PREFIX), "bval1".to_string()),
            (format!("{}batch2", TEST_PREFIX), "bval2".to_string()),
        ];

        let result = batch_put(items).await;
        if result.is_ok() {
            let kvs = get_all_with_prefix(&format!("{}batch", TEST_PREFIX)).await;
            if let Ok(kvs) = kvs {
                assert!(kvs.len() >= 2, "Expected at least 2 batch-put keys");
            }
        }

        let _ = delete(&format!("{}batch1", TEST_PREFIX)).await;
        let _ = delete(&format!("{}batch2", TEST_PREFIX)).await;
    }

    #[tokio::test]
    async fn test_health_check() {
        let result = health_check().await;
        // Just check it doesn't panic; service may or may not be running
        let _ = result;
    }
}
use crate::core::types::{SerializableResourceState};
use common::statemanager::ResourceType;
use common::Result;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Get current resource state from etcd
pub async fn get_current_state(
    resource_key: &str,
) -> common::Result<Option<SerializableResourceState>> {
    match common::etcd::get(resource_key).await {
        Ok(serialized_data) => {
            match serde_yaml::from_str::<SerializableResourceState>(&serialized_data) {
                Ok(state) => Ok(Some(state)),
                Err(e) => {
                    eprintln!("Failed to deserialize state for {}: {}", resource_key, e);
                    Err(format!("Deserialization error: {}", e).into())
                }
            }
        }
        Err(e) => {
            // Check if it's a "Key not found" error (which is normal)
            let error_msg = format!("{:?}", e);
            if error_msg.contains("Key not found") {
                println!("Resource {} not found in etcd", resource_key);
                Ok(None)
            } else {
                eprintln!("Failed to get resource {} from etcd: {}", resource_key, e);
                Err(format!("etcd error: {}", e).into())
            }
        }
    }
}

/// Set current resource state in etcd
pub async fn set_current_state(
    resource_key: &str,
    state: &SerializableResourceState,
) -> common::Result<()> {
    let serialized =
        serde_yaml::to_string(state).map_err(|e| format!("Failed to serialize state: {}", e))?;

    common::etcd::put(resource_key, &serialized)
        .await
        .map_err(|e| format!("Failed to put state to etcd: {}", e))?;

    println!(
        "Successfully persisted state for resource: {}",
        resource_key
    );
    Ok(())
}

/// Delete resource state from etcd
pub async fn delete_current_state(resource_key: &str) -> common::Result<()> {
    common::etcd::delete(resource_key)
        .await
        .map_err(|e| format!("Failed to delete from etcd: {}", e))?;

    println!("Successfully deleted state for resource: {}", resource_key);
    Ok(())
}

/// Check if resource exists in etcd
pub async fn resource_exists(resource_key: &str) -> bool {
    match common::etcd::get(resource_key).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// List all resource keys with a specific prefix
pub async fn list_resources_with_prefix(prefix: &str) -> common::Result<Vec<String>> {
    match common::etcd::get_all_with_prefix(prefix).await {
        Ok(kvs) => {
            let keys: Vec<String> = kvs.into_iter().map(|kv| kv.key).collect();
            Ok(keys)
        }
        Err(e) => {
            eprintln!("Failed to list keys with prefix '{}': {}", prefix, e);
            Err(format!("Failed to list keys: {}", e).into())
        }
    }
}

/// Get all resource states from etcd
pub async fn get_all_resource_states() -> common::Result<Vec<(String, SerializableResourceState)>> {
    debug!("Retrieving all resource states from etcd");
    
    match common::etcd::get_all_with_prefix("state/").await {
        Ok(kvs) => {
            let mut states = Vec::new();
            
            for kv in kvs {
                match serde_yaml::from_str::<SerializableResourceState>(&kv.value) {
                    Ok(state) => {
                        states.push((kv.key, state));
                    }
                    Err(e) => {
                        error!("Failed to deserialize state for key {}: {}", kv.key, e);
                    }
                }
            }
            
            info!("Retrieved {} resource states from etcd", states.len());
            Ok(states)
        }
        Err(e) => {
            error!("Failed to retrieve states from etcd: {}", e);
            Err(format!("Failed to retrieve states from etcd: {}", e).into())
        }
    }
}

/// Get resource states filtered by type
pub async fn get_resource_states_by_type(
    resource_type: ResourceType,
) -> common::Result<Vec<(String, SerializableResourceState)>> {
    let prefix = format!("{:?}::", resource_type);
    let mut states = Vec::new();

    match list_resources_with_prefix(&prefix).await {
        Ok(keys) => {
            for key in keys {
                match get_current_state(&key).await {
                    Ok(Some(state)) => {
                        states.push((key, state));
                    }
                    Ok(None) => {
                        println!("Warning: Key {} exists but has no valid state", key);
                    }
                    Err(e) => {
                        eprintln!("Failed to get state for key {}: {}", key, e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to list keys with prefix '{}': {}", prefix, e);
            return Err(e);
        }
    }

    Ok(states)
}

/// Clean up invalid or corrupted state entries
pub async fn cleanup_invalid_states() -> common::Result<u32> {
    let mut cleaned_count = 0;

    match get_all_resource_states().await {
        Ok(states) => {
            for (key, state) in states {
                // Validate state integrity
                if !is_valid_resource_state(&state) {
                    println!("Cleaning up invalid state for key: {}", key);
                    if let Err(e) = delete_current_state(&key).await {
                        eprintln!("Failed to delete invalid state {}: {}", key, e);
                    } else {
                        cleaned_count += 1;
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to get all states for cleanup: {}", e);
            return Err(e);
        }
    }

    println!("Cleaned up {} invalid state entries", cleaned_count);
    Ok(cleaned_count)
}

/// Validate if a resource state is valid
fn is_valid_resource_state(state: &SerializableResourceState) -> bool {
    // Basic validation checks
    if state.resource_name.trim().is_empty() {
        return false;
    }

    // Check if resource type is valid
    if ResourceType::try_from(state.resource_type).is_err() {
        return false;
    }

    // Check if current_state is reasonable (not negative, within bounds)
    if state.current_state.trim().is_empty() {
        return false;
    }

    // Check if timestamps are reasonable (not in the future by more than 1 hour)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if state.last_transition_unix_timestamp > now + 3600 {
        return false;
    }

    if state.health_status.last_check_unix_timestamp > now + 3600 {
        return false;
    }

    true
}

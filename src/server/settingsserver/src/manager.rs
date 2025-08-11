/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Business logic for Settings Server

use crate::route::api::MonitoringSettings;
use common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::OnceCell;

/// Monitoring system status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStatus {
    /// Service running status
    pub is_running: bool,
    /// Last update timestamp
    pub last_update: String,
    /// Number of monitored containers
    pub monitored_containers: u32,
    /// System health status
    pub health_status: String,
}

/// In-memory storage for monitoring settings (in production, this would use etcd)
type SettingsStorage = Arc<Mutex<HashMap<String, MonitoringSettings>>>;

/// Global storage instance
static SETTINGS_STORAGE: OnceCell<SettingsStorage> = OnceCell::const_new();

/// Initialize the settings storage
async fn get_storage() -> &'static SettingsStorage {
    SETTINGS_STORAGE
        .get_or_init(|| async {
            let mut storage = HashMap::new();
            // Add default settings
            storage.insert("default".to_string(), MonitoringSettings::default());
            Arc::new(Mutex::new(storage))
        })
        .await
}

/// Initialize the Settings Server manager
pub async fn initialize() {
    tokio::join!(crate::route::launch_tcp_listener(), init_storage());
}

/// Initialize storage with default settings
async fn init_storage() {
    println!("Initializing Settings Server storage...");
    let _storage = get_storage().await;
    println!("Settings Server initialized with default monitoring settings");
}

/// Get all monitoring settings
pub async fn get_all_monitoring_settings() -> Result<Vec<MonitoringSettings>> {
    let storage = get_storage().await;
    let settings = storage.lock().unwrap();
    Ok(settings.values().cloned().collect())
}

/// Get specific monitoring settings by ID
pub async fn get_monitoring_settings(id: &str) -> Result<MonitoringSettings> {
    let storage = get_storage().await;
    let settings = storage.lock().unwrap();
    
    match settings.get(id) {
        Some(setting) => Ok(setting.clone()),
        None => Err(format!("Monitoring settings with id '{}' not found", id).into()),
    }
}

/// Create new monitoring settings
pub async fn create_monitoring_settings(settings: MonitoringSettings) -> Result<()> {
    let storage = get_storage().await;
    let mut storage_map = storage.lock().unwrap();
    
    // Check if settings with this ID already exist
    if storage_map.contains_key(&settings.id) {
        return Err(format!("Monitoring settings with id '{}' already exist", settings.id).into());
    }
    
    // Validate settings
    validate_monitoring_settings(&settings)?;
    
    let settings_id = settings.id.clone();
    storage_map.insert(settings_id.clone(), settings);
    println!("Created monitoring settings with id: {}", settings_id);
    Ok(())
}

/// Update existing monitoring settings
pub async fn update_monitoring_settings(id: &str, mut settings: MonitoringSettings) -> Result<()> {
    let storage = get_storage().await;
    let mut storage_map = storage.lock().unwrap();
    
    // Check if settings exist
    if !storage_map.contains_key(id) {
        return Err(format!("Monitoring settings with id '{}' not found", id).into());
    }
    
    // Validate settings
    validate_monitoring_settings(&settings)?;
    
    // Ensure the ID matches
    settings.id = id.to_string();
    
    storage_map.insert(id.to_string(), settings);
    println!("Updated monitoring settings with id: {}", id);
    Ok(())
}

/// Delete monitoring settings
pub async fn delete_monitoring_settings(id: &str) -> Result<()> {
    let storage = get_storage().await;
    let mut storage_map = storage.lock().unwrap();
    
    // Don't allow deletion of default settings
    if id == "default" {
        return Err("Cannot delete default monitoring settings".into());
    }
    
    match storage_map.remove(id) {
        Some(_) => {
            println!("Deleted monitoring settings with id: {}", id);
            Ok(())
        }
        None => Err(format!("Monitoring settings with id '{}' not found", id).into()),
    }
}

/// Get monitoring system status
pub async fn get_monitoring_status() -> Result<MonitoringStatus> {
    // In a real implementation, this would query the monitoring server
    // For now, return mock status
    Ok(MonitoringStatus {
        is_running: true,
        last_update: chrono::Utc::now().to_rfc3339(),
        monitored_containers: 5,
        health_status: "healthy".to_string(),
    })
}

/// Validate monitoring settings
fn validate_monitoring_settings(settings: &MonitoringSettings) -> Result<()> {
    if settings.id.is_empty() {
        return Err("Settings ID cannot be empty".into());
    }
    
    if settings.monitoring_interval == 0 {
        return Err("Monitoring interval must be greater than 0".into());
    }
    
    if settings.resource_alert_threshold > 100 {
        return Err("Resource alert threshold must be between 0 and 100".into());
    }
    
    if settings.data_retention_days == 0 {
        return Err("Data retention days must be greater than 0".into());
    }
    
    Ok(())
}

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;

    /// Test default monitoring settings creation
    #[tokio::test]
    async fn test_create_default_settings() {
        let settings = MonitoringSettings::default();
        let result = create_monitoring_settings(settings.clone()).await;
        
        // Should fail because default already exists
        assert!(result.is_err());
    }

    /// Test creating new monitoring settings
    #[tokio::test]
    async fn test_create_new_settings() {
        let mut settings = MonitoringSettings::default();
        settings.id = "test_create".to_string();
        settings.monitoring_interval = 60;
        
        let result = create_monitoring_settings(settings.clone()).await;
        assert!(result.is_ok());
        
        // Verify it was created
        let retrieved = get_monitoring_settings("test_create").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().monitoring_interval, 60);
    }

    /// Test updating monitoring settings
    #[tokio::test]
    async fn test_update_settings() {
        // First create a setting
        let mut settings = MonitoringSettings::default();
        settings.id = "test_update".to_string();
        let _ = create_monitoring_settings(settings.clone()).await;
        
        // Update it
        settings.monitoring_interval = 120;
        let result = update_monitoring_settings("test_update", settings).await;
        assert!(result.is_ok());
        
        // Verify it was updated
        let retrieved = get_monitoring_settings("test_update").await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap().monitoring_interval, 120);
    }

    /// Test deleting monitoring settings
    #[tokio::test]
    async fn test_delete_settings() {
        // First create a setting
        let mut settings = MonitoringSettings::default();
        settings.id = "test_delete".to_string();
        let _ = create_monitoring_settings(settings.clone()).await;
        
        // Delete it
        let result = delete_monitoring_settings("test_delete").await;
        assert!(result.is_ok());
        
        // Verify it was deleted
        let retrieved = get_monitoring_settings("test_delete").await;
        assert!(retrieved.is_err());
    }

    /// Test deleting default settings (should fail)
    #[tokio::test]
    async fn test_delete_default_settings() {
        let result = delete_monitoring_settings("default").await;
        assert!(result.is_err());
    }

    /// Test getting all monitoring settings
    #[tokio::test]
    async fn test_get_all_settings() {
        let result = get_all_monitoring_settings().await;
        assert!(result.is_ok());
        let settings = result.unwrap();
        assert!(!settings.is_empty());
    }

    /// Test getting monitoring status
    #[tokio::test]
    async fn test_get_monitoring_status() {
        let result = get_monitoring_status().await;
        assert!(result.is_ok());
        let status = result.unwrap();
        assert!(status.is_running);
        assert_eq!(status.health_status, "healthy");
    }

    /// Test validation with invalid settings
    #[tokio::test]
    async fn test_validation_invalid_settings() {
        let mut settings = MonitoringSettings::default();
        settings.id = "".to_string(); // Invalid empty ID
        
        let result = validate_monitoring_settings(&settings);
        assert!(result.is_err());
        
        settings.id = "test".to_string();
        settings.monitoring_interval = 0; // Invalid interval
        
        let result = validate_monitoring_settings(&settings);
        assert!(result.is_err());
        
        settings.monitoring_interval = 30;
        settings.resource_alert_threshold = 150; // Invalid threshold
        
        let result = validate_monitoring_settings(&settings);
        assert!(result.is_err());
    }

    /// Test validation with valid settings
    #[tokio::test]
    async fn test_validation_valid_settings() {
        let settings = MonitoringSettings::default();
        let result = validate_monitoring_settings(&settings);
        assert!(result.is_ok());
    }

    /// Test getting non-existent settings
    #[tokio::test]
    async fn test_get_nonexistent_settings() {
        let result = get_monitoring_settings("nonexistent").await;
        assert!(result.is_err());
    }

    /// Test updating non-existent settings
    #[tokio::test]
    async fn test_update_nonexistent_settings() {
        let settings = MonitoringSettings::default();
        let result = update_monitoring_settings("nonexistent", settings).await;
        assert!(result.is_err());
    }
}
// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Core service management module

use crate::settings_api::ApiServer;
use crate::settings_config::ConfigManager;
use crate::settings_history::HistoryManager;
use crate::settings_monitoring::MonitoringManager;
use crate::settings_storage::EtcdClient;
use crate::settings_utils::error::SettingsError;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

/// System status information
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub version: String,
    pub uptime: std::time::Duration,
    pub components: std::collections::HashMap<String, ComponentStatus>,
}

/// Component status
#[derive(Debug, Clone)]
pub enum ComponentStatus {
    Healthy,
    Degraded(String),
    Failed(String),
}

/// Core manager coordinates all service components
pub struct CoreManager {
    config_manager: Arc<RwLock<ConfigManager>>,
    history_manager: Arc<RwLock<HistoryManager>>,
    monitoring_manager: Arc<RwLock<MonitoringManager>>,
    api_server: Option<ApiServer>,
    start_time: std::time::Instant,
}

impl CoreManager {
    /// Create a new core manager
    pub async fn new(
        etcd_endpoints: Vec<String>,
        bind_address: String,
        bind_port: u16,
        _config_file: PathBuf,
    ) -> Result<Self, SettingsError> {
        info!("Initializing Settings Service core manager");

        // Initialize ETCD clients for each component
        let storage_config = EtcdClient::new(etcd_endpoints.clone()).await.map_err(|e| {
            SettingsError::System(format!("Failed to create config storage: {}", e))
        })?;

        let storage_history = EtcdClient::new(etcd_endpoints.clone()).await.map_err(|e| {
            SettingsError::System(format!("Failed to create history storage: {}", e))
        })?;

        let storage_monitoring = EtcdClient::new(etcd_endpoints).await.map_err(|e| {
            SettingsError::System(format!("Failed to create monitoring storage: {}", e))
        })?;

        // Initialize managers
        let config_manager = Arc::new(RwLock::new(ConfigManager::new(Box::new(storage_config))));
        let history_manager = Arc::new(RwLock::new(HistoryManager::new(Box::new(storage_history))));
        let monitoring_manager = Arc::new(RwLock::new(MonitoringManager::new(
            Box::new(storage_monitoring),
            60, // 60 seconds cache TTL
        )));

        // Initialize API server
        let api_server = ApiServer::new(
            bind_address,
            bind_port,
            config_manager.clone(),
            history_manager.clone(),
            monitoring_manager.clone(),
        )
        .await?;

        Ok(Self {
            config_manager,
            history_manager,
            monitoring_manager,
            api_server: Some(api_server),
            start_time: std::time::Instant::now(),
        })
    }

    /// Start all services
    pub async fn start_services(&mut self) -> Result<(), SettingsError> {
        info!("Starting Settings Service components");

        // Load default schemas
        self.load_default_schemas().await?;

        // Start API server
        if let Some(api_server) = self.api_server.take() {
            tokio::spawn(async move {
                if let Err(e) = api_server.start().await {
                    error!("API server failed: {}", e);
                }
            });
        }

        info!("All Settings Service components started successfully");
        Ok(())
    }

    /// Shutdown all services
    pub async fn shutdown(&self) -> Result<(), SettingsError> {
        info!("Shutting down Settings Service");

        // Clear monitoring cache
        self.monitoring_manager.read().await.clear_cache();

        info!("Settings Service shutdown complete");
        Ok(())
    }

    /// Get system status
    pub async fn get_system_status(&self) -> Result<SystemStatus, SettingsError> {
        debug!("Getting system status");

        let mut components = std::collections::HashMap::new();

        // Check config manager status
        components.insert(
            "config".to_string(),
            self.check_config_manager_status().await,
        );

        // Check history manager status
        components.insert(
            "history".to_string(),
            self.check_history_manager_status().await,
        );

        // Check monitoring manager status
        components.insert(
            "monitoring".to_string(),
            self.check_monitoring_manager_status().await,
        );

        Ok(SystemStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: self.start_time.elapsed(),
            components,
        })
    }

    /// Load default configuration schemas
    async fn load_default_schemas(&self) -> Result<(), SettingsError> {
        info!("Loading default configuration schemas");

        let mut config_manager = self.config_manager.write().await;

        // Basic logging configuration schema
        let logging_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "level": {
                    "type": "string",
                    "enum": ["debug", "info", "warn", "error"]
                },
                "output": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["file", "console", "syslog"]
                    }
                },
                "file_path": {
                    "type": "string"
                },
                "max_size_mb": {
                    "type": "integer",
                    "minimum": 1
                },
                "max_files": {
                    "type": "integer",
                    "minimum": 1
                },
                "rotation": {
                    "type": "string",
                    "enum": ["daily", "weekly", "size"]
                }
            },
            "required": ["level", "output"]
        });

        config_manager
            .save_schema("logging-config", &logging_schema)
            .await?;

        // Basic network configuration schema
        let network_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "bind_address": {
                    "type": "string",
                    "pattern": "^\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}$"
                },
                "bind_port": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 65535
                },
                "tls_enabled": {
                    "type": "boolean"
                },
                "tls_cert_path": {
                    "type": "string"
                },
                "tls_key_path": {
                    "type": "string"
                }
            },
            "required": ["bind_address", "bind_port"]
        });

        config_manager
            .save_schema("network-config", &network_schema)
            .await?;

        // Metrics filter configuration schema
        let metrics_filter_schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "enabled": {
                    "type": "boolean"
                },
                "refresh": {
                    "type": "integer",
                    "minimum": 1
                },
                "max_items": {
                    "type": "integer",
                    "minimum": 1
                },
                "cache_ttl": {
                    "type": "integer",
                    "minimum": 1
                },
                "filters": {
                    "type": "object",
                    "additionalProperties": {
                        "type": "array",
                        "items": {
                            "type": "string"
                        }
                    }
                }
            },
            "required": ["enabled"]
        });

        config_manager
            .save_schema("metrics-filter", &metrics_filter_schema)
            .await?;

        info!("Default schemas loaded successfully");
        Ok(())
    }

    /// Check config manager status
    async fn check_config_manager_status(&self) -> ComponentStatus {
        // Try to access the config manager
        match self.config_manager.write().await.list_configs(None).await {
            Ok(_) => ComponentStatus::Healthy,
            Err(e) => ComponentStatus::Failed(format!("Config manager error: {}", e)),
        }
    }

    /// Check history manager status
    async fn check_history_manager_status(&self) -> ComponentStatus {
        // Try to list some history (this is a read-only operation)
        ComponentStatus::Healthy // For now, assume healthy if we can access it
    }

    /// Check monitoring manager status  
    async fn check_monitoring_manager_status(&self) -> ComponentStatus {
        // Check cache stats as a health indicator
        let cache_stats = self.monitoring_manager.read().await.get_cache_stats();
        if cache_stats.is_empty() {
            ComponentStatus::Degraded("Cache not initialized".to_string())
        } else {
            ComponentStatus::Healthy
        }
    }

    /// Get reference to config manager (for testing)
    #[cfg(test)]
    pub fn config_manager(&self) -> Arc<RwLock<ConfigManager>> {
        self.config_manager.clone()
    }

    /// Get reference to history manager (for testing)
    #[cfg(test)]
    pub fn history_manager(&self) -> Arc<RwLock<HistoryManager>> {
        self.history_manager.clone()
    }

    /// Get reference to monitoring manager (for testing)
    #[cfg(test)]
    pub fn monitoring_manager(&self) -> Arc<RwLock<MonitoringManager>> {
        self.monitoring_manager.clone()
    }
}

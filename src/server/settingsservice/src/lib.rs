// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! PICCOLO Settings Service Library
//!
//! This library provides centralized configuration management and metrics filtering
//! for the PICCOLO framework.

pub mod monitoring_etcd;
pub mod monitoring_types;
pub mod settings_api;
pub mod settings_cli;
pub mod settings_config;
pub mod settings_core;
pub mod settings_history;
pub mod settings_monitoring;
pub mod settings_storage;
pub mod settings_utils;
pub use settings_core::CoreManager;
pub use settings_utils::error::{SettingsError, StorageError};

/// Re-export commonly used types
pub use settings_config::{Config, ConfigManager};
pub use settings_history::{HistoryEntry, HistoryManager};
pub use settings_monitoring::MonitoringManager;
pub use settings_storage::Storage;

/// Re-export monitoring types for external use
pub use monitoring_types::{BoardInfo, NodeInfo, SocInfo};

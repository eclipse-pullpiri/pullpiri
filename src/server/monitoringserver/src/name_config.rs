/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Name configuration for Board and SoC monitoring information.
//!
//! This module loads a YAML configuration file that maps IP-based Board/SoC IDs
//! to human-readable names. The default configuration path is
//! `/etc/piccolo/monitoring_names.yaml`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default path for the monitoring name configuration file
pub const DEFAULT_CONFIG_PATH: &str = "/etc/piccolo/monitoring_names.yaml";

/// Name entry for a single Board or SoC
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NameEntry {
    /// Human-readable name for this Board or SoC
    pub name: String,
}

/// Top-level name configuration loaded from YAML
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MonitoringNamesConfig {
    /// Maps board_id (IP-based) to name entry
    #[serde(default)]
    pub boards: HashMap<String, NameEntry>,
    /// Maps soc_id (IP-based) to name entry
    #[serde(default)]
    pub socs: HashMap<String, NameEntry>,
}

impl MonitoringNamesConfig {
    /// Loads configuration from the given YAML file path.
    ///
    /// Returns an empty configuration if the file does not exist or cannot be parsed,
    /// so that the service can still run without a name config.
    pub fn load_from_file(path: &str) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_yaml::from_str::<Self>(&content) {
                Ok(config) => {
                    eprintln!(
                        "[NameConfig] Loaded monitoring names config from '{}'",
                        path
                    );
                    config
                }
                Err(e) => {
                    eprintln!(
                        "[NameConfig] Failed to parse '{}': {}. Using empty config.",
                        path, e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!(
                    "[NameConfig] Could not read '{}': {}. Using empty config.",
                    path, e
                );
                Self::default()
            }
        }
    }

    /// Looks up the name for a given soc_id. Returns an empty string if not configured.
    pub fn soc_name(&self, soc_id: &str) -> String {
        self.socs
            .get(soc_id)
            .map(|e| e.name.clone())
            .unwrap_or_default()
    }

    /// Looks up the name for a given board_id. Returns an empty string if not configured.
    pub fn board_name(&self, board_id: &str) -> String {
        self.boards
            .get(board_id)
            .map(|e| e.name.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_empty() {
        let config = MonitoringNamesConfig::default();
        assert!(config.boards.is_empty());
        assert!(config.socs.is_empty());
    }

    #[test]
    fn test_soc_name_lookup_missing_returns_empty() {
        let config = MonitoringNamesConfig::default();
        assert_eq!(config.soc_name("192.168.10.200"), "");
    }

    #[test]
    fn test_board_name_lookup_missing_returns_empty() {
        let config = MonitoringNamesConfig::default();
        assert_eq!(config.board_name("192.168.10.0"), "");
    }

    #[test]
    fn test_soc_name_lookup_found() {
        let mut config = MonitoringNamesConfig::default();
        config.socs.insert(
            "192.168.10.200".to_string(),
            NameEntry {
                name: "Alpha SoC".to_string(),
            },
        );
        assert_eq!(config.soc_name("192.168.10.200"), "Alpha SoC");
    }

    #[test]
    fn test_board_name_lookup_found() {
        let mut config = MonitoringNamesConfig::default();
        config.boards.insert(
            "192.168.10.0".to_string(),
            NameEntry {
                name: "Main Board".to_string(),
            },
        );
        assert_eq!(config.board_name("192.168.10.0"), "Main Board");
    }

    #[test]
    fn test_load_from_nonexistent_file_returns_default() {
        let config = MonitoringNamesConfig::load_from_file("/nonexistent/path/config.yaml");
        assert!(config.boards.is_empty());
        assert!(config.socs.is_empty());
    }

    #[test]
    fn test_deserialize_from_yaml() {
        let yaml = r#"
boards:
  "192.168.10.0":
    name: "Main Board"
  "192.168.11.0":
    name: "Rear Board"
socs:
  "192.168.10.200":
    name: "Alpha SoC"
  "192.168.10.210":
    name: "Beta SoC"
"#;
        let config: MonitoringNamesConfig =
            serde_yaml::from_str(yaml).expect("Failed to parse YAML");

        assert_eq!(config.boards.len(), 2);
        assert_eq!(config.socs.len(), 2);
        assert_eq!(config.board_name("192.168.10.0"), "Main Board");
        assert_eq!(config.board_name("192.168.11.0"), "Rear Board");
        assert_eq!(config.soc_name("192.168.10.200"), "Alpha SoC");
        assert_eq!(config.soc_name("192.168.10.210"), "Beta SoC");
    }

    #[test]
    fn test_deserialize_partial_yaml() {
        let yaml = r#"
boards:
  "10.0.0.0":
    name: "Only Board"
"#;
        let config: MonitoringNamesConfig =
            serde_yaml::from_str(yaml).expect("Failed to parse partial YAML");
        assert_eq!(config.board_name("10.0.0.0"), "Only Board");
        assert_eq!(config.soc_name("10.0.0.0"), "");
    }

    #[test]
    fn test_load_from_file_with_valid_content() {
        use std::io::Write;

        let mut temp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
        let yaml = r#"
boards:
  "192.168.1.0":
    name: "Test Board"
socs:
  "192.168.1.100":
    name: "Test SoC"
"#;
        temp.write_all(yaml.as_bytes())
            .expect("Failed to write temp file");

        let config =
            MonitoringNamesConfig::load_from_file(temp.path().to_str().expect("Invalid path"));
        assert_eq!(config.board_name("192.168.1.0"), "Test Board");
        assert_eq!(config.soc_name("192.168.1.100"), "Test SoC");
    }
}

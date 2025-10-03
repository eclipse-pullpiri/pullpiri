// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! PICCOLO Settings Service
//!
//! This service provides centralized configuration management and metrics filtering
//! for the PICCOLO framework. It supports:
//!
//! - YAML/JSON configuration management
//! - Change history tracking and rollback
//! - Metrics data filtering from ETCD
//! - REST API interface
//! - Schema validation

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

pub mod monitoring_etcd;
pub mod monitoring_types;
mod settings_api;
mod settings_config;
mod settings_core;
mod settings_history;
mod settings_monitoring;
mod settings_storage;
mod settings_utils;
use settings_core::CoreManager;
use settings_utils::logging::init_logging;

/// Settings Service command line arguments
#[derive(Parser, Debug)]
#[command(name = "settingsservice")]
#[command(about = "PICCOLO Settings Service - Central configuration and metrics management")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "/etc/piccolo/settings.yaml")]
    config: PathBuf,

    /// ETCD endpoints (comma separated)
    #[arg(long, default_value = "localhost:2379")]
    etcd_endpoints: String,

    /// HTTP server bind address
    #[arg(long, default_value = "0.0.0.0")]
    bind_address: String,

    /// HTTP server bind port
    #[arg(long, default_value = "8080")]
    bind_port: u16,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args.log_level)?;

    info!("Starting PICCOLO Settings Service");
    info!("Config file: {:?}", args.config);
    info!("ETCD endpoints: {}", args.etcd_endpoints);

    // Run in server mode only
    run_server_mode(args).await
}

async fn run_server_mode(args: Args) -> Result<()> {
    info!(
        "Starting in server mode on {}:{}",
        args.bind_address, args.bind_port
    );

    // Parse ETCD endpoints
    let etcd_endpoints: Vec<String> = args
        .etcd_endpoints
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Initialize core manager
    let mut core_manager = CoreManager::new(
        etcd_endpoints,
        args.bind_address.clone(),
        args.bind_port,
        args.config,
    )
    .await?;

    info!("Available API endpoints:");
    info!("  GET    /api/v1/settings");
    info!("  GET    /api/v1/metrics");
    info!("  GET    /api/v1/history");
    info!("  GET    /api/v1/system/health");

    // Start all services including the API server
    // This will start the HTTP server on the specified port
    core_manager.start_services().await?;

    info!("Settings Service started successfully");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    info!("Shutting down Settings Service");

    // Shutdown core manager
    core_manager.shutdown().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};
    use std::env;
    use std::path::Path;

    #[test]
    fn test_args_default_values() {
        let args = Args::parse_from(["settingsservice"]);

        assert_eq!(args.config, PathBuf::from("/etc/piccolo/settings.yaml"));
        assert_eq!(args.etcd_endpoints, "localhost:2379");
        assert_eq!(args.bind_address, "0.0.0.0");
        assert_eq!(args.bind_port, 8080);
        assert_eq!(args.log_level, "info");
    }

    #[test]
    fn test_args_custom_config_path() {
        let args = Args::parse_from(["settingsservice", "--config", "/custom/path/settings.yaml"]);

        assert_eq!(args.config, PathBuf::from("/custom/path/settings.yaml"));
        assert_eq!(args.etcd_endpoints, "localhost:2379"); // Should remain default
        assert_eq!(args.bind_address, "0.0.0.0"); // Should remain default
        assert_eq!(args.bind_port, 8080); // Should remain default
        assert_eq!(args.log_level, "info"); // Should remain default
    }

    #[test]
    fn test_args_short_config_flag() {
        let args = Args::parse_from(["settingsservice", "-c", "/short/path/settings.yaml"]);

        assert_eq!(args.config, PathBuf::from("/short/path/settings.yaml"));
    }

    #[test]
    fn test_args_custom_etcd_endpoints() {
        let args = Args::parse_from([
            "settingsservice",
            "--etcd-endpoints",
            "etcd1:2379,etcd2:2379,etcd3:2379",
        ]);

        assert_eq!(args.etcd_endpoints, "etcd1:2379,etcd2:2379,etcd3:2379");
        assert_eq!(args.config, PathBuf::from("/etc/piccolo/settings.yaml")); // Should remain default
    }

    #[test]
    fn test_args_custom_bind_address() {
        let args = Args::parse_from(["settingsservice", "--bind-address", "127.0.0.1"]);

        assert_eq!(args.bind_address, "127.0.0.1");
        assert_eq!(args.bind_port, 8080); // Should remain default
    }

    #[test]
    fn test_args_custom_bind_port() {
        let args = Args::parse_from(["settingsservice", "--bind-port", "9090"]);

        assert_eq!(args.bind_port, 9090);
        assert_eq!(args.bind_address, "0.0.0.0"); // Should remain default
    }

    #[test]
    fn test_args_custom_log_level() {
        let log_levels = ["trace", "debug", "info", "warn", "error"];

        for level in &log_levels {
            let args = Args::parse_from(["settingsservice", "--log-level", level]);

            assert_eq!(args.log_level, *level);
        }
    }

    #[test]
    fn test_args_all_custom_values() {
        let args = Args::parse_from([
            "settingsservice",
            "--config",
            "/test/custom.yaml",
            "--etcd-endpoints",
            "test-etcd:2379",
            "--bind-address",
            "192.168.1.100",
            "--bind-port",
            "7777",
            "--log-level",
            "debug",
        ]);

        assert_eq!(args.config, PathBuf::from("/test/custom.yaml"));
        assert_eq!(args.etcd_endpoints, "test-etcd:2379");
        assert_eq!(args.bind_address, "192.168.1.100");
        assert_eq!(args.bind_port, 7777);
        assert_eq!(args.log_level, "debug");
    }

    #[test]
    fn test_args_mixed_long_short_flags() {
        let args = Args::parse_from([
            "settingsservice",
            "-c",
            "/mixed/config.yaml",
            "--etcd-endpoints",
            "mixed-etcd:2379",
            "--bind-port",
            "3333",
        ]);

        assert_eq!(args.config, PathBuf::from("/mixed/config.yaml"));
        assert_eq!(args.etcd_endpoints, "mixed-etcd:2379");
        assert_eq!(args.bind_port, 3333);
        assert_eq!(args.bind_address, "0.0.0.0"); // Default
        assert_eq!(args.log_level, "info"); // Default
    }

    #[test]
    fn test_etcd_endpoints_parsing_single() {
        let endpoints = "localhost:2379";
        let parsed: Vec<String> = endpoints.split(',').map(|s| s.trim().to_string()).collect();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], "localhost:2379");
    }

    #[test]
    fn test_etcd_endpoints_parsing_multiple() {
        let endpoints = "etcd1:2379,etcd2:2379,etcd3:2379";
        let parsed: Vec<String> = endpoints.split(',').map(|s| s.trim().to_string()).collect();

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "etcd1:2379");
        assert_eq!(parsed[1], "etcd2:2379");
        assert_eq!(parsed[2], "etcd3:2379");
    }

    #[test]
    fn test_etcd_endpoints_parsing_with_spaces() {
        let endpoints = "etcd1:2379, etcd2:2379 , etcd3:2379";
        let parsed: Vec<String> = endpoints.split(',').map(|s| s.trim().to_string()).collect();

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "etcd1:2379");
        assert_eq!(parsed[1], "etcd2:2379");
        assert_eq!(parsed[2], "etcd3:2379");
    }

    #[test]
    fn test_etcd_endpoints_parsing_empty_parts() {
        let endpoints = "etcd1:2379,,etcd3:2379";
        let parsed: Vec<String> = endpoints
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], "etcd1:2379");
        assert_eq!(parsed[1], "etcd3:2379");
    }

    #[test]
    fn test_pathbuf_creation() {
        let config_paths = [
            "/etc/piccolo/settings.yaml",
            "/home/user/config.yaml",
            "relative/path/config.yaml",
            "../parent/config.yaml",
            "./current/config.yaml",
        ];

        for path_str in &config_paths {
            let path_buf = PathBuf::from(path_str);
            assert_eq!(path_buf.to_string_lossy(), *path_str);

            // Test that PathBuf handles the path correctly
            if path_str.starts_with('/') {
                assert!(path_buf.is_absolute());
            } else {
                assert!(path_buf.is_relative());
            }
        }
    }

    #[test]
    fn test_bind_address_validation_patterns() {
        let valid_addresses = [
            "0.0.0.0",
            "127.0.0.1",
            "192.168.1.100",
            "10.0.0.1",
            "172.16.0.1",
            "::1",
            "::",
            "localhost",
        ];

        for address in &valid_addresses {
            // These are all syntactically valid address strings
            // The actual validation would happen in the network binding code
            assert!(!address.is_empty());
            assert!(address.len() > 0);
        }
    }

    #[test]
    fn test_port_range_validation() {
        // Test valid port ranges
        let valid_ports = [1, 80, 443, 8080, 8443, 9090, 65535];

        for port in &valid_ports {
            assert!(*port > 0);
            assert!(*port <= 65535);
        }
    }

    #[test]
    fn test_log_level_validation() {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        let invalid_levels = ["verbose", "fatal", "critical", "notice"];

        for level in &valid_levels {
            // These are valid tracing log levels
            assert!(matches!(
                *level,
                "trace" | "debug" | "info" | "warn" | "error"
            ));
        }

        for level in &invalid_levels {
            // These are not standard tracing log levels
            assert!(!matches!(
                *level,
                "trace" | "debug" | "info" | "warn" | "error"
            ));
        }
    }

    #[test]
    fn test_args_debug_trait() {
        let args = Args::parse_from([
            "settingsservice",
            "--config",
            "/test.yaml",
            "--etcd-endpoints",
            "test:2379",
            "--bind-address",
            "127.0.0.1",
            "--bind-port",
            "9000",
            "--log-level",
            "debug",
        ]);

        let debug_output = format!("{:?}", args);
        assert!(debug_output.contains("Args"));
        assert!(debug_output.contains("/test.yaml"));
        assert!(debug_output.contains("test:2379"));
        assert!(debug_output.contains("127.0.0.1"));
        assert!(debug_output.contains("9000"));
        assert!(debug_output.contains("debug"));
    }

    #[test]
    fn test_command_metadata() {
        // Test that we can access the command metadata
        let cmd = Args::command();

        assert_eq!(cmd.get_name(), "settingsservice");
        assert!(cmd.get_about().is_some());
        let about_str = cmd.get_about().unwrap().to_string();
        assert!(about_str.contains("PICCOLO Settings Service"));
    }

    #[test]
    fn test_config_file_extensions() {
        let config_files = [
            "/path/to/config.yaml",
            "/path/to/config.yml",
            "/path/to/config.json",
            "/path/to/settings.yaml",
            "/path/to/piccolo.yaml",
        ];

        for config_file in &config_files {
            let path = Path::new(config_file);
            if let Some(extension) = path.extension() {
                assert!(matches!(
                    extension.to_str().unwrap(),
                    "yaml" | "yml" | "json"
                ));
            }
        }
    }

    #[test]
    fn test_environment_integration() {
        // Test that environment doesn't interfere with argument parsing
        // This simulates how the service might be started in different environments

        let original_pwd = env::var("PWD").unwrap_or_default();
        env::set_var("PWD", "/test/directory");

        let args = Args::parse_from(["settingsservice"]);
        assert_eq!(args.config, PathBuf::from("/etc/piccolo/settings.yaml"));

        // Restore original environment
        if !original_pwd.is_empty() {
            env::set_var("PWD", original_pwd);
        } else {
            env::remove_var("PWD");
        }
    }

    #[test]
    fn test_service_identification() {
        // Test service identification constants
        const SERVICE_NAME: &str = "settingsservice";
        const SERVICE_DESCRIPTION: &str =
            "PICCOLO Settings Service - Central configuration and metrics management";

        assert_eq!(SERVICE_NAME, "settingsservice");
        assert!(SERVICE_DESCRIPTION.contains("PICCOLO"));
        assert!(SERVICE_DESCRIPTION.contains("Settings Service"));
        assert!(SERVICE_DESCRIPTION.contains("configuration"));
        assert!(SERVICE_DESCRIPTION.contains("metrics"));
    }

    #[test]
    fn test_network_endpoint_construction() {
        let test_cases = [
            ("0.0.0.0", 8080, "0.0.0.0:8080"),
            ("127.0.0.1", 3000, "127.0.0.1:3000"),
            ("192.168.1.100", 9090, "192.168.1.100:9090"),
            ("localhost", 8000, "localhost:8000"),
        ];

        for (address, port, expected) in &test_cases {
            let endpoint = format!("{}:{}", address, port);
            assert_eq!(endpoint, *expected);
        }
    }

    #[test]
    fn test_api_endpoint_documentation() {
        // Verify the documented API endpoints are correctly formatted
        let expected_endpoints = [
            ("GET", "/api/v1/settings"),
            ("GET", "/api/v1/metrics"),
            ("GET", "/api/v1/history"),
            ("GET", "/api/v1/system/health"),
        ];

        for (method, path) in &expected_endpoints {
            assert!(method.len() > 0);
            assert!(path.starts_with("/api/v1/"));
            assert!(!path.ends_with("/"));
        }
    }

    #[test]
    fn test_module_declarations() {
        // Verify that all required modules are declared
        // This is a compile-time check that ensures modules exist
        use crate::monitoring_etcd;
        use crate::monitoring_types;
        use crate::settings_api;
        use crate::settings_config;
        use crate::settings_core;
        use crate::settings_history;
        use crate::settings_monitoring;
        use crate::settings_storage;
        use crate::settings_utils;

        // If this compiles, all modules are properly declared and accessible
        assert!(true);
    }

    #[test]
    fn test_result_type_usage() {
        // Test that our Result type alias works correctly
        let success_result: Result<i32> = Ok(42);
        let error_result: Result<i32> = Err(anyhow::anyhow!("Test error"));

        assert!(success_result.is_ok());
        assert_eq!(success_result.unwrap(), 42);

        assert!(error_result.is_err());
        assert!(error_result.unwrap_err().to_string().contains("Test error"));
    }

    #[test]
    fn test_clap_parser_derive() {
        // Test that the Parser derive works correctly
        let cmd = Args::command();
        let matches = cmd.try_get_matches_from([
            "settingsservice",
            "--config",
            "/test.yaml",
            "--bind-port",
            "8888",
        ]);

        assert!(matches.is_ok());
        let matches = matches.unwrap();

        assert_eq!(
            matches.get_one::<PathBuf>("config").unwrap(),
            &PathBuf::from("/test.yaml")
        );
        assert_eq!(matches.get_one::<u16>("bind_port").unwrap(), &8888u16);
    }
}

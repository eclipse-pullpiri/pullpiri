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
//! - REST API and CLI interfaces
//! - Schema validation

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;

pub mod monitoring_etcd;
pub mod monitoring_types;
mod settings_api;
mod settings_cli;
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

    /// Enable CLI mode instead of server mode
    #[arg(long)]
    cli: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    init_logging(&args.log_level)?;

    info!("Starting PICCOLO Settings Service");
    info!("Config file: {:?}", args.config);
    info!("ETCD endpoints: {}", args.etcd_endpoints);

    if args.cli {
        // Run in CLI mode
        run_cli_mode(args).await
    } else {
        // Run in server mode
        run_server_mode(args).await
    }
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

async fn run_cli_mode(args: Args) -> Result<()> {
    info!("Starting in CLI mode");

    use settings_cli::cli::run_cli;

    let etcd_endpoints: Vec<String> = args
        .etcd_endpoints
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    run_cli(etcd_endpoints).await
}

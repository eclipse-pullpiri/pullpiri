// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! PICCOLO Settings Service CLI
//! 
//! Command-line interface for the Settings Service

use anyhow::Result;
use clap::Parser;

/// Settings CLI command line arguments
#[derive(Parser, Debug)]
#[command(name = "settings-cli")]
#[command(about = "PICCOLO Settings Service CLI")]
struct Args {
    /// ETCD endpoints (comma separated)
    #[arg(long, default_value = "localhost:2379")]
    etcd_endpoints: String,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    settingsservice::settings_utils::logging::init_logging(&args.log_level)?;

    let etcd_endpoints: Vec<String> = args.etcd_endpoints
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    settingsservice::settings_cli::cli::run_cli(etcd_endpoints).await
}
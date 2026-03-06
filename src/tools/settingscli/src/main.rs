/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! SettingsCLI - Command Line Interface for Pullpiri SettingsService
//!
//! This CLI tool provides a convenient way to interact with the Pullpiri SettingsService
//! via REST APIs. It supports various operations for managing boards, nodes, and SoCs.
//! YAML commands are routed directly to the API Server (port 47099) while all other
//! commands go to the SettingsService (port 8080).

use clap::{Parser, Subcommand};
use colored::Colorize;
use settingscli::commands::{board, container, metrics, node, soc, yaml};
use settingscli::{Result, SettingsClient};

#[derive(Parser)]
#[command(name = "settingscli")]
#[command(about = "CLI tool for Pullpiri SettingsService")]
#[command(version)]
#[command(long_about = None)]
struct Cli {
    /// Base URL (scheme and host, without port)
    #[arg(short, long, env = "PICCOLO_URL", default_value = "http://localhost")]
    url: String,

    /// SettingsService port
    #[arg(long, env = "SETTINGS_PORT", default_value = "8080")]
    settings_port: u16,

    /// API Server port
    #[arg(long, env = "API_PORT", default_value = "47099")]
    api_port: u16,

    /// Request timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get system metrics
    Metrics {
        #[command(subcommand)]
        action: metrics::MetricsAction,
    },
    /// Board-related operations
    Board {
        #[command(subcommand)]
        action: board::BoardAction,
    },
    /// Node-related operations
    Node {
        #[command(subcommand)]
        action: node::NodeAction,
    },
    /// SoC-related operations
    Soc {
        #[command(subcommand)]
        action: soc::SocAction,
    },
    /// Container-related operations
    Container {
        #[command(subcommand)]
        action: container::ContainerAction,
    },
    /// YAML artifact management
    Yaml {
        #[command(subcommand)]
        action: yaml::YamlAction,
    },
    /// Test connection to SettingsService and API Server
    Health,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let settings_url = format!("{}:{}", cli.url, cli.settings_port);
    let api_url = format!("{}:{}", cli.url, cli.api_port);

    if cli.verbose {
        println!(
            "{} SettingsService URL: {}",
            "ℹ".blue().bold(),
            settings_url
        );
        println!("{} API Server URL: {}", "ℹ".blue().bold(), api_url);
    }

    // Create SettingsService client
    let settings_client = match SettingsClient::new(&settings_url, cli.timeout) {
        Ok(client) => client,
        Err(e) => {
            eprintln!(
                "{} Failed to create SettingsService client: {}",
                "✗".red().bold(),
                e
            );
            std::process::exit(1);
        }
    };

    // Create API Server client
    let api_client = match SettingsClient::new(&api_url, cli.timeout) {
        Ok(client) => client,
        Err(e) => {
            eprintln!(
                "{} Failed to create API Server client: {}",
                "✗".red().bold(),
                e
            );
            std::process::exit(1);
        }
    };

    // Execute command: YAML commands go to API Server, others to SettingsService
    let result = match cli.command {
        Commands::Metrics { action } => metrics::handle(&settings_client, action).await,
        Commands::Board { action } => board::handle(&settings_client, action).await,
        Commands::Node { action } => node::handle(&settings_client, action).await,
        Commands::Soc { action } => soc::handle(&settings_client, action).await,
        Commands::Container { action } => container::handle(&settings_client, action).await,
        Commands::Yaml { action } => yaml::handle(&api_client, action).await,
        Commands::Health => health_check(&settings_client, &api_client).await,
    };

    match result {
        Ok(_) => {
            if cli.verbose {
                println!("{} Command completed successfully", "✓".green().bold());
            }
        }
        Err(e) => {
            eprintln!("{} Command failed: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Perform a health check on both SettingsService and API Server
async fn health_check(settings_client: &SettingsClient, api_client: &SettingsClient) -> Result<()> {
    println!("{} Checking SettingsService health...", "ℹ".blue().bold());

    match settings_client.health_check().await {
        Ok(true) => {
            println!(
                "{} SettingsService is healthy and reachable",
                "✓".green().bold()
            );
        }
        Ok(false) => {
            println!("{} SettingsService is not reachable", "✗".red().bold());
            return Err(settingscli::error::CliError::Custom(
                "SettingsService health check failed".to_string(),
            ));
        }
        Err(e) => {
            println!(
                "{} SettingsService health check failed: {}",
                "✗".red().bold(),
                e
            );
            return Err(e);
        }
    }

    println!("{} Checking API Server health...", "ℹ".blue().bold());

    match api_client.api_health_check().await {
        Ok(true) => {
            println!("{} API Server is healthy and reachable", "✓".green().bold());
        }
        Ok(false) => {
            println!("{} API Server is not reachable", "✗".red().bold());
            return Err(settingscli::error::CliError::Custom(
                "API Server health check failed".to_string(),
            ));
        }
        Err(e) => {
            println!("{} API Server health check failed: {}", "✗".red().bold(), e);
            return Err(e);
        }
    }

    Ok(())
}

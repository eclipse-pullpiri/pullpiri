// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! CLI main interface

use crate::settings_config::ConfigManager;
use crate::settings_history::HistoryManager;
use crate::settings_monitoring::MonitoringManager;
use crate::settings_storage::EtcdClient;
use crate::settings_utils::error::SettingsError;
use anyhow::Result;
use rustyline::{DefaultEditor, Editor};
use tracing::{debug, error, info};

/// Run the interactive CLI
pub async fn run_cli(etcd_endpoints: Vec<String>) -> Result<()> {
    info!("Starting Settings Service CLI");

    // Initialize storage clients
    let storage_config = EtcdClient::new(etcd_endpoints.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create config storage: {}", e))?;

    let storage_history = EtcdClient::new(etcd_endpoints.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create history storage: {}", e))?;

    let storage_monitoring = EtcdClient::new(etcd_endpoints)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create monitoring storage: {}", e))?;

    // Initialize managers
    let config_manager = ConfigManager::new(Box::new(storage_config));
    let history_manager = HistoryManager::new(Box::new(storage_history));
    let monitoring_manager = MonitoringManager::new(Box::new(storage_monitoring), 60);

    // Create CLI context
    let mut cli_context = CliContext {
        config_manager,
        history_manager,
        monitoring_manager,
    };

    // Start interactive shell
    let mut rl = DefaultEditor::new()?;

    println!("PICCOLO Settings Service CLI");
    println!("Type 'help' for available commands or 'quit' to exit");

    loop {
        match rl.readline("settings> ") {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line)?;

                if line == "quit" || line == "exit" {
                    break;
                }

                if let Err(e) = handle_command(&mut cli_context, line).await {
                    eprintln!("Error: {}", e);
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("Interrupted");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("EOF");
                break;
            }
            Err(err) => {
                error!("Error reading line: {}", err);
                break;
            }
        }
    }

    info!("CLI session ended");
    Ok(())
}

/// CLI context containing managers
struct CliContext {
    config_manager: ConfigManager,
    history_manager: HistoryManager,
    monitoring_manager: MonitoringManager,
}

/// Handle a CLI command
async fn handle_command(ctx: &mut CliContext, command: &str) -> Result<(), SettingsError> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(());
    }

    match parts[0] {
        "help" => show_help(),
        "config" => handle_config_command(ctx, &parts[1..]).await?,
        "metrics" => handle_metrics_command(ctx, &parts[1..]).await?,
        "history" => handle_history_command(ctx, &parts[1..]).await?,
        "status" => handle_status_command(ctx, &parts[1..]).await?,
        _ => println!(
            "Unknown command: {}. Type 'help' for available commands.",
            parts[0]
        ),
    }

    Ok(())
}

/// Show help information
fn show_help() {
    println!("Available commands:");
    println!("  config list [prefix]           - List configurations");
    println!("  config get <path>              - Get configuration");
    println!("  config set <path> <value>      - Set configuration");
    println!("  config delete <path>           - Delete configuration");
    println!("  config validate <path>         - Validate configuration");
    println!();
    println!("  metrics list                   - List all metrics");
    println!("  metrics get <id>               - Get specific metric");
    println!("  metrics filter <component>     - Filter metrics by component");
    println!("  metrics filters                - List all filters");
    println!();
    println!("  history <path>                 - Show configuration history");
    println!("  history rollback <path> <ver>  - Rollback to version");
    println!();
    println!("  status                         - Show system status");
    println!("  help                           - Show this help");
    println!("  quit/exit                      - Exit CLI");
}

/// Handle config commands
async fn handle_config_command(ctx: &mut CliContext, args: &[&str]) -> Result<(), SettingsError> {
    if args.is_empty() {
        println!("Usage: config <list|get|set|delete|validate> [args...]");
        return Ok(());
    }

    match args[0] {
        "list" => {
            let prefix = args.get(1).copied();
            let configs = ctx.config_manager.list_configs(prefix).await?;

            if configs.is_empty() {
                println!("No configurations found");
            } else {
                println!("Configurations:");
                for config in configs {
                    println!(
                        "  {} (v{}) - {} [{}]",
                        config.path,
                        config.version,
                        config.schema_type,
                        config.modified_at.format("%Y-%m-%d %H:%M")
                    );
                }
            }
        }
        "get" => {
            if args.len() < 2 {
                println!("Usage: config get <path>");
                return Ok(());
            }

            let config = ctx.config_manager.load_config(args[1]).await?;
            println!("Configuration: {}", config.path);
            println!("Version: {}", config.metadata.version);
            println!("Schema: {}", config.metadata.schema_type);
            println!("Author: {}", config.metadata.author);
            if let Some(comment) = &config.metadata.comment {
                println!("Comment: {}", comment);
            }
            println!("Content:");
            println!(
                "{}",
                serde_json::to_string_pretty(&config.content).unwrap_or_default()
            );
        }
        "set" => {
            if args.len() < 3 {
                println!("Usage: config set <path> <json_value>");
                return Ok(());
            }

            let path = args[1];
            let value_str = args[2..].join(" ");

            match serde_json::from_str(&value_str) {
                Ok(value) => {
                    // Try to update existing config
                    match ctx
                        .config_manager
                        .update_config(
                            path,
                            value,
                            "cli-user",
                            Some("Updated via CLI".to_string()),
                            None, // CLI doesn't have access to history manager
                        )
                        .await
                    {
                        Ok(config) => {
                            println!(
                                "Configuration updated: {} (v{})",
                                config.path, config.metadata.version
                            );
                        }
                        Err(_) => {
                            // If update fails, try to create new config
                            match ctx
                                .config_manager
                                .create_config(
                                    path,
                                    serde_json::from_str(&value_str).unwrap(),
                                    "generic",
                                    "cli-user",
                                    Some("Created via CLI".to_string()),
                                    None, // CLI doesn't have access to history manager
                                )
                                .await
                            {
                                Ok(config) => {
                                    println!(
                                        "Configuration created: {} (v{})",
                                        config.path, config.metadata.version
                                    );
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Invalid JSON value: {}", e);
                }
            }
        }
        "delete" => {
            if args.len() < 2 {
                println!("Usage: config delete <path>");
                return Ok(());
            }

            ctx.config_manager.delete_config(args[1], None).await?;
            println!("Configuration deleted: {}", args[1]);
        }
        "validate" => {
            if args.len() < 2 {
                println!("Usage: config validate <path>");
                return Ok(());
            }

            let config = ctx.config_manager.load_config(args[1]).await?;
            let result = ctx.config_manager.validate_config(&config).await?;

            if result.is_valid {
                println!("Configuration is valid");
            } else {
                println!("Configuration validation failed:");
                for error in result.errors {
                    println!("  {}: {}", error.path, error.message);
                }
            }
        }
        _ => {
            println!("Unknown config command: {}", args[0]);
        }
    }

    Ok(())
}

/// Handle metrics commands
async fn handle_metrics_command(ctx: &mut CliContext, args: &[&str]) -> Result<(), SettingsError> {
    if args.is_empty() {
        println!("Usage: metrics <list|get|filter|filters> [args...]");
        return Ok(());
    }

    match args[0] {
        "list" => {
            let metrics = ctx.monitoring_manager.get_metrics(None).await?;

            if metrics.is_empty() {
                println!("No metrics found");
            } else {
                println!("Metrics:");
                for metric in metrics.iter().take(10) {
                    // Show first 10
                    println!(
                        "  {} ({}) - {} = {:?}",
                        metric.id, metric.component, metric.metric_type, metric.value
                    );
                }
                if metrics.len() > 10 {
                    println!("  ... and {} more metrics", metrics.len() - 10);
                }
            }
        }
        "get" => {
            if args.len() < 2 {
                println!("Usage: metrics get <id>");
                return Ok(());
            }

            if let Some(metric) = ctx.monitoring_manager.get_metric_by_id(args[1]).await? {
                println!("Metric: {}", metric.id);
                println!("Component: {}", metric.component);
                println!("Type: {}", metric.metric_type);
                println!("Value: {:?}", metric.value);
                println!("Timestamp: {}", metric.timestamp);
                if !metric.labels.is_empty() {
                    println!("Labels:");
                    for (key, value) in &metric.labels {
                        println!("  {}: {}", key, value);
                    }
                }
            } else {
                println!("Metric not found: {}", args[1]);
            }
        }
        "filter" => {
            if args.len() < 2 {
                println!("Usage: metrics filter <component>");
                return Ok(());
            }

            let metrics = ctx
                .monitoring_manager
                .get_metrics_by_component(args[1])
                .await?;

            if metrics.is_empty() {
                println!("No metrics found for component: {}", args[1]);
            } else {
                println!("Metrics for component '{}':", args[1]);
                for metric in metrics {
                    println!(
                        "  {} ({}) = {:?}",
                        metric.id, metric.metric_type, metric.value
                    );
                }
            }
        }
        "filters" => {
            let filters = ctx.monitoring_manager.list_filters().await?;

            if filters.is_empty() {
                println!("No filters found");
            } else {
                println!("Filters:");
                for filter in filters {
                    println!(
                        "  {} - {} ({})",
                        filter.id,
                        filter.name,
                        if filter.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                }
            }
        }
        _ => {
            println!("Unknown metrics command: {}", args[0]);
        }
    }

    Ok(())
}

/// Handle history commands
async fn handle_history_command(ctx: &mut CliContext, args: &[&str]) -> Result<(), SettingsError> {
    if args.is_empty() {
        println!("Usage: history <path> or history rollback <path> <version>");
        return Ok(());
    }

    if args[0] == "rollback" {
        if args.len() < 3 {
            println!("Usage: history rollback <path> <version>");
            return Ok(());
        }

        let path = args[1];
        let version: u64 = args[2]
            .parse()
            .map_err(|_| SettingsError::Cli("Invalid version number".to_string()))?;

        let config = ctx
            .history_manager
            .rollback_to_version(
                path,
                version,
                &mut ctx.config_manager,
                "cli-user",
                Some(format!("Rollback to version {} via CLI", version)),
            )
            .await?;

        println!(
            "Rolled back {} to version {} (new version: {})",
            path, version, config.metadata.version
        );
    } else {
        let path = args[0];
        let history = ctx.history_manager.list_history(path, Some(10)).await?;

        if history.is_empty() {
            println!("No history found for: {}", path);
        } else {
            println!("History for '{}':", path);
            for entry in history {
                println!(
                    "  v{} - {} ({}) - {}",
                    entry.version,
                    entry.timestamp.format("%Y-%m-%d %H:%M"),
                    entry.author,
                    entry.change_summary
                );
                if let Some(comment) = &entry.comment {
                    println!("    Comment: {}", comment);
                }
            }
        }
    }

    Ok(())
}

/// Handle status commands
async fn handle_status_command(_ctx: &mut CliContext, _args: &[&str]) -> Result<(), SettingsError> {
    println!("Settings Service Status:");
    println!("  Version: {}", env!("CARGO_PKG_VERSION"));
    println!("  Mode: CLI");
    println!("  Status: Running");

    // TODO: Add real status information
    Ok(())
}

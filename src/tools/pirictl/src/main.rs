/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
//! PiriCTL - Command Line Interface for Pullpiri SettingsService
//!
//! This CLI tool provides a convenient way to interact with the Pullpiri SettingsService
//! via REST APIs. It supports various operations for managing boards, nodes, and SoCs.

use clap::{Parser, Subcommand};
use colored::Colorize;
use pirictl::commands::{board, container, metrics, node, soc, top, yaml};
use pirictl::{Result, SettingsClient};
use url::Url;

#[derive(Parser)]
#[command(name = "pirictl")]
#[command(about = "CLI tool for Pullpiri SettingsService")]
#[command(version)]
#[command(long_about = None)]
struct Cli {
    /// Base URL (host only, without port)
    #[arg(short, long, env = "PULLPIRI_URL", default_value = "http://localhost")]
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
    /// Get all resources
    Get {
        #[command(subcommand)]
        resource: ResourceType,
    },
    /// Describe specific resource by ID
    Describe {
        #[command(subcommand)]
        resource: ResourceTypeWithId,
    },
    /// Get resource data in raw JSON format
    Raw {
        #[command(subcommand)]
        resource: ResourceTypeRaw,
    },
    /// Display live system information
    Top {
        #[command(subcommand)]
        resource: top::TopResource,
    },
    /// Apply YAML artifact to the system
    Apply {
        /// Path to YAML file
        #[arg(short = 'f', long = "file")]
        file: String,
    },
    /// Delete YAML artifact from the system
    Delete {
        /// Path to YAML file
        #[arg(short = 'f', long = "file")]
        file: String,
    },
    /// Test connection to SettingsService
    Health,
}

#[derive(Subcommand)]
enum ResourceType {
    /// Get all boards
    Boards,
    /// Get all nodes
    Nodes,
    /// Get all SoCs
    Socs,
    /// Get all containers
    Containers,
}

#[derive(Subcommand)]
enum ResourceTypeWithId {
    /// Describe specific board by ID
    Board {
        /// Board ID
        id: String,
    },
    /// Describe specific node by ID
    Node {
        /// Node ID
        id: String,
    },
    /// Describe specific SoC by ID
    Soc {
        /// SoC ID
        id: String,
    },
    /// Describe specific container by ID
    Container {
        /// Container ID
        id: String,
    },
}

#[derive(Subcommand)]
enum ResourceTypeRaw {
    /// Get board data in raw JSON format
    Board {
        /// Board ID (optional)
        id: Option<String>,
    },
    /// Get node data in raw JSON format
    Node {
        /// Node ID (optional)
        id: Option<String>,
    },
    /// Get SoC data in raw JSON format
    Soc {
        /// SoC ID (optional)
        id: Option<String>,
    },
    /// Get container data in raw JSON format
    Container,
    /// Get metrics in raw JSON format
    Metrics,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Use the `url` crate to parse the base URL, replace the port, and build the final URLs
    let settings_url = build_url_with_port(&cli.url, cli.settings_port).unwrap_or_else(|e| {
        eprintln!("{} Invalid URL: {}", "✗".red().bold(), e);
        std::process::exit(1);
    });
    let api_url = build_url_with_port(&cli.url, cli.api_port).unwrap_or_else(|e| {
        eprintln!("{} Invalid URL: {}", "✗".red().bold(), e);
        std::process::exit(1);
    });

    if cli.verbose {
        println!(
            "{} SettingsService URL: {}",
            "ℹ".blue().bold(),
            settings_url
        );
        println!("{} API Server URL: {}", "ℹ".blue().bold(), api_url);
    }

    // Create two clients: one for SettingsService, one for API Server
    let settings_client = match SettingsClient::new(&settings_url, cli.timeout) {
        Ok(client) => client,
        Err(e) => {
            eprintln!(
                "{} Failed to create settings client: {}",
                "✗".red().bold(),
                e
            );
            std::process::exit(1);
        }
    };

    let api_client = match SettingsClient::new(&api_url, cli.timeout) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("{} Failed to create API client: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    };

    // Execute command - YAML commands go directly to API Server; others go to SettingsService
    let result = match cli.command {
        Commands::Get { resource } => match resource {
            ResourceType::Boards => board::handle(&settings_client, board::BoardAction::Get).await,
            ResourceType::Nodes => node::handle(&settings_client, node::NodeAction::Get).await,
            ResourceType::Socs => soc::handle(&settings_client, soc::SocAction::Get).await,
            ResourceType::Containers => {
                container::handle(&settings_client, container::ContainerAction::Get).await
            }
        },
        Commands::Describe { resource } => match resource {
            ResourceTypeWithId::Board { id } => {
                board::handle(&settings_client, board::BoardAction::Describe { id }).await
            }
            ResourceTypeWithId::Node { id } => {
                node::handle(&settings_client, node::NodeAction::Describe { id }).await
            }
            ResourceTypeWithId::Soc { id } => {
                soc::handle(&settings_client, soc::SocAction::Describe { id }).await
            }
            ResourceTypeWithId::Container { id } => {
                container::handle(
                    &settings_client,
                    container::ContainerAction::Describe { id },
                )
                .await
            }
        },
        Commands::Raw { resource } => match resource {
            ResourceTypeRaw::Board { id } => {
                board::handle(&settings_client, board::BoardAction::Raw { id }).await
            }
            ResourceTypeRaw::Node { id } => {
                node::handle(&settings_client, node::NodeAction::Raw { id }).await
            }
            ResourceTypeRaw::Soc { id } => {
                soc::handle(&settings_client, soc::SocAction::Raw { id }).await
            }
            ResourceTypeRaw::Container => {
                container::handle(&settings_client, container::ContainerAction::Raw).await
            }
            ResourceTypeRaw::Metrics => {
                metrics::handle(&settings_client, metrics::MetricsAction::Raw).await
            }
        },
        Commands::Top { resource } => top::handle(&settings_client, resource).await,
        Commands::Apply { file } => {
            yaml::handle(&api_client, yaml::YamlAction::Apply { file }).await
        }
        Commands::Delete { file } => {
            yaml::handle(&api_client, yaml::YamlAction::Withdraw { file }).await
        }
        Commands::Health => health_check(&settings_client).await,
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

/// Build a URL by parsing the given base URL and replacing (or setting) the port.
///
/// Any port already present in `base_url` is overwritten with `port`.
fn build_url_with_port(base_url: &str, port: u16) -> std::result::Result<String, String> {
    let mut parsed = Url::parse(base_url).map_err(|e| format!("'{}': {}", base_url, e))?;
    parsed
        .set_port(Some(port))
        .map_err(|_| format!("cannot set port on '{}'", base_url))?;
    // Remove trailing slash for consistency with the rest of the codebase
    Ok(parsed.as_str().trim_end_matches('/').to_string())
}

/// Perform a health check on the SettingsService
async fn health_check(client: &SettingsClient) -> Result<()> {
    println!("{} Checking SettingsService health...", "ℹ".blue().bold());

    match client.health_check().await {
        Ok(true) => {
            println!(
                "{} SettingsService is healthy and reachable",
                "✓".green().bold()
            );
        }
        Ok(false) => {
            println!("{} SettingsService is not reachable", "✗".red().bold());
            return Err(pirictl::error::CliError::Custom(
                "Health check failed".to_string(),
            ));
        }
        Err(e) => {
            println!("{} Health check failed: {}", "✗".red().bold(), e);
            return Err(e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── build_url_with_port ───────────────────────────────────────────────────

    #[test]
    fn test_build_url_replaces_port() {
        assert_eq!(
            build_url_with_port("http://localhost:8080", 47099).unwrap(),
            "http://localhost:47099"
        );
        assert_eq!(
            build_url_with_port("http://10.0.0.1:9999", 8080).unwrap(),
            "http://10.0.0.1:8080"
        );
    }

    #[test]
    fn test_build_url_sets_port_when_absent() {
        assert_eq!(
            build_url_with_port("http://localhost", 8080).unwrap(),
            "http://localhost:8080"
        );
        assert_eq!(
            build_url_with_port("http://10.231.178.2", 47099).unwrap(),
            "http://10.231.178.2:47099"
        );
    }

    #[test]
    fn test_build_url_invalid_input() {
        assert!(build_url_with_port("not-a-url", 8080).is_err());
    }

    #[test]
    fn test_build_url_strips_trailing_slash() {
        // The function should trim the trailing slash from the result
        let result = build_url_with_port("http://localhost", 9090).unwrap();
        assert!(
            !result.ends_with('/'),
            "URL should not end with slash: {}",
            result
        );
    }

    #[test]
    fn test_build_url_empty_string() {
        assert!(build_url_with_port("", 8080).is_err());
    }

    // ── health_check() ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_health_check_fn_healthy() {
        // Both health endpoints → primary OK → health_check returns Ok(true) → fn returns Ok(())
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/system/health"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"healthy": true})),
            )
            .mount(&server)
            .await;
        let client = SettingsClient::new(&server.uri(), 5).unwrap();
        let result = health_check(&client).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_fn_not_reachable() {
        // Both endpoints fail → health_check returns Ok(false) → fn returns Err (lines 280-284)
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/system/health"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/health"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        let client = SettingsClient::new(&server.uri(), 5).unwrap();
        let result = health_check(&client).await;
        // health_check client returns Ok(false) → our fn returns Err
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Health check failed"));
    }
}

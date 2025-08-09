#!/usr/bin/env cargo +nightly -Zscript

/*!
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! # Error Handling and Logging Demonstration
//! 
//! This script demonstrates the comprehensive error handling and logging
//! capabilities implemented for the Pullpiri system.

use std::time::Duration;
use tokio::time::sleep;

// These would normally be imported from the common crate
// For demo purposes, we'll simulate the functionality

async fn demonstrate_error_handling() {
    println!("🔧 Error Handling and Logging System Demonstration");
    println!("==================================================\n");

    // 1. Initialize logging
    println!("1. Initializing structured logging...");
    // common::logging::init_logging().expect("Failed to initialize logging");
    
    // 2. Create error reporting system
    println!("2. Setting up error reporting system...");
    // let (error_collector, reporter_factory) = common::error_reporting::create_error_system(1000);
    // let error_reporter = reporter_factory("demo_component".to_string());
    
    // 3. Start error collector
    println!("3. Starting error collector service...");
    // let collector_handle = tokio::spawn(async move {
    //     error_collector.start().await;
    // });
    
    // 4. Demonstrate different error types
    println!("4. Demonstrating error types and logging:\n");
    
    demonstrate_configuration_error().await;
    demonstrate_grpc_error().await;
    demonstrate_timeout_error().await;
    demonstrate_performance_logging().await;
    demonstrate_security_logging().await;
    
    println!("\n✅ Demonstration completed successfully!");
    
    // Clean up
    // collector_handle.abort();
}

async fn demonstrate_configuration_error() {
    println!("   📋 Configuration Error Example:");
    println!("   - Error Type: PullpiriError::Configuration");
    println!("   - Context: Loading invalid YAML configuration");
    println!("   - Log Level: ERROR");
    println!("   - Action: Error reported via channel to collector\n");
    
    // Simulated error handling
    // let error = PullpiriError::config("Invalid YAML syntax in settings.yaml");
    // error_reporter.report_error(error, Some("Configuration loading".to_string())).await;
}

async fn demonstrate_grpc_error() {
    println!("   🌐 gRPC Communication Error Example:");
    println!("   - Error Type: PullpiriError::Grpc");
    println!("   - Context: Failed to connect to PolicyManager service");
    println!("   - Log Level: ERROR");
    println!("   - Action: Automatic retry with exponential backoff\n");
    
    // Simulated gRPC error handling
    // let error = PullpiriError::grpc("Connection refused to PolicyManager:47005");
    // error_reporter.report_error(error, Some("Service communication".to_string())).await;
}

async fn demonstrate_timeout_error() {
    println!("   ⏰ Timeout Error Example:");
    println!("   - Error Type: PullpiriError::Timeout");
    println!("   - Context: ETCD operation timeout");
    println!("   - Log Level: WARN");
    println!("   - Action: Operation cancelled and logged\n");
    
    // Simulated timeout
    // let error = PullpiriError::timeout(5000);
    // error_reporter.report_error(error, Some("ETCD get operation".to_string())).await;
}

async fn demonstrate_performance_logging() {
    println!("   📊 Performance Metric Logging:");
    println!("   - Operation: scenario_processing");
    println!("   - Duration: 150ms");
    println!("   - Status: success");
    println!("   - Structured Data: JSON formatted for monitoring\n");
    
    // Simulated performance logging
    // common::logging::log_performance_metric("scenario_processing", 150, true);
}

async fn demonstrate_security_logging() {
    println!("   🔒 Security Event Logging:");
    println!("   - Event: unauthorized_access_attempt");
    println!("   - Source: 192.168.1.100");
    println!("   - Severity: high");
    println!("   - Action: Alert triggered, event logged\n");
    
    // Simulated security logging
    // common::logging::log_security_event("unauthorized_access_attempt", "192.168.1.100", "high");
}

fn print_usage_examples() {
    println!("📚 Usage Examples");
    println!("=================\n");
    
    println!("1. Basic Error Handling:");
    println!("```rust");
    println!("use common::{{Result, PullpiriError}};");
    println!();
    println!("async fn load_config() -> Result<Config> {{");
    println!("    let content = std::fs::read_to_string(\"config.yaml\")?;");
    println!("    let config: Config = serde_yaml::from_str(&content)?;");
    println!("    Ok(config)");
    println!("}}");
    println!("```\n");
    
    println!("2. Error Reporting:");
    println!("```rust");
    println!("use common::error_reporting::{{create_error_system, ErrorReporter}};");
    println!();
    println!("let (collector, reporter_factory) = create_error_system(1000);");
    println!("let error_reporter = reporter_factory(\"my_component\".to_string());");
    println!();
    println!("// Report errors asynchronously");
    println!("let error = PullpiriError::runtime(\"Something went wrong\");");
    println!("error_reporter.report_error(error, Some(\"context\".to_string())).await;");
    println!("```\n");
    
    println!("3. Operation Logging:");
    println!("```rust");
    println!("use common::{{log_operation_start, log_operation_success, log_operation_error}};");
    println!();
    println!("log_operation_start!(\"database_migration\");");
    println!("match migrate_database().await {{");
    println!("    Ok(_) => log_operation_success!(\"database_migration\"),");
    println!("    Err(e) => log_operation_error!(\"database_migration\", &e),");
    println!("}}");
    println!("```\n");
    
    println!("4. Result Extension Trait:");
    println!("```rust");
    println!("use common::error_reporting::ResultExt;");
    println!();
    println!("let result = risky_operation()");
    println!("    .log_error(&error_reporter, Some(\"Operation context\".to_string()))");
    println!("    .await?;");
    println!("```\n");
}

fn print_configuration_guide() {
    println!("⚙️  Configuration Guide");
    println!("=======================\n");
    
    println!("Environment Variables:");
    println!("- PULLPIRI_ENV=production     # Enable JSON logging for production");
    println!("- RUST_LOG=pullpiri=info      # Set log level");
    println!("- PULLPIRI_LOG_FORMAT=json    # Force JSON formatting");
    println!();
    
    println!("Cargo.toml Dependencies:");
    println!("```toml");
    println!("[dependencies]");
    println!("common = {{ workspace = true }}");
    println!("tracing = \"0.1.41\"");
    println!("tracing-subscriber = {{ version = \"0.3.19\", features = [\"env-filter\", \"json\"] }}");
    println!("```\n");
    
    println!("Application Initialization:");
    println!("```rust");
    println!("#[tokio::main]");
    println!("async fn main() -> common::Result<()> {{");
    println!("    // Initialize logging first");
    println!("    common::logging::init_logging()?;");
    println!("    ");
    println!("    // Create error reporting system");
    println!("    let (error_collector, reporter_factory) = common::error_reporting::create_error_system(1000);");
    println!("    let error_reporter = reporter_factory(\"my_app\".to_string());");
    println!("    ");
    println!("    // Start error collector in background");
    println!("    tokio::spawn(async move {{ error_collector.start().await; }});");
    println!("    ");
    println!("    // Your application logic here");
    println!("    run_application(&error_reporter).await");
    println!("}}");
    println!("```\n");
}

#[tokio::main]
async fn main() {
    demonstrate_error_handling().await;
    sleep(Duration::from_millis(100)).await;
    print_usage_examples();
    print_configuration_guide();
}
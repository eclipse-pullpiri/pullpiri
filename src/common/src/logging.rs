/*!
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use tracing::{info, warn, error};
use tracing_subscriber::{
    layer::SubscriberExt,
    util::SubscriberInitExt,
    fmt,
    EnvFilter,
};
use std::io;

/// Initialize structured logging for the application
/// 
/// Sets up tracing with JSON formatting for production environments
/// and human-readable formatting for development.
pub fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("pullpiri=info,common=info"))
        .unwrap();

    // Check if we're in production (when PULLPIRI_ENV=production)
    let is_production = std::env::var("PULLPIRI_ENV")
        .map(|env| env == "production")
        .unwrap_or(false);

    if is_production {
        // JSON formatting for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_target(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_writer(io::stdout)
            )
            .init();
    } else {
        // Human-readable formatting for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_target(true)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_writer(io::stdout)
            )
            .init();
    }

    info!("Logging initialized successfully");
    Ok(())
}

/// Log an operation start with context
#[macro_export]
macro_rules! log_operation_start {
    ($operation:expr) => {
        tracing::info!(operation = $operation, "Operation started");
    };
    ($operation:expr, $($field:tt)*) => {
        tracing::info!(operation = $operation, $($field)*, "Operation started");
    };
}

/// Log an operation success with context
#[macro_export]
macro_rules! log_operation_success {
    ($operation:expr) => {
        tracing::info!(operation = $operation, "Operation completed successfully");
    };
    ($operation:expr, $($field:tt)*) => {
        tracing::info!(operation = $operation, $($field)*, "Operation completed successfully");
    };
}

/// Log an operation failure with context
#[macro_export]
macro_rules! log_operation_error {
    ($operation:expr, $error:expr) => {
        tracing::error!(operation = $operation, error = %$error, "Operation failed");
    };
    ($operation:expr, $error:expr, $($field:tt)*) => {
        tracing::error!(operation = $operation, error = %$error, $($field)*, "Operation failed");
    };
}

/// Structured event logging for significant system events
pub fn log_system_event(event_type: &str, component: &str, details: &str) {
    info!(
        event_type = event_type,
        component = component,
        details = details,
        "System event occurred"
    );
}

/// Log performance metrics
pub fn log_performance_metric(operation: &str, duration_ms: u64, success: bool) {
    if success {
        info!(
            operation = operation,
            duration_ms = duration_ms,
            status = "success",
            "Performance metric"
        );
    } else {
        warn!(
            operation = operation,
            duration_ms = duration_ms,
            status = "failure",
            "Performance metric"
        );
    }
}

/// Log security events
pub fn log_security_event(event: &str, source: &str, severity: &str) {
    match severity {
        "critical" | "high" => {
            error!(
                security_event = event,
                source = source,
                severity = severity,
                "Security event detected"
            );
        }
        "medium" => {
            warn!(
                security_event = event,
                source = source,
                severity = severity,
                "Security event detected"
            );
        }
        _ => {
            info!(
                security_event = event,
                source = source,
                severity = severity,
                "Security event detected"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_init_logging() {
        // Test that logging initialization doesn't panic
        // In a real test environment, we'd capture and verify log output
        let result = init_logging();
        assert!(result.is_ok(), "Logging initialization should succeed");
    }
    
    #[test]
    fn test_log_functions() {
        // These tests mainly ensure the logging functions don't panic
        log_system_event("test_event", "test_component", "test details");
        log_performance_metric("test_operation", 100, true);
        log_performance_metric("test_operation", 200, false);
        log_security_event("test_security", "test_source", "high");
        log_security_event("test_security", "test_source", "low");
    }
}
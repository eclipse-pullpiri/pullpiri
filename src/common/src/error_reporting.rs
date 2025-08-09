/*!
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::error::{PullpiriError, ErrorReport, Result};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tracing::{error, warn, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Error reporter service for collecting and handling errors across the system
pub struct ErrorReporter {
    /// Sender for error reports
    tx: Sender<ErrorReport>,
    /// Component name
    component: String,
}

impl ErrorReporter {
    /// Create a new error reporter for a specific component
    pub fn new(component: String, tx: Sender<ErrorReport>) -> Self {
        Self { tx, component }
    }
    
    /// Report an error asynchronously
    pub async fn report_error(&self, error: PullpiriError, context: Option<String>) {
        let report = ErrorReport::new(error.to_string(), self.component.clone());
        let report = if let Some(ref ctx) = context {
            report.with_context(ctx.clone())
        } else {
            report
        };
        
        if let Err(e) = self.tx.send(report.clone()).await {
            // Fallback to direct logging if channel is closed
            error!(
                component = %self.component,
                error = %error,
                context = ?context,
                channel_error = %e,
                "Failed to send error report through channel, logging directly"
            );
        } else {
            tracing::debug!(
                component = %self.component,
                error = %error,
                "Error report sent successfully"
            );
        }
    }
    
    /// Report an error and return it for further handling
    pub async fn report_and_return<T>(&self, error: PullpiriError, context: Option<String>) -> Result<T> {
        self.report_error(error.clone(), context).await;
        Err(error)
    }
}

/// Error collection and monitoring service
pub struct ErrorCollector {
    /// Receiver for error reports
    rx: Receiver<ErrorReport>,
    /// Error statistics by component
    stats: Arc<RwLock<HashMap<String, ComponentErrorStats>>>,
}

#[derive(Debug, Clone)]
pub struct ComponentErrorStats {
    pub total_errors: u64,
    pub last_error: Option<chrono::DateTime<chrono::Utc>>,
    pub error_rate: f64, // errors per minute
}

impl ComponentErrorStats {
    fn new() -> Self {
        Self {
            total_errors: 0,
            last_error: None,
            error_rate: 0.0,
        }
    }
    
    fn record_error(&mut self) {
        self.total_errors += 1;
        self.last_error = Some(chrono::Utc::now());
        // Simple rate calculation (errors in last minute)
        // In production, this could be more sophisticated
        self.error_rate = self.total_errors as f64 / 60.0;
    }
}

impl ErrorCollector {
    /// Create a new error collector
    pub fn new(buffer_size: usize) -> (Self, Sender<ErrorReport>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        
        let collector = Self {
            rx,
            stats: Arc::new(RwLock::new(HashMap::new())),
        };
        
        (collector, tx)
    }
    
    /// Start the error collection service
    pub async fn start(mut self) {
        info!("Error collector service started");
        
        while let Some(error_report) = self.rx.recv().await {
            self.handle_error_report(error_report).await;
        }
        
        warn!("Error collector service stopped - channel closed");
    }
    
    /// Handle a single error report
    async fn handle_error_report(&self, report: ErrorReport) {
        // Log the error with structured data
        error!(
            component = %report.component,
            error = %report.error,
            timestamp = %report.timestamp,
            context = ?report.context,
            "Error reported by component"
        );
        
        // Update statistics
        let mut stats = self.stats.write().await;
        let component_stats = stats.entry(report.component.clone())
            .or_insert_with(ComponentErrorStats::new);
        component_stats.record_error();
        
        // Check for error rate thresholds and trigger alerts if needed
        if component_stats.error_rate > 10.0 { // More than 10 errors per minute
            warn!(
                component = %report.component,
                error_rate = component_stats.error_rate,
                total_errors = component_stats.total_errors,
                "High error rate detected for component"
            );
        }
    }
    
    /// Get error statistics for a component
    pub async fn get_component_stats(&self, component: &str) -> Option<ComponentErrorStats> {
        let stats = self.stats.read().await;
        stats.get(component).cloned()
    }
    
    /// Get error statistics for all components
    pub async fn get_all_stats(&self) -> HashMap<String, ComponentErrorStats> {
        let stats = self.stats.read().await;
        stats.clone()
    }
}

/// Create an error reporting system for the application
pub fn create_error_system(buffer_size: usize) -> (ErrorCollector, impl Fn(String) -> ErrorReporter) {
    let (collector, tx) = ErrorCollector::new(buffer_size);
    
    let reporter_factory = move |component: String| -> ErrorReporter {
        ErrorReporter::new(component, tx.clone())
    };
    
    (collector, reporter_factory)
}

/// Helper trait for adding error reporting to Results
pub trait ResultExt<T> {
    /// Log an error if Result is Err, then return the Result unchanged
    async fn log_error(self, reporter: &ErrorReporter, context: Option<String>) -> Result<T>;
    
    /// Report an error if Result is Err and convert to a different error type
    async fn report_error_as<E>(self, reporter: &ErrorReporter, context: Option<String>, error_mapper: impl FnOnce(PullpiriError) -> E) -> std::result::Result<T, E>;
}

impl<T> ResultExt<T> for Result<T> {
    async fn log_error(self, reporter: &ErrorReporter, context: Option<String>) -> Result<T> {
        if let Err(error) = &self {
            reporter.report_error(error.clone(), context).await;
        }
        self
    }
    
    async fn report_error_as<E>(self, reporter: &ErrorReporter, context: Option<String>, error_mapper: impl FnOnce(PullpiriError) -> E) -> std::result::Result<T, E> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                reporter.report_error(error.clone(), context).await;
                Err(error_mapper(error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_error_reporter() {
        let (collector, reporter_factory) = create_error_system(100);
        let reporter = reporter_factory("test_component".to_string());
        
        // Start collector in background
        let collector_handle = tokio::spawn(async move {
            collector.start().await;
        });
        
        // Report some errors
        let error = PullpiriError::runtime("Test error");
        reporter.report_error(error, Some("Test context".to_string())).await;
        
        // Give some time for processing
        sleep(Duration::from_millis(10)).await;
        
        // The test mainly verifies that no panics occur
        // In a real scenario, we'd verify the logged output
        
        // Cleanup
        drop(reporter);
        sleep(Duration::from_millis(10)).await;
        collector_handle.abort();
    }
    
    #[tokio::test]
    async fn test_result_ext() {
        let (collector, reporter_factory) = create_error_system(100);
        let reporter = reporter_factory("test_component".to_string());
        
        // Start collector in background
        let collector_handle = tokio::spawn(async move {
            collector.start().await;
        });
        
        // Test successful result
        let success_result: Result<i32> = Ok(42);
        let logged_result = success_result.log_error(&reporter, None).await;
        assert!(logged_result.is_ok());
        assert_eq!(logged_result.unwrap(), 42);
        
        // Test error result
        let error_result: Result<i32> = Err(PullpiriError::runtime("Test error"));
        let logged_result = error_result.log_error(&reporter, Some("Test context".to_string())).await;
        assert!(logged_result.is_err());
        
        // Cleanup
        drop(reporter);
        sleep(Duration::from_millis(10)).await;
        collector_handle.abort();
    }
}
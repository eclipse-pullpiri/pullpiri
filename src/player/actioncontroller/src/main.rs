use common::{Result, PullpiriError, error_reporting::{create_error_system, ErrorReporter}, logging};
use tracing::info;

mod grpc;
mod manager;
mod runtime;

/// Initialize the ActionController component
///
/// Reads node information from `settings.yaml` file, distinguishes between
/// Bluechi nodes and NodeAgent nodes, and sets up the initial configuration
/// for the component to start processing workload orchestration requests.
///
/// # Errors
///
/// Returns an error if:
/// - Configuration files cannot be read
/// - Node information is invalid
/// - gRPC server setup fails
async fn initialize(skip_grpc: bool, error_reporter: &ErrorReporter) -> Result<()> {
    common::log_operation_start!("actioncontroller_initialization");
    
    match perform_initialization(skip_grpc).await {
        Ok(_) => {
            common::log_operation_success!("actioncontroller_initialization");
            info!("ActionController initialized successfully");
            Ok(())
        }
        Err(e) => {
            let error = PullpiriError::runtime(format!("Failed to initialize ActionController: {}", e));
            error_reporter.report_error(error.clone(), Some("Component initialization".to_string())).await;
            common::log_operation_error!("actioncontroller_initialization", &error);
            Err(error)
        }
    }
}

async fn perform_initialization(skip_grpc: bool) -> Result<()> {
    // TODO: Implementation
    let manager = manager::ActionControllerManager::new();
    //Production code will not effect by this change
    if !skip_grpc {
        grpc::init(manager).await.map_err(|e| PullpiriError::grpc(e.to_string()))?;
    }

    Ok(())
}

/// Main function for the ActionController component
///
/// Sets up and runs the ActionController service which:
/// 1. Receives events from FilterGateway and StateManager
/// 2. Manages workloads via Bluechi Controller API or NodeAgent API
/// 3. Orchestrates node operations based on scenario requirements
///
/// # Errors
///
/// Returns an error if the service fails to start or encounters a
/// critical error during operation.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging first
    if let Err(e) = logging::init_logging() {
        eprintln!("Failed to initialize logging: {}", e);
        std::process::exit(1);
    }

    info!("Starting ActionController...");

    // Create error reporting system
    let (error_collector, reporter_factory) = create_error_system(1000);
    let error_reporter = reporter_factory("actioncontroller".to_string());

    // Start error collector in background
    let collector_handle = tokio::spawn(async move {
        error_collector.start().await;
    });

    let result = run_service(&error_reporter).await;

    // Clean shutdown
    info!("Shutting down ActionController...");
    collector_handle.abort();

    result
}

async fn run_service(error_reporter: &ErrorReporter) -> Result<()> {
    // Initialize the controller
    initialize(false, error_reporter).await?;

    // TODO: Set up gRPC server
    info!("ActionController service started successfully");

    // Keep the application running
    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            info!("Received shutdown signal");
            Ok(())
        }
        Err(e) => {
            let error = PullpiriError::runtime(format!("Failed to listen for shutdown signal: {}", e));
            error_reporter.report_error(error.clone(), Some("Signal handling".to_string())).await;
            Err(error)
        }
    }
}

//UNIT TEST
#[cfg(test)]
mod tests {
    use super::*;
    use common::error_reporting::create_error_system;

    // Positive test: initialize should succeed when skip_grpc is true
    #[tokio::test]
    async fn test_initialize_success() {
        let (_, reporter_factory) = create_error_system(10);
        let error_reporter = reporter_factory("test_component".to_string());
        
        let result = initialize(true, &error_reporter).await;
        assert!(
            result.is_ok(),
            "Expected initialize() to return Ok(), got Err: {:?}",
            result.err()
        );
    }

    // Negative test (edge case): double initialization (should not panic or fail)
    #[tokio::test]
    async fn test_double_initialize() {
        let (_, reporter_factory) = create_error_system(10);
        let error_reporter = reporter_factory("test_component".to_string());
        
        let first = initialize(true, &error_reporter).await;
        let second = initialize(true, &error_reporter).await;

        assert!(first.is_ok(), "First initialize() should succeed");
        assert!(second.is_ok(), "Second initialize() should succeed");
    }

    #[tokio::test]
    async fn test_perform_initialization() {
        let result = perform_initialization(true).await;
        assert!(result.is_ok(), "perform_initialization should succeed when skipping gRPC");
    }
}

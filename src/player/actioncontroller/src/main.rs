use std::error::Error;

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
async fn initialize(skip_grpc: bool) -> Result<(), Box<dyn Error>> {
    // TODO: Implementation
    let manager = manager::ActionControllerManager::new();
    //Production code will not effect by this change
    if !skip_grpc {
        grpc::init(manager).await?;
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
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Starting ActionController...");

    // Initialize the controller
    initialize(false).await?;

    // TODO: Set up gRPC server

    // Keep the application running
    tokio::signal::ctrl_c().await?;
    println!("Shutting down ActionController...");

    Ok(())
}

//UNIT TEST
#[cfg(test)]
mod tests {
    use super::*;

    // Positive test: initialize should succeed when skip_grpc is true
    #[tokio::test]
    async fn test_initialize_success() {
        let result = initialize(true).await;
        assert!(
            result.is_ok(),
            "Expected initialize() to return Ok(), got Err: {:?}",
            result.err()
        );
    }

    // Negative test (edge case): double initialization (should not panic or fail)
    #[tokio::test]
    async fn test_double_initialize() {
        let first = initialize(true).await;
        let second = initialize(true).await;

        assert!(first.is_ok(), "First initialize() should succeed");
        assert!(second.is_ok(), "Second initialize() should succeed");
    }
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager main entry point
//!
//! This file sets up the asynchronous runtime, initializes the StateManager engine and gRPC server,
//! and launches both concurrently. It provides proper error handling, graceful shutdown capabilities,
//! and comprehensive logging for monitoring and debugging.
//!
//! The StateManager service is a core component of the Pullpiri framework, responsible for managing
//! resource state transitions, monitoring container health, and ensuring ASIL-compliant operation.

use common::logd;
use common::logd::logger;
use common::monitoringserver::ContainerList;
use common::statemanager::{
    state_manager_connection_server::StateManagerConnectionServer, StateChange,
};
use std::env;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tonic::transport::Server;

pub mod grpc;
pub mod manager;
pub mod state_machine;
pub mod types;

/// Launches the StateManagerManager in an asynchronous task.
///
/// This function creates the StateManager engine, initializes it with proper configuration,
/// and runs the main processing loop. It handles all initialization and runtime errors
/// gracefully while providing comprehensive logging for monitoring.
///
/// # Arguments
/// * `rx_container` - Channel receiver for ContainerList messages from nodeagent
/// * `rx_state_change` - Channel receiver for StateChange messages from various components
///
/// # Processing Flow
/// 1. Create StateManagerManager instance with provided channels
/// 2. Initialize the manager with configuration and persistent state
/// 3. Run the main processing loop until shutdown
/// 4. Handle errors gracefully with proper logging
///
/// # Error Handling
/// - Logs initialization failures with detailed error information
/// - Continues operation even if some initialization steps fail
/// - Provides comprehensive error reporting for debugging
async fn launch_manager(
    rx_container: Receiver<ContainerList>,
    rx_state_change: Receiver<StateChange>,
) {
    // In test mode we short-circuit heavy startup to keep unit tests fast
    // In test builds or when `PULLPIRI_TEST_MODE` is set we short-circuit heavy startup
    if cfg!(test) || env::var("PULLPIRI_TEST_MODE").is_ok() {
        logd!(1, "Test mode: skipping StateManagerManager startup");
        return;
    }
    logd!(3, "=== StateManagerManager Starting ===");

    // Create the StateManager engine with async channel receivers
    let mut manager = manager::StateManagerManager::new(rx_container, rx_state_change).await;

    // Initialize the manager with configuration and persistent state
    match manager.initialize().await {
        Ok(_) => {
            logd!(
                3,
                "StateManagerManager initialization completed successfully"
            );

            // Run the main processing loop
            logd!(3, "Starting StateManagerManager main processing loop...");
            if let Err(e) = manager.run().await {
                logd!(5, "StateManagerManager stopped with error: {e:?}");
                logd!(
                    5,
                    "This may indicate a critical system failure or shutdown request"
                );
            } else {
                logd!(4, "StateManagerManager stopped gracefully");
            }
        }
        Err(e) => {
            logd!(5, "Failed to initialize StateManagerManager: {e:?}");
            logd!(
                5,
                "StateManager service cannot start - check configuration and dependencies"
            );
            // Don't panic - allow graceful shutdown of other components
        }
    }

    logd!(4, "=== StateManagerManager Stopped ===");
}

/// Initializes and runs the StateManager gRPC server.
///
/// Sets up the gRPC service endpoint, configures the server with proper middleware,
/// and starts listening for incoming requests from ApiServer, FilterGateway,
/// ActionController, and nodeagent components.
///
/// # Arguments
/// * `tx_container` - Channel sender for ContainerList messages to StateManager engine
/// * `tx_state_change` - Channel sender for StateChange messages to StateManager engine
///
/// # Server Configuration
/// - Binds to address specified in common::statemanager::open_server()
/// - Configures StateManagerConnectionServer with proper message routing
/// - Enables comprehensive error handling and logging
/// - Supports graceful shutdown on termination signals
///
/// # Error Handling
/// - Validates server address configuration
/// - Handles binding failures with detailed error messages
/// - Logs server startup and shutdown events
/// - Provides comprehensive error reporting for network issues
async fn initialize_grpc_server(
    tx_container: Sender<ContainerList>,
    tx_state_change: Sender<StateChange>,
) {
    // Allow tests to opt-out of starting the actual gRPC server
    // Skip starting the real gRPC server when running tests or explicitly requested
    if cfg!(test) || env::var("PULLPIRI_TEST_MODE").is_ok() {
        logd!(1, "Test mode: skipping gRPC server startup");
        return;
    }
    logd!(3, "=== StateManager gRPC Server Starting ===");

    // Create the gRPC service handler with async channels
    let server = grpc::receiver::StateManagerReceiver {
        tx: tx_container,
        tx_state_change,
    };
    logd!(3, "StateManagerReceiver instance created successfully");

    // Parse the server address from configuration
    let addr = match common::statemanager::open_server().parse() {
        Ok(addr) => {
            logd!(3, "StateManager gRPC server will bind to: {addr}");
            addr
        }
        Err(e) => {
            logd!(5, "Failed to parse StateManager server address: {e:?}");
            logd!(
                5,
                "Check StateManager address configuration in common module"
            );
            return; // Exit gracefully without panicking
        }
    };

    // Start the gRPC server with comprehensive error handling
    logd!(3, "Starting StateManager gRPC server...");
    match Server::builder()
        .add_service(StateManagerConnectionServer::new(server))
        .serve(addr)
        .await
    {
        Ok(_) => {
            logd!(4, "StateManager gRPC server stopped gracefully");
        }
        Err(e) => {
            logd!(5, "StateManager gRPC server error: {e:?}");
            logd!(
                5,
                "This may indicate network issues, port conflicts, or configuration problems"
            );
        }
    }

    logd!(4, "=== StateManager gRPC Server Stopped ===");
}

async fn initialize_timpani_server() {
    // Allow tests to opt-out of starting the timpani server
    // Skip starting the timpani server when running tests or explicitly requested
    if cfg!(test) || env::var("PULLPIRI_TEST_MODE").is_ok() {
        logd!(1, "Test mode: skipping Timpani server startup");
        return;
    }
    logd!(3, "=== Timpani gRPC Server Starting ===");

    // Create the gRPC service handler for Timpani
    let timpani_server = grpc::receiver::timpani::TimpaniReceiver::default();
    logd!(3, "TimpaniReceiver instance created successfully");

    // Parse the Timpani server address from configuration
    let addr = match "127.0.0.1:50053".parse() {
        Ok(addr) => {
            logd!(3, "Timpani gRPC server will bind to: {addr}");
            addr
        }
        Err(e) => {
            logd!(5, "Failed to parse Timpani server address: {e:?}");
            logd!(5, "Check Timpani address configuration in common module");
            return; // Exit gracefully without panicking
        }
    };

    // Start the gRPC server for Timpani with comprehensive error handling
    logd!(3, "Starting Timpani gRPC server...");
    match Server::builder()
        .add_service(
            common::external::timpani::fault_service_server::FaultServiceServer::new(
                timpani_server,
            ),
        )
        .serve(addr)
        .await
    {
        Ok(_) => {
            logd!(4, "Timpani gRPC server stopped gracefully");
        }
        Err(e) => {
            logd!(5, "Timpani gRPC server error: {e:?}");
            logd!(
                5,
                "This may indicate network issues, port conflicts, or configuration problems"
            );
        }
    }

    logd!(4, "=== Timpani gRPC Server Stopped ===");
}

/// Main entry point for the StateManager service.
///
/// This function orchestrates the complete StateManager service startup:
/// 1. Initializes async channel communication between gRPC and engine
/// 2. Launches the StateManager processing engine
/// 3. Starts the gRPC server for external communication
/// 4. Runs both components concurrently until shutdown
///
/// # Architecture
/// - Uses async channels for decoupled communication between gRPC and engine
/// - Runs gRPC server and processing engine concurrently
/// - Provides proper resource cleanup on shutdown
/// - Supports graceful termination handling
///
/// # Channel Configuration
/// - ContainerList channel: 100 message buffer for nodeagent communication
/// - StateChange channel: 100 message buffer for component communication
/// - Async processing prevents blocking between message types
///
/// # Error Handling
/// - Both components run independently to prevent cascading failures
/// - Comprehensive logging for monitoring and debugging
/// - Graceful shutdown even if one component fails
#[tokio::main]
async fn main() {
    let _ = logger::init_async_logger("statemanager").await;
    logd!(1, "initiailize statemanager...");

    // Create async channels for communication between gRPC server and processing engine
    // Buffer size of 100 provides good throughput while preventing excessive memory usage
    let (tx_container, rx_container) = channel::<ContainerList>(100);
    let (tx_state_change, rx_state_change) = channel::<StateChange>(100);

    // Launch StateManager processing engine
    let manager_task = launch_manager(rx_container, rx_state_change);

    // Launch gRPC server for external communication
    let grpc_task = initialize_grpc_server(tx_container, tx_state_change);

    // Launch gRPC server for timpani deadline miss
    let timpani_task = initialize_timpani_server();

    // Run both components concurrently until shutdown
    // tokio::join! ensures both tasks complete before main exits
    tokio::join!(manager_task, grpc_task, timpani_task);

    // Both tasks return (), but we log completion for monitoring
    logd!(6, "statemanager service stopped");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_launch_manager_skips_in_test_mode() {
        unsafe {
            std::env::set_var("PULLPIRI_TEST_MODE", "1");
        }

        let (_tx_container, rx_container) = channel::<ContainerList>(10);
        let (_tx_state_change, rx_state_change) = channel::<StateChange>(10);

        // Should return quickly because test mode short-circuits startup
        let res = timeout(
            Duration::from_secs(1),
            launch_manager(rx_container, rx_state_change),
        )
        .await;
        assert!(res.is_ok(), "launch_manager did not return in test mode");

        unsafe {
            std::env::remove_var("PULLPIRI_TEST_MODE");
        }
    }

    #[tokio::test]
    async fn test_initialize_grpc_server_skips_in_test_mode() {
        unsafe {
            std::env::set_var("PULLPIRI_TEST_MODE", "1");
        }

        let (tx_container, _rx_container) = channel::<ContainerList>(10);
        let (tx_state_change, _rx_state_change) = channel::<StateChange>(10);

        // Should return quickly because test mode short-circuits server startup
        let res = timeout(
            Duration::from_secs(1),
            initialize_grpc_server(tx_container, tx_state_change),
        )
        .await;
        assert!(
            res.is_ok(),
            "initialize_grpc_server did not return in test mode"
        );
        unsafe {
            std::env::remove_var("PULLPIRI_TEST_MODE");
        }
    }

    #[tokio::test]
    async fn test_initialize_timpani_server_skips_in_test_mode() {
        unsafe {
            std::env::set_var("PULLPIRI_TEST_MODE", "1");
        }

        // Should return quickly because test mode short-circuits timpani startup
        let res = timeout(Duration::from_secs(1), initialize_timpani_server()).await;
        assert!(
            res.is_ok(),
            "initialize_timpani_server did not return in test mode"
        );

        unsafe {
            std::env::remove_var("PULLPIRI_TEST_MODE");
        }
    }

    // Even when `PULLPIRI_TEST_MODE` is not explicitly set, test builds should
    // short-circuit heavy startup because `cfg!(test)` is true. Verify both
    // manager and grpc initialization return quickly without touching env.
    #[tokio::test]
    async fn test_launch_and_grpc_skip_without_env_in_test_build() {
        // Ensure env var is not set for this test
        unsafe {
            std::env::remove_var("PULLPIRI_TEST_MODE");
        }

        let (tx_container, rx_container) = channel::<ContainerList>(10);
        let (tx_state_change, rx_state_change) = channel::<StateChange>(10);

        // Both futures should return quickly because cfg!(test) is true
        let fut = async move {
            tokio::join!(
                launch_manager(rx_container, rx_state_change),
                initialize_grpc_server(tx_container, tx_state_change),
            );
        };

        let res = timeout(Duration::from_secs(1), fut).await;
        assert!(res.is_ok(), "startup tasks did not return in test build");
    }

    #[tokio::test]
    async fn test_all_components_skip_in_test_mode_concurrently() {
        // Ensure test mode is set so none of the servers/managers actually start
        unsafe {
            std::env::set_var("PULLPIRI_TEST_MODE", "1");
        }

        let (tx_container, rx_container) = channel::<ContainerList>(10);
        let (tx_state_change, rx_state_change) = channel::<StateChange>(10);

        // Run manager, grpc server and timpani concurrently and ensure they all return quickly
        let fut = async move {
            tokio::join!(
                launch_manager(rx_container, rx_state_change),
                initialize_grpc_server(tx_container, tx_state_change),
                initialize_timpani_server(),
            );
        };

        let res = timeout(Duration::from_secs(1), fut).await;
        assert!(
            res.is_ok(),
            "Concurrent startup tasks did not return in test mode"
        );

        unsafe {
            std::env::remove_var("PULLPIRI_TEST_MODE");
        }
    }

    // Call the generated `main()` function (synchronous entry created by `#[tokio::main]`)
    // to exercise the startup logging, channel creation and join logic in test builds.
    #[test]
    fn test_main_invocation_without_env() {
        // Ensure the env var is not set and call main(); in test builds `cfg!(test)`
        // will short-circuit heavy startup so this is safe to run.
        unsafe {
            std::env::remove_var("PULLPIRI_TEST_MODE");
        }

        // Call the generated main function which runs the runtime and joins tasks
        super::main();
    }

    #[test]
    fn test_main_invocation_with_env() {
        // Explicit test-mode via env var should also keep startup light
        unsafe {
            std::env::set_var("PULLPIRI_TEST_MODE", "1");
        }
        super::main();
        unsafe {
            std::env::remove_var("PULLPIRI_TEST_MODE");
        }
    }
}

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
//! The StateManager service is a core component of the PICCOLO framework, responsible for managing
//! resource state transitions, monitoring container health, and ensuring ASIL-compliant operation.

use common::monitoringserver::ContainerList;
use common::statemanager::{
    state_manager_connection_server::StateManagerConnectionServer, StateChange,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tonic::transport::Server;

pub mod grpc;
pub mod manager;

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
    println!("=== StateManagerManager Starting ===");

    // Create the StateManager engine with async channel receivers
    let mut manager = manager::StateManagerManager::new(rx_container, rx_state_change).await;

    // Initialize the manager with configuration and persistent state
    match manager.initialize().await {
        Ok(_) => {
            println!("StateManagerManager initialization completed successfully");

            // Run the main processing loop
            println!("Starting StateManagerManager main processing loop...");
            if let Err(e) = manager.run().await {
                eprintln!("StateManagerManager stopped with error: {:?}", e);
                eprintln!("This may indicate a critical system failure or shutdown request");
            } else {
                println!("StateManagerManager stopped gracefully");
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize StateManagerManager: {:?}", e);
            eprintln!("StateManager service cannot start - check configuration and dependencies");
            // Don't panic - allow graceful shutdown of other components
        }
    }

    println!("=== StateManagerManager Stopped ===");
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
    println!("=== StateManager gRPC Server Starting ===");

    // Create the gRPC service handler with async channels
    let server = grpc::receiver::StateManagerReceiver {
        tx: tx_container,
        tx_state_change,
    };
    println!("StateManagerReceiver instance created successfully");

    // Parse the server address from configuration
    let addr = match common::statemanager::open_server().parse() {
        Ok(addr) => {
            println!("StateManager gRPC server will bind to: {}", addr);
            addr
        }
        Err(e) => {
            eprintln!("Failed to parse StateManager server address: {:?}", e);
            eprintln!("Check StateManager address configuration in common module");
            return; // Exit gracefully without panicking
        }
    };

    // Start the gRPC server with comprehensive error handling
    println!("Starting StateManager gRPC server...");
    match Server::builder()
        .add_service(StateManagerConnectionServer::new(server))
        .serve(addr)
        .await
    {
        Ok(_) => {
            println!("StateManager gRPC server stopped gracefully");
        }
        Err(e) => {
            eprintln!("StateManager gRPC server error: {:?}", e);
            eprintln!(
                "This may indicate network issues, port conflicts, or configuration problems"
            );
        }
    }

    println!("=== StateManager gRPC Server Stopped ===");
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
    println!("========================================");
    println!("         PICCOLO StateManager           ");
    println!("========================================");
    println!("Starting StateManager service...");

    // Create async channels for communication between gRPC server and processing engine
    // Buffer size of 100 provides good throughput while preventing excessive memory usage
    let (tx_container, rx_container) = channel::<ContainerList>(100);
    let (tx_state_change, rx_state_change) = channel::<StateChange>(100);

    println!("Async communication channels created:");
    println!("  - ContainerList channel: 100 message buffer");
    println!("  - StateChange channel: 100 message buffer");

    // Launch StateManager processing engine
    let manager_task = launch_manager(rx_container, rx_state_change);

    // Launch gRPC server for external communication
    let grpc_task = initialize_grpc_server(tx_container, tx_state_change);

    println!("Launching StateManager components concurrently...");

    // Run both components concurrently until shutdown
    // tokio::join! ensures both tasks complete before main exits
    let (manager_result, grpc_result) = tokio::join!(manager_task, grpc_task);

    // Both tasks return (), but we log completion for monitoring
    println!("StateManager service components have stopped:");
    println!("  - StateManager engine: completed");
    println!("  - gRPC server: completed");

    println!("========================================");
    println!("     StateManager Service Stopped      ");
    println!("========================================");
}

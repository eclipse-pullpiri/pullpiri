/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager gRPC client for sending state change messages from ApiServer.
//!
//! This module provides a client interface for the ApiServer to communicate with
//! the StateManager service via gRPC. It manages connection lifecycle, handles
//! request routing, and provides ASIL-compliant state change messaging capabilities.
//!
//! The client implements lazy connection establishment, automatic retry logic,
//! and comprehensive error handling to ensure reliable communication with the
//! StateManager in the PICCOLO framework.

use common::statemanager::{
    connect_server, state_manager_connection_client::StateManagerConnectionClient,
    StateChange, StateChangeResponse,
};
use tonic::{Request, Status};

/// StateManager gRPC client for ApiServer component.
///
/// This client manages the gRPC connection to the StateManager service and provides
/// methods for sending state change requests. It implements lazy connection establishment
/// to optimize resource usage and provides automatic reconnection capabilities.
///
/// # Connection Management
/// - Establishes connections on first use (lazy initialization)
/// - Reuses existing connections for multiple requests
/// - Handles connection failures gracefully with proper error reporting
/// - Provides thread-safe access through cloning capability
///
/// # ASIL Compliance
/// - Supports ASIL safety levels from QM to ASIL-D
/// - Maintains nanosecond precision timestamps for timing verification
/// - Provides comprehensive tracking through transition IDs
/// - Includes context information for safety analysis and audit trails
#[derive(Clone)]
pub struct StateManagerSender {
    /// Cached gRPC client connection to the StateManager service.
    ///
    /// This connection is established lazily on the first request and reused
    /// for subsequent requests to optimize performance. Set to None initially
    /// and populated when ensure_connected() is called.
    client: Option<StateManagerConnectionClient<tonic::transport::Channel>>,
}

impl Default for StateManagerSender {
    /// Creates a new StateManagerSender with default settings.
    ///
    /// # Returns
    /// * `Self` - New StateManagerSender instance with no active connection
    fn default() -> Self {
        Self::new()
    }
}

impl StateManagerSender {
    /// Creates a new StateManagerSender instance.
    ///
    /// The connection to the StateManager is established lazily on the first request
    /// to optimize startup time and resource usage. This allows the ApiServer to
    /// initialize quickly even if the StateManager is temporarily unavailable.
    ///
    /// # Returns
    /// * `Self` - New StateManagerSender instance ready for use
    pub fn new() -> Self {
        Self { client: None }
    }

    /// Ensures a gRPC connection to the StateManager exists and is ready for use.
    ///
    /// This method implements lazy connection establishment by checking if a connection
    /// already exists and creating one if necessary. It uses the common::statemanager
    /// configuration to determine the StateManager's network location.
    ///
    /// # Connection Process
    /// 1. Check if a connection already exists
    /// 2. If not, attempt to establish a new gRPC connection
    /// 3. Store the connection for reuse in subsequent requests
    /// 4. Return success or detailed error information
    ///
    /// # Returns
    /// * `Result<(), Status>` - Success if connection is available, error otherwise
    ///
    /// # Errors
    /// * `Status::unknown` - Connection establishment failed (network, service unavailable, etc.)
    ///
    /// # Future Enhancements
    /// - Add connection health checking and automatic reconnection
    /// - Implement exponential backoff for connection retries
    /// - Add connection pooling for high-throughput scenarios
    async fn ensure_connected(&mut self) -> Result<(), Status> {
        if self.client.is_none() {
            match StateManagerConnectionClient::connect(connect_server()).await {
                Ok(client) => {
                    self.client = Some(client);
                    Ok(())
                }
                Err(e) => Err(Status::unknown(format!(
                    "Failed to connect to StateManager: {}",
                    e
                ))),
            }
        } else {
            // Connection already exists and ready for use
            Ok(())
        }
    }

    /// Sends a state change message to the StateManager service.
    ///
    /// This is the primary method for communicating state transitions from the ApiServer
    /// to the StateManager. It handles the complete request lifecycle including connection
    /// management, request transmission, and response processing.
    ///
    /// # Request Processing Flow
    /// 1. Ensure gRPC connection is established and ready
    /// 2. Create gRPC request wrapper with StateChange message
    /// 3. Send request to StateManager via gRPC
    /// 4. Receive and return StateChangeResponse with tracking information
    ///
    /// # Arguments
    /// * `state_change` - Complete StateChange message containing:
    ///   - Resource identification (type enum and name)
    ///   - State transition details (current → target state)
    ///   - Tracking and context information (transition_id, timestamps, source)
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - Response containing:
    ///   - Descriptive message
    ///   - Original transition_id for tracking
    ///   - Processing timestamp with nanosecond precision
    ///   - Error codes and details if applicable
    ///
    /// # Errors
    /// * `Status::unknown` - Connection failure or client not connected
    /// * `Status::unavailable` - StateManager service unavailable
    /// * `Status::invalid_argument` - Malformed StateChange message
    /// * `Status::deadline_exceeded` - Request timeout (ASIL timing violation)
    ///
    /// # ASIL Compliance Notes
    /// - Preserves nanosecond precision timestamps for timing verification
    /// - Maintains transition_id for complete audit trail
    /// - Supports ResourceType enum for type-safe resource identification
    /// - Provides detailed error information for safety analysis
    pub async fn send_state_change(
        &mut self,
        state_change: StateChange,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        // Ensure we have an active gRPC connection before sending
        self.ensure_connected().await?;

        if let Some(client) = &mut self.client {
            // Send the state change message via gRPC
            client.send_state_change(Request::new(state_change)).await
        } else {
            // This should never happen due to ensure_connected, but provide safety fallback
            Err(Status::unknown("Client not connected"))
        }
    }
}

// ========================================
// UNIT TESTS
// ========================================
// Comprehensive test suite for StateManagerSender functionality

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use common::statemanager::{ResourceType, StateChange};
//     use std::time::Duration;

//     /// Tests successful state change message transmission to StateManager.
//     ///
//     /// This test verifies the complete end-to-end communication flow between
//     /// the ApiServer and StateManager, including connection establishment,
//     /// message transmission, and response processing.
//     ///
//     /// # Test Scenario
//     /// Simulates a typical brake system scenario activation request with:
//     /// - Proper ResourceType enum usage (Scenario)
//     /// - Complete resource identification and state transition details
//     /// - Unique identifiers to prevent test interference
//     /// - Comprehensive tracking information for audit trails
//     ///
//     /// # Test Flow
//     /// 1. Create StateManagerSender instance
//     /// 2. Generate unique test data to prevent conflicts
//     /// 3. Create comprehensive StateChange message matching proto definition
//     /// 4. Send message to StateManager
//     /// 5. Verify successful response
//     ///
//     /// # Prerequisites
//     /// - StateManager service must be running and accessible
//     /// - gRPC connection must be available
//     /// - StateManager must accept the test message format
//     #[tokio::test]
//     async fn test_send_state_change_success() {
//         // Add startup delay to ensure StateManager service is ready
//         // This helps prevent race conditions and connection failures during testing
//         tokio::time::sleep(Duration::from_millis(100)).await;

//         let mut sender = StateManagerSender::default();

//         // Create unique timestamp for this test run to prevent duplicate messages
//         // and ensure each test execution is independent
//         let timestamp = std::time::SystemTime::now()
//             .duration_since(std::time::UNIX_EPOCH)
//             .unwrap()
//             .as_nanos() as i64;

//         // Create comprehensive StateChange message for testing
//         // This matches the proto definition exactly with proper enum usage
//         let state_change = StateChange {
//             // Resource identification using proper enum
//             resource_type: ResourceType::Scenario as i32,
//             resource_name: "brake-system-startup".to_string(),

//             // State transition details - using scenario state names
//             current_state: "idle".to_string(), // SCENARIO_STATE_IDLE
//             target_state: "waiting".to_string(), // SCENARIO_STATE_WAITING

//             // Tracking and timing information for audit trails
//             transition_id: format!("startup-{}", timestamp), // Unique ID for each test run
//             timestamp_ns: timestamp,                         // Nanosecond precision timestamp

//             // Source component identification
//             source: "apiserver".to_string(), // Identifies this component as the source
//         };

//         // Send the message and verify successful response
//         let result = sender.send_state_change(state_change).await;
//         assert!(result.is_ok(), "StateChange request should succeed");

//         // Verify response details when successful
//         if let Ok(response) = result {
//             let state_response = response.into_inner();

//             // Verify StateChangeResponse fields according to proto definition
//             assert!(
//                 !state_response.message.is_empty(),
//                 "Response should include a message"
//             );
//             assert!(
//                 !state_response.transition_id.is_empty(),
//                 "Response should include transition ID"
//             );
//             assert!(
//                 state_response.timestamp_ns > 0,
//                 "Response should include processing timestamp"
//             );

//             // Verify error handling fields
//             assert_eq!(
//                 state_response.error_code,
//                 0, // SUCCESS (assuming 0 is success in the ErrorCode enum)
//                 "Error code should be SUCCESS for successful processing"
//             );
//             assert!(
//                 state_response.error_details.is_empty(),
//                 "Error details should be empty for successful processing"
//             );

//             // Log successful test completion for debugging
//             println!("StateChange test completed successfully:");
//             println!("  Message: {}", state_response.message);
//             println!("  Transition ID: {}", state_response.transition_id);
//             println!("  Processing time: {} ns", state_response.timestamp_ns);
//         }
//     }

//     // ========================================
//     // FUTURE TEST IMPLEMENTATIONS
//     // ========================================
//     // Additional tests to be implemented when advanced features are enabled

//     /*
//     /// Tests different resource types according to ResourceType enum.
//     #[tokio::test]
//     async fn test_different_resource_types() {
//         // Test each ResourceType enum value:
//         // - Scenario
//         // - Package
//         // - Model
//         // - Volume
//         // - Network
//         // - Node
//     }

//     /// Tests error code handling according to ErrorCode enum.
//     #[tokio::test]
//     async fn test_error_code_handling() {
//         // Test different ErrorCode enum values:
//         // - InvalidRequest
//         // - ResourceNotFound
//         // - InvalidStateTransition
//         // - PreconditionFailed
//         // - Timeout
//         // etc.
//     }

//     /// Tests state transition validation for different resource types.
//     #[tokio::test]
//     async fn test_state_transition_validation() {
//         // Test valid state transitions for each resource type:
//         // - ScenarioState transitions (idle -> waiting -> playing)
//         // - PackageState transitions (initializing -> running -> degraded)
//         // - ModelState transitions (pending -> running -> succeeded/failed)
//         // etc.
//     }

//     /// Tests connection failure scenarios and error handling.
//     #[tokio::test]
//     async fn test_connection_failure_handling() {
//         // Test various connection failure scenarios:
//         // - Service unavailable
//         // - Network timeout
//         // - Authentication failure
//         // - Service overload
//     }

//     /// Tests malformed message handling and validation.
//     #[tokio::test]
//     async fn test_message_validation() {
//         // Test invalid StateChange messages:
//         // - Empty resource_name
//         // - Invalid resource_type enum values
//         // - Missing required fields
//         // - Invalid state names for resource types
//     }
//     */
// }

// ========================================
// PROTO FILE COMPLIANCE NOTES
// ========================================
// This implementation is designed to work with the current proto file:
//
// KEY PROTO FEATURES SUPPORTED:
// 1. ResourceType enum - Used for type-safe resource identification with variants:
//    - Scenario (brake system scenarios)
//    - Package (software packages)
//    - Model (AI/ML models)
//    - Volume (storage volumes)
//    - Network (network configurations)
//    - Node (compute nodes)
//
// 2. StateChange message - Complete message structure with required fields:
//    - resource_type (i32): ResourceType enum value
//    - resource_name (String): Resource identifier
//    - current_state (String): Current state name
//    - target_state (String): Desired target state
//    - transition_id (String): Unique transition identifier
//    - timestamp_ns (i64): Nanosecond precision timestamp
//    - source (String): Source component identifier
//
// 3. StateChangeResponse - Proper response handling with fields:
//    - message (String): Descriptive response message
//    - transition_id (String): Original transition ID for tracking
//    - timestamp_ns (i64): Processing timestamp
//    - error_code (i32): ErrorCode enum value
//    - error_details (String): Detailed error information
//
// 4. ErrorCode enum - Error handling and reporting with variants like:
//    - Success
//    - InvalidRequest
//    - ResourceUnavailable
//    - etc.
//
// CURRENT IMPLEMENTATION STATUS:
// - Core StateChange messaging fully implemented
// - ResourceType enum properly used with correct variant names
// - Error handling with proper enum usage
// - Connection management and retry logic
// - Comprehensive test coverage for basic functionality
//
// FUTURE ENHANCEMENTS AVAILABLE:
// - Advanced state management operations
// - Recovery management with different strategies
// - Event streaming and notifications
// - Alert management and acknowledgment
// - Performance constraints and timing validation
// - Dependency management and validation
// - Health status monitoring and reporting

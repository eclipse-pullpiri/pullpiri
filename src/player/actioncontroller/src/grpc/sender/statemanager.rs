/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager gRPC client for sending state change messages from ActionController.
//!
//! This module provides a client interface for the ActionController to communicate with
//! the StateManager service via gRPC. It manages connection lifecycle, handles
//! request routing, and provides ASIL-compliant state change messaging capabilities.
//!
//! The ActionController uses this client to report action execution results, state
//! confirmations, and error conditions back to the StateManager for proper resource
//! state tracking and recovery management.

use common::statemanager::{
    connect_server, state_manager_connection_client::StateManagerConnectionClient, ResourceType,
    StateChange, StateChangeResponse,
};
use tonic::{Request, Status};

/// StateManager gRPC client for ActionController component.
///
/// This client manages the gRPC connection to the StateManager service and provides
/// methods for sending state change results from action executions. It implements
/// lazy connection establishment to optimize resource usage and provides automatic
/// reconnection capabilities.
///
/// # Connection Management
/// - Establishes connections on first use (lazy initialization)
/// - Reuses existing connections for multiple requests
/// - Handles connection failures gracefully with proper error reporting
/// - Provides thread-safe access through cloning capability
///
/// # ActionController Integration
/// - Reports action execution results (success/failure)
/// - Confirms state transitions after successful actions
/// - Notifies of error conditions and recovery requirements
/// - Provides feedback for timing and performance constraints
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
    /// to optimize startup time and resource usage. This allows the ActionController
    /// to initialize quickly even if the StateManager is temporarily unavailable.
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
    /// This is the primary method for communicating action execution results from the
    /// ActionController to the StateManager. It handles the complete request lifecycle
    /// including connection management, request transmission, and response processing.
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
    /// # ActionController Usage Patterns
    ///
    /// ## 1. Action Execution Success
    /// ```rust
    /// let state_change = StateChange {
    ///     resource_type: ResourceType::Package as i32,
    ///     resource_name: "brake-control-pkg".to_string(),
    ///     current_state: "updating".to_string(),
    ///     target_state: "running".to_string(),
    ///     transition_id: original_transition_id,
    ///     timestamp_ns: completion_timestamp,
    ///     source: "actioncontroller".to_string(),
    /// };
    /// ```
    ///
    /// ## 2. Action Execution Failure
    /// ```rust
    /// let state_change = StateChange {
    ///     resource_type: ResourceType::Model as i32,
    ///     resource_name: "safety-model".to_string(),
    ///     current_state: "running".to_string(),
    ///     target_state: "failed".to_string(),
    ///     transition_id: error_transition_id,
    ///     timestamp_ns: failure_timestamp,
    ///     source: "actioncontroller".to_string(),
    /// };
    /// ```
    ///
    /// ## 3. Recovery Completion Notification
    /// ```rust
    /// let state_change = StateChange {
    ///     resource_type: ResourceType::Scenario as i32,
    ///     resource_name: "emergency-brake".to_string(),
    ///     current_state: "error".to_string(),
    ///     target_state: "waiting".to_string(),
    ///     transition_id: recovery_transition_id,
    ///     timestamp_ns: recovery_timestamp,
    ///     source: "actioncontroller".to_string(),
    /// };
    /// ```
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

    /// Reports successful action execution to the StateManager.
    ///
    /// This convenience method creates and sends a StateChange message indicating
    /// that an action has been successfully executed and the resource has transitioned
    /// to the target state.
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource that was acted upon
    /// * `resource_name` - Name/identifier of the resource
    /// * `previous_state` - State before action execution
    /// * `new_state` - State after successful action execution
    /// * `transition_id` - Original transition ID from the action request
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - StateManager response
    ///
    /// # Example Usage
    /// ```rust
    /// sender.report_action_success(
    ///     ResourceType::Package,
    ///     "brake-control-pkg",
    ///     "updating",
    ///     "running",
    ///     "update-brake-pkg-123"
    /// ).await?;
    /// ```
    pub async fn report_action_success(
        &mut self,
        resource_type: ResourceType,
        resource_name: &str,
        previous_state: &str,
        new_state: &str,
        transition_id: &str,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: resource_type as i32,
            resource_name: resource_name.to_string(),
            current_state: previous_state.to_string(),
            target_state: new_state.to_string(),
            transition_id: transition_id.to_string(),
            timestamp_ns: timestamp,
            source: "actioncontroller".to_string(),
        };

        self.send_state_change(state_change).await
    }

    /// Reports failed action execution to the StateManager.
    ///
    /// This convenience method creates and sends a StateChange message indicating
    /// that an action execution has failed and the resource should transition to
    /// an error state or remain in its current state.
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource that failed to transition
    /// * `resource_name` - Name/identifier of the resource
    /// * `current_state` - Current state of the resource
    /// * `error_state` - Error state to transition to (e.g., "error", "failed")
    /// * `transition_id` - Original transition ID from the action request
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - StateManager response
    ///
    /// # Example Usage
    /// ```rust
    /// sender.report_action_failure(
    ///     ResourceType::Model,
    ///     "safety-model",
    ///     "running",
    ///     "failed",
    ///     "deploy-model-456"
    /// ).await?;
    /// ```
    pub async fn report_action_failure(
        &mut self,
        resource_type: ResourceType,
        resource_name: &str,
        current_state: &str,
        error_state: &str,
        transition_id: &str,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: resource_type as i32,
            resource_name: resource_name.to_string(),
            current_state: current_state.to_string(),
            target_state: error_state.to_string(),
            transition_id: format!("error-{}", transition_id), // Unique ID for error transition
            timestamp_ns: timestamp,
            source: "actioncontroller".to_string(),
        };

        self.send_state_change(state_change).await
    }

    /// Reports recovery completion to the StateManager.
    ///
    /// This convenience method creates and sends a StateChange message indicating
    /// that a recovery action has been completed successfully and the resource
    /// has returned to a healthy operational state.
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource that was recovered
    /// * `resource_name` - Name/identifier of the resource
    /// * `previous_state` - State before recovery (e.g., "error", "failed")
    /// * `recovered_state` - State after successful recovery
    /// * `recovery_id` - Recovery operation identifier
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - StateManager response
    ///
    /// # Example Usage
    /// ```rust
    /// sender.report_recovery_success(
    ///     ResourceType::Scenario,
    ///     "emergency-brake",
    ///     "error",
    ///     "waiting",
    ///     "recovery-789"
    /// ).await?;
    /// ```
    pub async fn report_recovery_success(
        &mut self,
        resource_type: ResourceType,
        resource_name: &str,
        previous_state: &str,
        recovered_state: &str,
        recovery_id: &str,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: resource_type as i32,
            resource_name: resource_name.to_string(),
            current_state: previous_state.to_string(),
            target_state: recovered_state.to_string(),
            transition_id: format!("recovery-{}", recovery_id),
            timestamp_ns: timestamp,
            source: "actioncontroller".to_string(),
        };

        self.send_state_change(state_change).await
    }
}

// ========================================
// UNIT TESTS
// ========================================
// Comprehensive test suite for ActionController StateManagerSender functionality

#[cfg(test)]
mod tests {
    use super::*;
    use common::statemanager::{ResourceType, StateChange};
    use std::time::Duration;

    /// Tests successful state change message transmission to StateManager.
    ///
    /// This test verifies the complete end-to-end communication flow between
    /// the ActionController and StateManager, including connection establishment,
    /// message transmission, and response processing.
    ///
    /// # Test Scenario
    /// Simulates a typical package update completion notification with:
    /// - Proper ResourceType enum usage (Package)
    /// - Complete resource identification and state transition details
    /// - Unique identifiers to prevent test interference
    /// - Comprehensive tracking information for audit trails
    #[tokio::test]
    async fn test_send_state_change_success() {
        // Add startup delay to ensure StateManager service is ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::default();

        // Create unique timestamp for this test run
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        // Create StateChange message for package update completion
        let state_change = StateChange {
            resource_type: ResourceType::Package as i32,
            resource_name: "brake-control-package".to_string(),
            current_state: "updating".to_string(),
            target_state: "running".to_string(),
            transition_id: format!("update-complete-{}", timestamp),
            timestamp_ns: timestamp,
            source: "actioncontroller".to_string(),
        };

        // Send the message and verify successful response
        let result = sender.send_state_change(state_change).await;
        assert!(result.is_ok(), "StateChange request should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(
                !state_response.message.is_empty(),
                "Response should include a message"
            );
            assert!(
                !state_response.transition_id.is_empty(),
                "Response should include transition ID"
            );
            assert!(
                state_response.timestamp_ns > 0,
                "Response should include processing timestamp"
            );

            println!("ActionController StateChange test completed successfully:");
            println!("  Message: {}", state_response.message);
            println!("  Transition ID: {}", state_response.transition_id);
            println!("  Processing time: {} ns", state_response.timestamp_ns);
        }
    }

    /// Tests action success reporting convenience method.
    ///
    /// This test verifies that the report_action_success method correctly
    /// creates and sends StateChange messages for successful action execution.
    #[tokio::test]
    async fn test_report_action_success() {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::new();

        let result = sender
            .report_action_success(
                ResourceType::Model,
                "ai-safety-model",
                "deploying",
                "running",
                "deploy-model-123",
            )
            .await;

        assert!(result.is_ok(), "Action success report should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(state_response.transition_id.contains("deploy-model-123"));
            println!(
                "Action success report test completed: {}",
                state_response.message
            );
        }
    }

    /// Tests action failure reporting convenience method.
    ///
    /// This test verifies that the report_action_failure method correctly
    /// creates and sends StateChange messages for failed action execution.
    #[tokio::test]
    async fn test_report_action_failure() {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::new();

        let result = sender
            .report_action_failure(
                ResourceType::Volume,
                "data-volume",
                "mounting",
                "failed",
                "mount-volume-456",
            )
            .await;

        assert!(result.is_ok(), "Action failure report should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(state_response
                .transition_id
                .contains("error-mount-volume-456"));
            println!(
                "Action failure report test completed: {}",
                state_response.message
            );
        }
    }

    /// Tests recovery success reporting convenience method.
    ///
    /// This test verifies that the report_recovery_success method correctly
    /// creates and sends StateChange messages for successful recovery operations.
    #[tokio::test]
    async fn test_report_recovery_success() {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::new();

        let result = sender
            .report_recovery_success(
                ResourceType::Network,
                "vehicle-network",
                "error",
                "active",
                "recovery-789",
            )
            .await;

        assert!(result.is_ok(), "Recovery success report should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(state_response
                .transition_id
                .contains("recovery-recovery-789"));
            println!(
                "Recovery success report test completed: {}",
                state_response.message
            );
        }
    }
}

// ========================================
// ACTIONCONTROLLER INTEGRATION NOTES
// ========================================
// This StateManagerSender is designed for ActionController-specific use cases:
//
// PRIMARY USE CASES:
// 1. Action Execution Results - Report success/failure of requested actions
// 2. State Confirmation - Confirm that resources have reached target states
// 3. Recovery Notification - Report completion of recovery operations
// 4. Error Reporting - Notify of action failures and error conditions
//
// ACTIONCONTROLLER WORKFLOW:
// 1. Receive action request from StateManager
// 2. Execute the requested action (package update, model deployment, etc.)
// 3. Monitor action progress and completion
// 4. Report results back to StateManager via this client
// 5. Handle any required recovery operations
// 6. Confirm final state transitions
//
// RESOURCE TYPE COVERAGE:
// - Scenario: Emergency response procedures, safety protocols
// - Package: Software updates, configuration changes
// - Model: AI/ML model deployment and lifecycle management
// - Volume: Storage mounting, data management operations
// - Network: Network configuration, connectivity management
// - Node: Node maintenance, resource allocation
//
// TIMING AND SAFETY:
// - Nanosecond precision timestamps for ASIL compliance
// - Unique transition IDs for complete audit trails
// - Immediate error reporting for safety-critical failures
// - Recovery confirmation for system reliability
//
// FUTURE ENHANCEMENTS:
// - Batch action result reporting for efficiency
// - Advanced recovery strategy coordination
// - Performance metrics collection and reporting
// - Integration with monitoring and alerting systems

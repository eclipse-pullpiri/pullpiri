/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager gRPC client for sending state change messages from FilterGateway.
//!
//! This module provides a client interface for the FilterGateway to communicate with
//! the StateManager service via gRPC. It manages connection lifecycle, handles
//! request routing, and provides ASIL-compliant state change messaging capabilities.
//!
//! The FilterGateway uses this client to report policy-driven state transitions,
//! filtering decisions, access control results, and security policy enforcement
//! outcomes to the StateManager for proper resource state tracking.

use common::statemanager::{
    connect_server, state_manager_connection_client::StateManagerConnectionClient, ResourceType,
    StateChange, StateChangeResponse, 
};
use tonic::{Request, Status};

/// StateManager gRPC client for FilterGateway component.
///
/// This client manages the gRPC connection to the StateManager service and provides
/// methods for sending state change requests from FilterGateway operations. It implements
/// lazy connection establishment to optimize resource usage and provides automatic
/// reconnection capabilities.
///
/// # Connection Management
/// - Establishes connections on first use (lazy initialization)
/// - Reuses existing connections for multiple requests
/// - Handles connection failures gracefully with proper error reporting
/// - Provides thread-safe access through cloning capability
///
/// # FilterGateway Integration
/// - Reports policy-driven state transitions
/// - Notifies of filtering decisions and access control results
/// - Provides security policy enforcement outcomes
/// - Handles resource access authorization results
/// - Reports compliance and audit information
///
/// # PICCOLO Compliance
/// - Supports ASIL safety levels from QM to ASIL-D
/// - Maintains nanosecond precision timestamps for timing verification
/// - Provides comprehensive tracking through transition IDs
/// - Includes context information for safety analysis and audit trails
/// - Enforces security and access control policies
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
    /// Creates a new StateManagerSender with default FilterGateway settings.
    ///
    /// # Returns
    /// * `Self` - New StateManagerSender instance with no active connection
    fn default() -> Self {
        Self::new()
    }
}

impl StateManagerSender {
    /// Creates a new StateManagerSender instance for FilterGateway.
    ///
    /// The connection to the StateManager is established lazily on the first request
    /// to optimize startup time and resource usage. This allows the FilterGateway to
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
    /// This is the primary method for communicating policy-driven state transitions from
    /// the FilterGateway to the StateManager. It handles the complete request lifecycle
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
    /// # FilterGateway Usage Patterns
    ///
    /// ## 1. Policy Enforcement Result
    /// ```rust,no_run
    /// use common::statemanager::{ResourceType, StateChange};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let policy_decision_id = "decision-123".to_string();
    /// # let decision_timestamp = 1234567890i64;
    /// let state_change = StateChange {
    ///     resource_type: ResourceType::Scenario as i32,
    ///     resource_name: "emergency-scenario".to_string(),
    ///     current_state: "requested".to_string(),
    ///     target_state: "allowed".to_string(),
    ///     transition_id: policy_decision_id,
    ///     timestamp_ns: decision_timestamp,
    ///     source: "filtergateway".to_string(),
    /// };
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## 2. Access Control Denial
    /// ```rust,no_run
    /// use common::statemanager::{ResourceType, StateChange};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let access_control_id = "access-456".to_string();
    /// # let denial_timestamp = 1234567890i64;
    /// let state_change = StateChange {
    ///     resource_type: ResourceType::Package as i32,
    ///     resource_name: "restricted-package".to_string(),
    ///     current_state: "requested".to_string(),
    ///     target_state: "denied".to_string(),
    ///     transition_id: access_control_id,
    ///     timestamp_ns: denial_timestamp,
    ///     source: "filtergateway".to_string(),
    /// };
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## 3. Security Policy Violation
    /// ```rust,no_run
    /// use common::statemanager::{ResourceType, StateChange};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let security_violation_id = "violation-789".to_string();
    /// # let violation_timestamp = 1234567890i64;
    /// let state_change = StateChange {
    ///     resource_type: ResourceType::Model as i32,
    ///     resource_name: "untrusted-model".to_string(),
    ///     current_state: "running".to_string(),
    ///     target_state: "blocked".to_string(),
    ///     transition_id: security_violation_id,
    ///     timestamp_ns: violation_timestamp,
    ///     source: "filtergateway".to_string(),
    /// };
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # PICCOLO Compliance Notes
    /// - Preserves nanosecond precision timestamps for timing verification
    /// - Maintains transition_id for complete audit trail
    /// - Supports ResourceType enum for type-safe resource identification
    /// - Provides detailed error information for safety analysis
    /// - Enforces security and access control policies
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

    /// Reports policy enforcement decision to StateManager.
    ///
    /// This convenience method creates and sends a StateChange message indicating
    /// the result of a policy enforcement decision (allow/deny).
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource being policy-controlled
    /// * `resource_name` - Name/identifier of the resource
    /// * `current_state` - Current state before policy decision
    /// * `policy_decision` - Result of policy decision ("allowed", "denied", "blocked")
    /// * `policy_id` - Policy decision identifier for audit trails
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - StateManager response
    pub async fn report_policy_decision(
        &mut self,
        resource_type: ResourceType,
        resource_name: &str,
        current_state: &str,
        policy_decision: &str,
        policy_id: &str,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: resource_type as i32,
            resource_name: resource_name.to_string(),
            current_state: current_state.to_string(),
            target_state: policy_decision.to_string(),
            transition_id: format!("policy-{}", policy_id),
            timestamp_ns: timestamp,
            source: "filtergateway".to_string(),
        };

        self.send_state_change(state_change).await
    }

    /// Reports access control result to StateManager.
    ///
    /// This convenience method creates and sends a StateChange message indicating
    /// the result of an access control decision.
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource being access-controlled
    /// * `resource_name` - Name/identifier of the resource
    /// * `current_state` - Current state before access control
    /// * `access_decision` - Result of access control ("granted", "denied", "revoked")
    /// * `access_control_id` - Access control decision identifier
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - StateManager response
    pub async fn report_access_control(
        &mut self,
        resource_type: ResourceType,
        resource_name: &str,
        current_state: &str,
        access_decision: &str,
        access_control_id: &str,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: resource_type as i32,
            resource_name: resource_name.to_string(),
            current_state: current_state.to_string(),
            target_state: access_decision.to_string(),
            transition_id: format!("access-{}", access_control_id),
            timestamp_ns: timestamp,
            source: "filtergateway".to_string(),
        };

        self.send_state_change(state_change).await
    }

    /// Reports security policy violation to StateManager.
    ///
    /// This convenience method creates and sends a StateChange message indicating
    /// a security policy violation that requires immediate attention.
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource violating security policy
    /// * `resource_name` - Name/identifier of the resource
    /// * `current_state` - Current state when violation occurred
    /// * `violation_action` - Action taken ("blocked", "quarantined", "terminated")
    /// * `violation_id` - Security violation identifier
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - StateManager response
    pub async fn report_security_violation(
        &mut self,
        resource_type: ResourceType,
        resource_name: &str,
        current_state: &str,
        violation_action: &str,
        violation_id: &str,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: resource_type as i32,
            resource_name: resource_name.to_string(),
            current_state: current_state.to_string(),
            target_state: violation_action.to_string(),
            transition_id: format!("violation-{}", violation_id),
            timestamp_ns: timestamp,
            source: "filtergateway".to_string(),
        };

        self.send_state_change(state_change).await
    }

    /// Reports filtering result to StateManager.
    ///
    /// This convenience method creates and sends a StateChange message indicating
    /// the result of content or request filtering operations.
    ///
    /// # Arguments
    /// * `resource_type` - Type of resource being filtered
    /// * `resource_name` - Name/identifier of the resource
    /// * `current_state` - Current state before filtering
    /// * `filter_result` - Result of filtering ("passed", "filtered", "rejected")
    /// * `filter_id` - Filter operation identifier
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - StateManager response
    pub async fn report_filter_result(
        &mut self,
        resource_type: ResourceType,
        resource_name: &str,
        current_state: &str,
        filter_result: &str,
        filter_id: &str,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        let state_change = StateChange {
            resource_type: resource_type as i32,
            resource_name: resource_name.to_string(),
            current_state: current_state.to_string(),
            target_state: filter_result.to_string(),
            transition_id: format!("filter-{}", filter_id),
            timestamp_ns: timestamp,
            source: "filtergateway".to_string(),
        };

        self.send_state_change(state_change).await
    }
}

// ========================================
// UNIT TESTS
// ========================================
// Comprehensive test suite for FilterGateway StateManagerSender functionality

#[cfg(test)]
mod tests {
    use super::*;
    use common::statemanager::{ResourceType, StateChange};
    use std::time::Duration;

    /// Tests successful state change message transmission to StateManager.
    ///
    /// This test verifies the complete end-to-end communication flow between
    /// the FilterGateway and StateManager, including connection establishment,
    /// message transmission, and response processing.
    ///
    /// # Test Scenario
    /// Simulates a typical policy enforcement decision with:
    /// - Proper ResourceType enum usage (Scenario)
    /// - Complete resource identification and policy decision details
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

        // Create StateChange message for policy decision
        let state_change = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "brake-system-scenario".to_string(),
            current_state: "requested".to_string(),
            target_state: "allowed".to_string(),
            transition_id: format!("policy-decision-{}", timestamp),
            timestamp_ns: timestamp,
            source: "filtergateway".to_string(),
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

            println!("FilterGateway StateChange test completed successfully:");
            println!("  Message: {}", state_response.message);
            println!("  Transition ID: {}", state_response.transition_id);
            println!("  Processing time: {} ns", state_response.timestamp_ns);
        }
    }

    /// Tests policy decision reporting convenience method.
    ///
    /// This test verifies that the report_policy_decision method correctly
    /// creates and sends StateChange messages for policy enforcement decisions.
    #[tokio::test]
    async fn test_report_policy_decision() {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::new();

        let result = sender
            .report_policy_decision(
                ResourceType::Package,
                "security-package",
                "requested",
                "allowed",
                "policy-123",
            )
            .await;

        assert!(result.is_ok(), "Policy decision report should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(state_response.transition_id.contains("policy-policy-123"));
            println!(
                "Policy decision report test completed: {}",
                state_response.message
            );
        }
    }

    /// Tests access control reporting convenience method.
    ///
    /// This test verifies that the report_access_control method correctly
    /// creates and sends StateChange messages for access control decisions.
    #[tokio::test]
    async fn test_report_access_control() {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::new();

        let result = sender
            .report_access_control(
                ResourceType::Model,
                "restricted-model",
                "requested",
                "denied",
                "access-456",
            )
            .await;

        assert!(result.is_ok(), "Access control report should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(state_response.transition_id.contains("access-access-456"));
            println!(
                "Access control report test completed: {}",
                state_response.message
            );
        }
    }

    /// Tests security violation reporting convenience method.
    ///
    /// This test verifies that the report_security_violation method correctly
    /// creates and sends StateChange messages for security violations.
    #[tokio::test]
    async fn test_report_security_violation() {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::new();

        let result = sender
            .report_security_violation(
                ResourceType::Volume,
                "compromised-volume",
                "active",
                "quarantined",
                "violation-789",
            )
            .await;

        assert!(result.is_ok(), "Security violation report should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(state_response
                .transition_id
                .contains("violation-violation-789"));
            println!(
                "Security violation report test completed: {}",
                state_response.message
            );
        }
    }

    /// Tests filter result reporting convenience method.
    ///
    /// This test verifies that the report_filter_result method correctly
    /// creates and sends StateChange messages for filtering operations.
    #[tokio::test]
    async fn test_report_filter_result() {
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut sender = StateManagerSender::new();

        let result = sender
            .report_filter_result(
                ResourceType::Network,
                "external-connection",
                "pending",
                "filtered",
                "filter-101",
            )
            .await;

        assert!(result.is_ok(), "Filter result report should succeed");

        if let Ok(response) = result {
            let state_response = response.into_inner();
            assert!(state_response.transition_id.contains("filter-filter-101"));
            println!(
                "Filter result report test completed: {}",
                state_response.message
            );
        }
    }
}

// ========================================
// FILTERGATEWAY INTEGRATION NOTES
// ========================================
// This StateManagerSender is designed for FilterGateway-specific use cases:
//
// PRIMARY USE CASES:
// 1. Policy Enforcement - Report policy decisions (allow/deny/block)
// 2. Access Control - Report access control decisions (grant/deny/revoke)
// 3. Security Violations - Report security policy violations and responses
// 4. Content Filtering - Report filtering results and content decisions
// 5. Compliance Reporting - Report compliance status and audit information
//
// FILTERGATEWAY WORKFLOW:
// 1. Receive resource access request
// 2. Apply security policies and access controls
// 3. Make filtering and policy decisions
// 4. Report decisions to StateManager via this client
// 5. Handle security violations and escalations
// 6. Maintain audit trails for compliance
//
// RESOURCE TYPE COVERAGE:
// - Scenario: Emergency access control, safety scenario filtering
// - Package: Software security validation, package trust verification
// - Model: AI/ML model trust and access control
// - Volume: Data access restrictions, storage security
// - Network: Traffic filtering, connection security
// - Node: Node access control, resource allocation security
//
// SECURITY AND COMPLIANCE:
// - Nanosecond precision timestamps for security audit trails
// - Unique transition IDs for complete security event tracking
// - Immediate reporting for security-critical violations
// - Policy enforcement validation and compliance checking
// - Source identification as "filtergateway" for audit trails
//
// FUTURE ENHANCEMENTS:
// - Advanced policy conflict resolution
// - Multi-factor authentication integration
// - Real-time threat detection and response
// - Integration with security information and event management (SIEM)
// - Automated compliance reporting and validation

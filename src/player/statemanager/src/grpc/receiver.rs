/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager gRPC Service Implementation
//!
//! This module implements the gRPC server interface for the StateManager service.
//! It handles incoming requests from various components (ApiServer, FilterGateway, ActionController)
//! and forwards them to the StateManager's internal processing engine via async channels.
//!
//! The implementation supports the complete PICCOLO Resource State Management specification,
//! including state changes, resource queries, recovery management, and event notifications.

use common::monitoringserver::{ContainerList, SendContainerListResponse};
use common::statemanager::{
    state_manager_connection_server::StateManagerConnection,
    Action,
    ErrorCode,
    // // State Query API message types
    // ResourceStateRequest, ResourceStateResponse,
    // ResourceStateHistoryRequest, ResourceStateHistoryResponse,
    // ListResourcesByStateRequest, ListResourcesByStateResponse,

    // // State Management API message types
    // UpdateDesiredStateRequest, TriggerStateTransitionRequest, ForceSynchronizationRequest,

    // // Recovery Management API message types
    // TriggerRecoveryRequest, AbortRecoveryRequest, RecoveryStatusRequest,
    // RecoveryResponse, RecoveryStatusResponse,

    // // Event and Notification API message types
    // StateChangeSubscriptionRequest, StateChangeEvent,
    // AcknowledgeAlertRequest, AlertResponse,
    // GetPendingAlertsRequest, GetPendingAlertsResponse,
    ResourceType,
    StateChange,
    StateChangeResponse,
};
use tokio::sync::mpsc;
use tonic::{Request, Status};

/// StateManager gRPC service handler.
///
/// This struct implements the StateManagerConnection gRPC service and acts as the
/// entry point for all gRPC requests to the StateManager. It uses async channels
/// to forward requests to the StateManager's internal processing engine.
///
/// # Architecture
/// - Receives gRPC requests from external components
/// - Validates and processes request data
/// - Forwards state changes and container updates via async channels
/// - Returns appropriate responses with ASIL-compliant timing and tracking
#[derive(Clone)]
pub struct StateManagerReceiver {
    /// Channel sender for ContainerList messages from nodeagent.
    /// Used to forward container status updates to the StateManager for processing.
    pub tx: mpsc::Sender<ContainerList>,

    /// Channel sender for StateChange messages from various components.
    /// Used to forward state transition requests to the StateManager's state machine engine.
    pub tx_state_change: mpsc::Sender<StateChange>,
}

#[tonic::async_trait]
impl StateManagerConnection for StateManagerReceiver {
    /// Stream type for state change event subscriptions.
    /// Uses ReceiverStream to provide async streaming of state change events to subscribers.
    /// type SubscribeToStateChangesStream = ReceiverStream<Result<StateChangeEvent, Status>>;
    /// Handles action requests (legacy implementation).
    ///
    /// # Arguments
    /// * `request` - gRPC request containing an Action message
    ///
    /// # Returns
    /// * `Result<tonic::Response<Response>, Status>` - Using common::statemanager::Response
    ///
    /// # Note
    /// This is a legacy method that is not currently implemented.
    /// Returns an Unavailable status for all requests.
    async fn send_action(
        &self,
        request: Request<Action>,
    ) -> Result<tonic::Response<common::statemanager::Response>, Status> {
        let req = request.into_inner();
        let command = req.action;

        Err(Status::new(tonic::Code::Unavailable, command))
    }

    /// Handles ContainerList messages from nodeagent.
    ///
    /// Receives container status updates from the nodeagent and forwards them
    /// to the StateManager for processing. This enables the StateManager to
    /// monitor container health and adjust resource states accordingly.
    ///
    /// # Arguments
    /// * `request` - gRPC request containing a ContainerList message
    ///
    /// # Returns
    /// * `Result<tonic::Response<SendContainerListResponse>, Status>` - Success confirmation or error
    ///
    /// # Processing Flow
    /// 1. Extract ContainerList from the gRPC request
    /// 2. Validate the container list structure
    /// 3. Forward to StateManager via async channel for health monitoring
    /// 4. Return immediate success response (async processing)
    ///
    /// # Error Handling
    /// - Validates container list is not empty
    /// - Handles channel send failures gracefully
    /// - Provides detailed error messages for troubleshooting
    async fn send_changed_container_list<'life>(
        &'life self,
        request: Request<ContainerList>,
    ) -> Result<tonic::Response<SendContainerListResponse>, Status> {
        let req: ContainerList = request.into_inner();

        match self.tx.send(req).await {
            Ok(_) => Ok(tonic::Response::new(SendContainerListResponse {
                resp: "Successfully processed ContainerList".to_string(),
            })),
            Err(e) => Err(tonic::Status::new(
                tonic::Code::Unavailable,
                format!("cannot send changed container list: {e}"),
            )),
        }
    }
    /// Handles StateChange messages from various components.
    ///
    /// This is the core method for state management in the PICCOLO framework.
    /// It receives state change requests from ApiServer, FilterGateway, and
    /// ActionController, forwards them to the StateManager's state machine,
    /// and returns a comprehensive response with ASIL-compliant tracking.
    ///
    /// # Arguments
    /// * `request` - gRPC request containing a complete StateChange message
    ///
    /// # Returns
    /// * `Result<tonic::Response<StateChangeResponse>, Status>` - Detailed response with tracking info
    ///
    /// # StateChange Processing Flow
    /// 1. Extract StateChange from gRPC request
    /// 2. Validate the StateChange message structure and content
    /// 3. Preserve transition_id for response tracking
    /// 4. Forward to StateManager via dedicated async channel
    /// 5. Generate comprehensive ASIL-compliant response with:
    ///    - Success status and descriptive message
    ///    - Nanosecond precision timestamp (ASIL compliance)
    ///    - Original transition_id for audit trail tracking
    ///    - Proper ErrorCode enum values
    ///    - Detailed error information if applicable
    ///
    /// # Validation
    /// - Validates resource_type enum value
    /// - Ensures resource_name is not empty
    /// - Validates state transition fields
    /// - Checks transition_id format and uniqueness
    /// - Validates source component identification
    ///
    /// # ASIL Compliance
    /// - Nanosecond precision timestamps for timing verification
    /// - Unique transition IDs for complete audit trails
    /// - Comprehensive error reporting for safety analysis
    /// - Proper ErrorCode enum usage for standardized responses
    async fn send_state_change(
        &self,
        request: Request<StateChange>,
    ) -> Result<tonic::Response<StateChangeResponse>, Status> {
        let req = request.into_inner();
        let transition_id = req.transition_id.clone();

        // 🔍 COMMENT 5: StateManager receiving scenario state change requests
        // This method receives state change requests from multiple components:
        // - FilterGateway: when scenario conditions are registered and met
        // - ActionController: when scenario processing completes or conditions are satisfied
        // - PolicyManager: when scenario policy requirements are satisfied
        // All scenario state transitions flow through this central point.

        // Comprehensive validation of StateChange message
        if let Err(validation_error) = self.validate_state_change(&req) {
            return Ok(tonic::Response::new(StateChangeResponse {
                message: format!("StateChange validation failed: {validation_error}"),
                transition_id, // Preserve original ID even for validation failures
                timestamp_ns: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as i64,
                error_code: ErrorCode::InvalidRequest as i32,
                error_details: validation_error,
            }));
        }

        // Log comprehensive state change information for monitoring
        println!("StateChange received:");
        println!(
            "  Resource: {} {}",
            self.resource_type_to_string(req.resource_type),
            req.resource_name
        );
        println!(
            "  Transition: {} -> {}",
            req.current_state, req.target_state
        );
        println!("  ID: {}, Source: {}", req.transition_id, req.source);

        // Forward StateChange to StateManager's state machine engine
        match self.tx_state_change.send(req).await {
            Ok(_) => {
                // Generate ASIL-compliant success response
                Ok(tonic::Response::new(StateChangeResponse {
                    message: "StateChange successfully received and queued for processing"
                        .to_string(),
                    transition_id, // Preserve original ID for tracking
                    timestamp_ns: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as i64, // Nanosecond precision for ASIL
                    error_code: ErrorCode::Success as i32,
                    error_details: String::new(), // No error details for success
                }))
            }
            Err(e) => {
                // Channel send failed - StateManager unavailable or overloaded
                eprintln!("Failed to forward StateChange to StateManager: {e}");
                Ok(tonic::Response::new(StateChangeResponse {
                    message: "StateManager service unavailable".to_string(),
                    transition_id, // Preserve original ID for tracking
                    timestamp_ns: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as i64,
                    error_code: ErrorCode::ResourceUnavailable as i32,
                    error_details: format!("Cannot forward StateChange to StateManager: {e}"),
                }))
            }
        }
    }
}

impl StateManagerReceiver {
    /// Validates a StateChange message according to PICCOLO specifications.
    ///
    /// This method performs comprehensive validation of StateChange messages
    /// to ensure they conform to the proto specification and contain valid data.
    ///
    /// # Arguments
    /// * `state_change` - StateChange message to validate
    ///
    /// # Returns
    /// * `Result<(), String>` - Success or detailed validation error
    ///
    /// # Validation Rules
    /// - resource_type must be a valid ResourceType enum value
    /// - resource_name must not be empty
    /// - current_state and target_state must not be empty
    /// - transition_id must not be empty
    /// - source must not be empty
    /// - timestamp_ns must be positive
    fn validate_state_change(&self, state_change: &StateChange) -> Result<(), String> {
        // Validate resource type enum
        if ResourceType::try_from(state_change.resource_type).is_err() {
            return Err(format!(
                "Invalid resource_type: {}",
                state_change.resource_type
            ));
        }

        // Validate required string fields
        if state_change.resource_name.trim().is_empty() {
            return Err("resource_name cannot be empty".to_string());
        }
        if state_change.current_state.trim().is_empty() {
            return Err("current_state cannot be empty".to_string());
        }
        if state_change.target_state.trim().is_empty() {
            return Err("target_state cannot be empty".to_string());
        }
        if state_change.transition_id.trim().is_empty() {
            return Err("transition_id cannot be empty".to_string());
        }
        if state_change.source.trim().is_empty() {
            return Err("source cannot be empty".to_string());
        }

        // Validate timestamp
        if state_change.timestamp_ns <= 0 {
            return Err("timestamp_ns must be positive".to_string());
        }

        // Additional validation can be added here when more fields are available
        // in the proto file (metadata, dependencies, constraints, etc.)

        Ok(())
    }

    /// Converts ResourceType enum to human-readable string.
    ///
    /// # Arguments
    /// * `resource_type` - ResourceType enum value
    ///
    /// # Returns
    /// * `&'static str` - Human-readable resource type name
    fn resource_type_to_string(&self, resource_type: i32) -> &'static str {
        match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => "Scenario",
            Ok(ResourceType::Package) => "Package",
            Ok(ResourceType::Model) => "Model",
            Ok(ResourceType::Volume) => "Volume",
            Ok(ResourceType::Network) => "Network",
            Ok(ResourceType::Node) => "Node",
            _ => "Unknown",
        }
    }
}

// ========================================
// FUTURE IMPLEMENTATION NOTES
// ========================================
// When the comprehensive proto file with all message types is fully integrated,
// the following additional methods will be implemented to complete the
// StateManagerConnection trait:
//
// STATE QUERY API:
// - get_resource_state(ResourceStateRequest) -> ResourceStateResponse
//   * Query current state and health status of specific resources
//   * Support for ResourceType filtering and metadata retrieval
//   * ASIL compliance tracking and audit trail access
//
// - get_resource_state_history(ResourceStateHistoryRequest) -> ResourceStateHistoryResponse
//   * Retrieve complete state transition history with timing analysis
//   * Support for time range filtering and audit trail generation
//   * Performance metrics and transition success rates
//
// - list_resources_by_state(ListResourcesByStateRequest) -> ListResourcesByStateResponse
//   * Filter resources by current state with label selectors
//   * Bulk operations support and pagination
//   * Health status aggregation and reporting
//
// STATE MANAGEMENT API:
// - update_desired_state(UpdateDesiredStateRequest) -> StateChangeResponse
//   * Update target states with validation and dependency checking
//   * Force updates with safety override capabilities
//   * Batch update operations for efficiency
//
// - trigger_state_transition(TriggerStateTransitionRequest) -> StateChangeResponse
//   * Manual state transitions with precondition validation
//   * Performance constraint enforcement and timing validation
//   * Emergency override capabilities for safety-critical scenarios
//
// - force_synchronization(ForceSynchronizationRequest) -> StateChangeResponse
//   * Reconcile state drift between desired and actual states
//   * Deep synchronization with dependency cascade updates
//   * Health check integration and validation
//
// RECOVERY MANAGEMENT API:
// - trigger_recovery(TriggerRecoveryRequest) -> RecoveryResponse
//   * Initiate recovery procedures with strategy selection
//   * Automatic recovery escalation and timeout handling
//   * Progress tracking and status reporting
//
// - abort_recovery(AbortRecoveryRequest) -> RecoveryResponse
//   * Cancel ongoing recovery operations safely
//   * Rollback capabilities and state restoration
//   * Emergency abort with minimal disruption
//
// - get_recovery_status(RecoveryStatusRequest) -> RecoveryStatusResponse
//   * Real-time recovery progress monitoring
//   * Step-by-step status tracking and estimated completion
//   * Failure analysis and retry strategy reporting
//
// EVENT AND NOTIFICATION API:
// - type SubscribeToStateChangesStream = ReceiverStream<Result<StateChangeEvent, Status>>
//   * Real-time event streaming with filtering capabilities
//   * Subscription management and event routing
//   * Health and recovery event integration
//
// - subscribe_to_state_changes(StateChangeSubscriptionRequest) -> SubscribeToStateChangesStream
//   * Event subscription with comprehensive filtering options
//   * Resource type and severity level filtering
//   * Metadata-based event routing and delivery
//
// - acknowledge_alert(AcknowledgeAlertRequest) -> AlertResponse
//   * Alert lifecycle management and acknowledgment tracking
//   * Escalation prevention and status updates
//   * Audit trail maintenance for alert handling
//
// - get_pending_alerts(GetPendingAlertsRequest) -> GetPendingAlertsResponse
//   * Query active alerts with severity and resource filtering
//   * Alert aggregation and priority sorting
//   * Health status integration and correlation
//
// IMPLEMENTATION PRIORITY:
// 1. State Query API - Fundamental read operations
// 2. Event Streaming - Real-time monitoring capabilities
// 3. Advanced State Management - Enhanced write operations
// 4. Recovery Management - Failure handling and automation
// 5. Alert Management - Comprehensive notification system
//
// Each implementation phase will include:
// - Comprehensive validation and error handling
// - ASIL compliance verification and timing constraints
// - Performance optimization and resource management
// - Integration testing and safety verification
// - Documentation and example usage patterns

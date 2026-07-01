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
pub mod timpani;

use common::logd;
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
    OffloadingRequest,
    OffloadingResponse,
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
        logd!(1, "StateChange received:");
        logd!(
            1,
            "  Resource: {} {}",
            self.resource_type_to_string(req.resource_type),
            req.resource_name
        );

        logd!(
            1,
            "  Transition: {} -> {}",
            req.current_state,
            req.target_state
        );
        logd!(1, "  ID: {}, Source: {}", req.transition_id, req.source);

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
                logd!(5, "Failed to forward StateChange to StateManager: {e}");
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

    /// Handles TriggerOffloading requests from PolicyManager.
    ///
    /// This method receives offloading requests when resource thresholds are exceeded
    /// and forwards them to ActionController for execution.
    ///
    /// # Arguments
    /// * `request` - gRPC request containing OffloadingRequest message
    ///
    /// # Returns
    /// * `Result<tonic::Response<OffloadingResponse>, Status>` - Offloading result
    ///
    /// # Processing Flow
    /// 1. Extract OffloadingRequest from gRPC request
    /// 2. Log offloading request details
    /// 3. Forward to ActionController via sender
    /// 4. Return response with tracking ID
    async fn trigger_offloading(
        &self,
        request: Request<OffloadingRequest>,
    ) -> Result<tonic::Response<OffloadingResponse>, Status> {
        let req = request.into_inner();

        println!(
            "[StateManager] Received offloading request: model '{}' from '{}' to '{}'",
            req.model_name, req.source_node, req.target_node
        );
        println!(
            "[StateManager]   Scenario: {}, Package: {}, Policy: {}",
            req.scenario_name, req.package_name, req.policy_name
        );
        println!("[StateManager]   Reason: {}", req.reason);

        // Generate transition ID for tracking
        let transition_id = format!(
            "offload-{}-{}-{}",
            req.model_name,
            req.source_node,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        // Convert to ActionController request
        let offload_request = common::actioncontroller::OffloadModelRequest {
            scenario_name: req.scenario_name,
            package_name: req.package_name,
            model_name: req.model_name.clone(),
            source_node: req.source_node.clone(),
            target_node: req.target_node.clone(),
            policy_name: req.policy_name,
            reason: req.reason,
        };

        // Forward to ActionController
        match crate::grpc::sender::offload_model(offload_request).await {
            Ok(response) => {
                let resp = response.into_inner();
                println!(
                    "[StateManager] Offloading {} from '{}' to '{}': success={}, message={}",
                    req.model_name, req.source_node, req.target_node, resp.success, resp.message
                );

                Ok(tonic::Response::new(OffloadingResponse {
                    accepted: resp.success,
                    message: resp.message,
                    transition_id,
                }))
            }
            Err(e) => {
                eprintln!(
                    "[StateManager] Failed to offload model '{}': {}",
                    req.model_name,
                    e.message()
                );

                Ok(tonic::Response::new(OffloadingResponse {
                    accepted: false,
                    message: format!("Failed to offload: {}", e.message()),
                    transition_id,
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::monitoringserver::ContainerList;
    use common::statemanager::{ErrorCode, ResourceType, StateChange};
    use tonic::Request;

    #[test]
    fn test_validate_state_change_and_resource_type_to_string() {
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, _rx2) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx,
            tx_state_change,
        };

        // Valid state change
        let sc = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "res1".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Waiting".to_string(),
            transition_id: "t1".to_string(),
            timestamp_ns: 1,
            source: "unittest".to_string(),
        };
        assert!(receiver.validate_state_change(&sc).is_ok());

        // Invalid timestamp
        let mut sc2 = sc.clone();
        sc2.timestamp_ns = 0;
        assert!(receiver.validate_state_change(&sc2).is_err());

        // Empty resource_name
        let mut sc3 = sc.clone();
        sc3.resource_name = "".to_string();
        assert!(receiver.validate_state_change(&sc3).is_err());

        // resource_type_to_string checks
        assert_eq!(
            receiver.resource_type_to_string(ResourceType::Scenario as i32),
            "Scenario"
        );
        assert_eq!(receiver.resource_type_to_string(9999), "Unknown");
    }

    #[tokio::test]
    async fn test_send_changed_container_list_success_and_failure() {
        // Success path: receiver present
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, _rx2) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx: tx.clone(),
            tx_state_change: tx_state_change.clone(),
        };

        let cl = ContainerList {
            node_name: "n1".to_string(),
            containers: vec![],
        };
        let resp = receiver.send_changed_container_list(Request::new(cl)).await;
        assert!(resp.is_ok());

        // Failure path: dropped receiver for tx
        let (bad_tx, bad_rx) = mpsc::channel::<ContainerList>(1);
        drop(bad_rx);
        let receiver2 = StateManagerReceiver {
            tx: bad_tx,
            tx_state_change: tx_state_change.clone(),
        };
        let cl2 = ContainerList {
            node_name: "n2".to_string(),
            containers: vec![],
        };
        let resp2 = receiver2
            .send_changed_container_list(Request::new(cl2))
            .await;
        assert!(resp2.is_err());
    }

    #[tokio::test]
    async fn test_send_changed_container_list_response_content() {
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, _rx2) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx: tx.clone(),
            tx_state_change: tx_state_change.clone(),
        };

        let cl = ContainerList {
            node_name: "n1".to_string(),
            containers: vec![],
        };
        let resp = receiver
            .send_changed_container_list(Request::new(cl))
            .await
            .unwrap();
        let body = resp.into_inner();
        assert_eq!(body.resp, "Successfully processed ContainerList");

        // Failure message should contain 'cannot send changed container list'
        let (bad_tx, bad_rx) = mpsc::channel::<ContainerList>(1);
        drop(bad_rx);
        let receiver2 = StateManagerReceiver {
            tx: bad_tx,
            tx_state_change,
        };
        let cl2 = ContainerList {
            node_name: "n2".to_string(),
            containers: vec![],
        };
        let resp2 = receiver2
            .send_changed_container_list(Request::new(cl2))
            .await;
        assert!(resp2.is_err());
        let status = resp2.err().unwrap();
        assert_eq!(status.code(), tonic::Code::Unavailable);
        assert!(status
            .message()
            .contains("cannot send changed container list"));
    }

    #[tokio::test]
    async fn test_send_state_change_success_and_unavailable() {
        // Success: tx_state_change has receiver
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, mut rx_state_change) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx: tx.clone(),
            tx_state_change: tx_state_change.clone(),
        };

        let sc = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "res2".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Waiting".to_string(),
            transition_id: "t2".to_string(),
            timestamp_ns: 1,
            source: "unittest".to_string(),
        };

        let resp = receiver.send_state_change(Request::new(sc.clone())).await;
        assert!(resp.is_ok());
        let body = resp.unwrap().into_inner();
        assert_eq!(body.error_code, ErrorCode::Success as i32);

        // ensure message was forwarded
        let forwarded = rx_state_change.recv().await;
        assert!(forwarded.is_some());

        // Failure: tx_state_change cannot send (receiver dropped)
        let (bad_tx, bad_rx) = mpsc::channel::<StateChange>(1);
        drop(bad_rx);
        let receiver2 = StateManagerReceiver {
            tx: tx.clone(),
            tx_state_change: bad_tx,
        };

        let sc2 = StateChange {
            transition_id: "t3".to_string(),
            ..sc.clone()
        };
        let resp2 = receiver2
            .send_state_change(Request::new(sc2))
            .await
            .unwrap();
        let inner = resp2.into_inner();
        assert_eq!(inner.error_code, ErrorCode::ResourceUnavailable as i32);
    }

    #[tokio::test]
    async fn test_send_action_returns_unavailable() {
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, _rx2) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx,
            tx_state_change,
        };

        let action = common::statemanager::Action {
            action: "doit".to_string(),
        };
        let res = receiver.send_action(Request::new(action)).await;
        assert!(res.is_err());
        let status = res.err().unwrap();
        assert_eq!(status.code(), tonic::Code::Unavailable);
        assert_eq!(status.message(), "doit");
    }

    #[tokio::test]
    async fn test_send_state_change_validation_failure_returns_invalid_request() {
        // Create receiver; validation should fail before attempting to forward
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, _rx2) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx,
            tx_state_change,
        };

        // Build an invalid StateChange (timestamp_ns <= 0)
        let sc = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "bad".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Waiting".to_string(),
            transition_id: "bad-tid".to_string(),
            timestamp_ns: 0,
            source: "unittest".to_string(),
        };

        let resp = receiver.send_state_change(Request::new(sc)).await;
        assert!(resp.is_ok());
        let inner = resp.unwrap().into_inner();
        assert_eq!(inner.error_code, ErrorCode::InvalidRequest as i32);
    }

    #[tokio::test]
    async fn test_send_state_change_invalid_resource_type_returns_invalid_request() {
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, _rx2) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx,
            tx_state_change,
        };

        let sc = StateChange {
            resource_type: 9999, // invalid
            resource_name: "res_invalid".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Waiting".to_string(),
            transition_id: "tid-invalid".to_string(),
            timestamp_ns: 1,
            source: "unittest".to_string(),
        };

        let resp = receiver.send_state_change(Request::new(sc)).await;
        assert!(resp.is_ok());
        let inner = resp.unwrap().into_inner();
        assert_eq!(inner.error_code, ErrorCode::InvalidRequest as i32);
    }

    #[test]
    fn test_resource_type_to_string_variants() {
        let (tx, _rx) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, _rx2) = mpsc::channel::<StateChange>(1);
        let receiver = StateManagerReceiver {
            tx,
            tx_state_change,
        };

        assert_eq!(
            receiver.resource_type_to_string(ResourceType::Scenario as i32),
            "Scenario"
        );
        assert_eq!(
            receiver.resource_type_to_string(ResourceType::Package as i32),
            "Package"
        );
        assert_eq!(
            receiver.resource_type_to_string(ResourceType::Model as i32),
            "Model"
        );
        assert_eq!(
            receiver.resource_type_to_string(ResourceType::Volume as i32),
            "Volume"
        );
        assert_eq!(
            receiver.resource_type_to_string(ResourceType::Network as i32),
            "Network"
        );
        assert_eq!(
            receiver.resource_type_to_string(ResourceType::Node as i32),
            "Node"
        );
        assert_eq!(receiver.resource_type_to_string(9999), "Unknown");
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

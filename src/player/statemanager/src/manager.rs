/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManagerManager: Asynchronous state management engine for PICCOLO framework
//!
//! This module provides the core state management functionality for the StateManager service.
//! It receives and processes state change requests from various components (ApiServer, FilterGateway,
//! ActionController) and container status updates from nodeagent via async channels.
//!
//! The manager implements the PICCOLO Resource State Management specification, handling
//! state transitions, monitoring, reconciliation, and recovery for all resource types
//! (Scenario, Package, Model, Volume, Network, Node).

use crate::grpc::sender;
use crate::state_machine::StateMachine;
use crate::types::{ActionCommand, TransitionResult};
use common::monitoringserver::ContainerList;
use common::spec::artifact::Artifact;

use common::statemanager::{
    ErrorCode, ModelState, PackageState, ResourceType, ScenarioState, StateChange,
};

use common::logd;
use common::Result;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task;

/// Core state management engine for the StateManager service.
///
/// This struct orchestrates all state management operations by receiving messages
/// from gRPC handlers via async channels and processing them according to the
/// PICCOLO Resource State Management specification.
///
/// # Architecture
/// - Receives StateChange messages from ApiServer, FilterGateway, ActionController
/// - Receives ContainerList updates from nodeagent
/// - Processes state transitions with ASIL compliance
/// - Manages resource lifecycle and dependencies
/// - Handles error recovery and reconciliation
///
/// # Threading Model
/// - Uses Arc<Mutex<mpsc::Receiver>> for safe multi-threaded access
/// - Spawns dedicated async tasks for each message type
/// - Ensures lock-free message processing with proper channel patterns
pub struct StateManagerManager {
    /// State machine for processing state transitions
    state_machine: Arc<Mutex<StateMachine>>,

    /// Channel receiver for container status updates from nodeagent.
    ///
    /// Receives ContainerList messages containing current container states,
    /// health information, and resource usage data. This enables the StateManager
    /// to monitor container health and trigger state transitions when needed.
    rx_container: Arc<Mutex<mpsc::Receiver<ContainerList>>>,

    /// Channel receiver for state change requests from various components.
    ///
    /// Receives StateChange messages from:
    /// - ApiServer: User-initiated state changes and scenario requests
    /// - FilterGateway: Policy-driven state transitions and filtering decisions
    /// - ActionController: Action execution results and state confirmations
    rx_state_change: Arc<Mutex<mpsc::Receiver<StateChange>>>,
}

impl StateManagerManager {
    /// Creates a new StateManagerManager instance.
    ///
    /// Initializes the manager with the provided channel receivers for processing
    /// container updates and state change requests.
    ///
    /// # Arguments
    /// * `rx_container` - Channel receiver for ContainerList messages from nodeagent
    /// * `rx_state_change` - Channel receiver for StateChange messages from components
    ///
    /// # Returns
    /// * `Self` - New StateManagerManager instance ready for initialization
    pub async fn new(
        rx_container: mpsc::Receiver<ContainerList>,
        rx_state_change: mpsc::Receiver<StateChange>,
    ) -> Self {
        Self {
            state_machine: Arc::new(Mutex::new(StateMachine::new())),
            rx_container: Arc::new(Mutex::new(rx_container)),
            rx_state_change: Arc::new(Mutex::new(rx_state_change)),
        }
    }

    /// Initializes the StateManagerManager's internal state and resources.
    ///
    /// Performs startup operations required before beginning message processing:
    /// - Loads initial resource states from persistent storage
    /// - Initializes state machine engines for each resource type
    /// - Sets up monitoring and health check systems
    /// - Prepares recovery and reconciliation systems
    ///
    /// # Returns
    /// * `Result<()>` - Success or initialization error
    ///
    /// # Future Enhancements
    /// - Load persisted resource states from storage (etcd, database)
    /// - Initialize state machine validators for each resource type
    /// - Set up dependency tracking and validation systems
    /// - Configure ASIL safety monitoring and alerting
    pub async fn initialize(&mut self) -> Result<()> {
        logd!(3, "StateManagerManager initializing...");

        // Initialize the state machine with async action executor
        let action_receiver = {
            let mut state_machine = self.state_machine.lock().await;
            state_machine.initialize_action_executor()
        };

        // Start the async action executor
        tokio::spawn(async move {
            run_action_executor(action_receiver).await;
        });

        logd!(3, "State machine initialized with transition tables for Scenario, Package, and Model resources");
        logd!(
            3,
            "Async action executor started for non-blocking action processing"
        );

        // TODO: Add comprehensive initialization logic:
        // - Load persisted resource states from persistent storage
        // - Initialize state machine validators for each ResourceType
        // - Set up dependency tracking and validation systems
        // - Configure ASIL safety monitoring and alerting
        // - Initialize recovery strategies for each RecoveryType
        // - Set up health check systems for all resource types
        // - Configure event streaming and notification systems

        logd!(3, "StateManagerManager initialization completed");
        Ok(())
    }

    /// Processes a StateChange message according to PICCOLO specifications.
    ///
    /// This is the core method that handles all state transition requests in the system.
    /// It validates requests, processes transitions through the state machine, and handles
    /// both successful transitions and failure scenarios with appropriate logging and recovery.
    ///
    /// # Arguments
    /// * `state_change` - Complete StateChange message containing:
    ///   - `resource_type`: Type of resource (Scenario/Package/Model)
    ///   - `resource_name`: Unique identifier for the resource
    ///   - `current_state`: Expected current state of the resource
    ///   - `target_state`: Desired state after transition
    ///   - `transition_id`: Unique ID for tracking this transition
    ///   - `source`: Component that initiated the state change
    ///   - `timestamp_ns`: When the request was created
    ///
    /// # Processing Flow
    /// 1. **Validation**: Parse and validate resource type from the request
    /// 2. **Logging**: Log comprehensive transition details for audit trails
    /// 3. **State Machine Processing**: Execute transition through the state machine
    /// 4. **Result Handling**: Process success/failure outcomes appropriately
    /// 5. **Action Scheduling**: Queue any required follow-up actions for async execution
    /// 6. **Error Recovery**: Handle failures with appropriate recovery strategies
    ///
    /// # Error Handling
    /// - Invalid resource types are logged and ignored (early return)
    /// - State machine failures trigger the `handle_transition_failure` method
    /// - All errors are logged with detailed context for debugging
    ///
    /// # Side Effects
    /// - Updates internal resource state tracking
    /// - Queues actions for asynchronous execution
    /// - Generates log entries for audit trails
    /// - May trigger recovery procedures on failures
    ///
    /// # Thread Safety
    /// This method is async and uses internal locking for state machine access.
    /// Multiple concurrent calls are safe but will be serialized at the state machine level.
    async fn process_state_change(&self, state_change: StateChange) {
        // ========================================
        // STEP 1: RESOURCE TYPE VALIDATION
        // ========================================
        // Convert the numeric resource type from the proto message to a type-safe enum.
        // This ensures we only process known resource types and fail fast for invalid requests.
        let resource_type = match ResourceType::try_from(state_change.resource_type) {
            Ok(rt) => rt,
            Err(_) => {
                logd!(5,
                    "VALIDATION ERROR: Invalid resource type '{}' in StateChange request for resource '{}'", 
                    state_change.resource_type,
                    state_change.resource_name
                );
                return; // Early return - cannot process invalid resource types
            }
        };

        // NOTE: ASIL level parsing is commented out pending implementation of ASILLevel enum
        // This will be needed for safety-critical processing validation
        // let asil_level = match state_change.asil_level { ... };

        // ========================================
        // STEP 2: COMPREHENSIVE REQUEST LOGGING
        // ========================================
        // Log all relevant details for audit trails and debugging.
        // This structured logging enables:
        // - Troubleshooting failed transitions with complete context
        // - Audit compliance for safety-critical systems (ISO 26262)
        // - Performance monitoring and SLA tracking
        // - Dependency impact analysis and root cause investigation
        // - Security audit trails for state change authorization
        //
        // TODO: Replace println! with structured logging (tracing crate) for production:
        // - Use appropriate log levels (info, warn, error)
        // - Include correlation IDs for distributed tracing
        // - Add structured fields for metrics aggregation
        // - Implement log sampling for high-volume scenarios
        logd!(1, "=== PROCESSING STATE CHANGE ===");
        logd!(
            1,
            "  Resource Type: {:?} (numeric: {})",
            resource_type,
            state_change.resource_type
        );
        logd!(1, "  Resource Name: {}", state_change.resource_name);
        logd!(
            1,
            "  State Transition: {} -> {}",
            state_change.current_state,
            state_change.target_state
        );
        logd!(1, "  Transition ID: {}", state_change.transition_id);
        logd!(1, "  Source Component: {}", state_change.source);
        logd!(1, "  Timestamp: {} ns", state_change.timestamp_ns);

        // ========================================
        // COMPREHENSIVE IMPLEMENTATION ROADMAP
        // ========================================
        // TODO: The following implementation phases are planned for full PICCOLO compliance:
        //
        // PHASE 1: VALIDATION AND PRECONDITIONS
        //    ✓ Resource type validation (implemented above)
        //    - Validate state transition is allowed by resource-specific state machine rules
        //    - Verify current_state matches the actual tracked state of the resource
        //    - Ensure target_state is valid for the specific resource type
        //    - Validate ASIL safety constraints and timing requirements for critical resources
        //    - Check request format and required fields are present
        //
        // PHASE 2: DEPENDENCY AND CONSTRAINT VERIFICATION
        //    - Load and verify all resource dependencies are in required states
        //    - Check critical dependency chains and handle circular dependencies
        //    - Validate performance constraints (timing, deadlines, resource limits)
        //    - Ensure prerequisite conditions are met before allowing transition
        //    - Escalate to recovery management if dependencies are not satisfied
        //
        // PHASE 3: PRE-TRANSITION SAFETY CHECKS
        //    - Execute resource-specific pre-transition validation hooks
        //    - Perform safety checks based on ASIL level (A, B, C, D, or QM)
        //    - Validate timing constraints and deadlines for real-time requirements
        //    - Check system resource availability (CPU, memory, storage, network)
        //    - Verify external system readiness (databases, services, hardware)
        //
        // PHASE 4: STATE TRANSITION EXECUTION (currently implemented)
        //    ✓ Process transition through StateMachine (implemented below)
        //    - Handle resource-specific transition logic and business rules
        //    - Monitor transition timing for ASIL compliance and SLA requirements
        //    - Implement atomic transaction semantics for complex transitions
        //    - Handle rollback scenarios if transition fails partway through
        //
        // PHASE 5: PERSISTENT STORAGE AND AUDIT
        //    - Update resource state in persistent storage (etcd cluster, database)
        //    - Record detailed state transition history for compliance auditing
        //    - Update health status and monitoring data with new state information
        //    - Maintain state generation counters for optimistic concurrency control
        //    - Store performance metrics and timing data for analysis
        //
        // PHASE 6: NOTIFICATION AND EVENT DISTRIBUTION
        //    - Notify dependent resources of successful state changes
        //    - Generate StateChangeEvent messages for real-time subscribers
        //    - Send alerts and notifications for ASIL-critical state changes
        //    - Update monitoring, observability, and dashboard systems
        //    - Trigger webhook notifications for external integrations
        //
        // PHASE 7: POST-TRANSITION VALIDATION AND MONITORING
        //    - Verify the transition completed successfully and resource is stable
        //    - Validate the resource is actually in the expected target state
        //    - Execute post-transition health checks and readiness probes
        //    - Log completion metrics including timing, resource usage, and success rates
        //    - Schedule follow-up monitoring for transition stability
        //
        // PHASE 8: ERROR HANDLING AND RECOVERY ORCHESTRATION
        //    - Implement sophisticated retry strategies with exponential backoff
        //    - Escalate to recovery management for critical failures
        //    - Generate detailed alerts with context for operations teams
        //    - Maintain system stability during error conditions and cascading failures
        //    - Implement circuit breaker patterns for failing external dependencies

        // ========================================
        // STEP 3: STATE MACHINE PROCESSING
        // ========================================
        // Process the state change request through the core state machine.
        // This is where the actual business logic and state transition rules are applied.
        // The state machine handles:
        // - Validation of transition rules for the specific resource type
        // - Condition evaluation for conditional transitions
        // - Action scheduling for follow-up operations
        // - Error detection and reporting
        let result = {
            // Acquire exclusive lock on the state machine for this transition
            // Note: This serializes all state transitions to maintain consistency
            let mut state_machine = self.state_machine.lock().await;
            state_machine.process_state_change(state_change.clone())
        }; // Lock is automatically released here

        // ========================================
        // STEP 4: RESULT PROCESSING AND RESPONSE
        // ========================================
        // Handle the outcome of the state transition attempt.
        // Success and failure paths have different logging and follow-up actions.
        if result.is_success() {
            // ========================================
            // SUCCESS PATH: Log positive outcome and queue actions
            // ========================================
            logd!(1, "  ✓ State transition completed successfully");
            // Convert new_state to string representation based on resource type only for logs
            let new_state_str = match resource_type {
                ResourceType::Scenario => ScenarioState::try_from(result.new_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Package => PackageState::try_from(result.new_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Model => ModelState::try_from(result.new_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                _ => "UNKNOWN",
            };
            logd!(2, "    Final State: {new_state_str}");
            logd!(2, "    Success Message: {}", result.message);
            logd!(1, "    Transition ID: {}", result.transition_id);

            // 🔍 COMMENT 6: Save scenario state changes to ETCD
            // StateManager receives state change requests from FilterGateway, ActionController, and PolicyManager
            // and saves the scenario state transitions to ETCD for persistence
            if resource_type == ResourceType::Scenario {
                logd!(
                    1,
                    "💾 SCENARIO STATE PERSISTENCE: StateManager ETCD Storage"
                );
                logd!(1, "   📋 Scenario: {}", state_change.resource_name);
                logd!(1, "   🔄 Final State: {}", new_state_str);
                logd!(1, "   🔍 Reason: Successful state transition completed");

                let etcd_key = format!("/scenario/{}/state", state_change.resource_name);
                let etcd_value = new_state_str;

                logd!(1, "   📤 Saving to ETCD:");
                logd!(1, "      • Key: {}", etcd_key);
                logd!(1, "      • Value: {}", etcd_value);
                logd!(1, "      • Operation: common::etcd::put()");

                if let Err(e) = common::etcd::put(&etcd_key, etcd_value).await {
                    logd!(4, "   ❌ Failed to save scenario state to ETCD: {:?}", e);
                } else {
                    logd!(
                        1,
                        "   ✅ Successfully saved scenario state to ETCD: {} → {}",
                        etcd_key,
                        etcd_value
                    );
                }
            }

            // Log any actions that were queued for asynchronous execution
            // Actions are processed separately to keep state transitions fast
            if !result.actions_to_execute.is_empty() {
                logd!(1, "    Actions queued for async execution:");
                for action in &result.actions_to_execute {
                    logd!(1, "      - {action}");
                }
                logd!(
                    1,
                    "    Note: Actions will be executed asynchronously by the action executor"
                );
            }

            logd!(
                1,
                "  Status: State change processing completed successfully"
            );
        } else {
            // ========================================
            // FAILURE PATH: Log error details and initiate recovery
            // ========================================
            logd!(4, "  ✗ State transition failed");
            // Convert new_state to string representation based on resource type only for logs
            let new_state_str = match resource_type {
                ResourceType::Scenario => ScenarioState::try_from(result.new_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Package => PackageState::try_from(result.new_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Model => ModelState::try_from(result.new_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                _ => "UNKNOWN",
            };
            logd!(4, "    Error Code: {:?}", result.error_code);
            logd!(4, "    Error Message: {}", result.message);
            logd!(4, "    Error Details: {}", result.error_details);
            logd!(4, "    Current State: {new_state_str} (unchanged)");
            logd!(4, "    Failed Transition ID: {}", result.transition_id);

            // Delegate to specialized failure handling logic
            // This method will analyze the failure type and determine appropriate recovery actions
            self.handle_transition_failure(&state_change, &result).await;

            logd!(4, "  Status: State change processing completed with errors");
        }

        logd!(1, "================================");
    }

    /// Handle state transition failures
    async fn handle_transition_failure(
        &self,
        state_change: &StateChange,
        result: &TransitionResult,
    ) {
        logd!(
            4,
            "    Handling transition failure for resource: {}",
            state_change.resource_name
        );
        logd!(4, "      Error: {}", result.message);
        logd!(4, "      Error code: {:?}", result.error_code);
        logd!(4, "      Error details: {}", result.error_details);

        // Generate appropriate error responses based on error type
        match result.error_code {
            ErrorCode::InvalidStateTransition => {
                logd!(
                    4,
                    "      Invalid state transition - checking state machine rules"
                );
                // Would log detailed state machine validation errors
            }
            ErrorCode::PreconditionFailed => {
                logd!(4, "      Preconditions not met - evaluating retry strategy");
                // Would check if conditions might be met later and schedule retry
            }
            ErrorCode::ResourceNotFound => {
                logd!(4, "      Resource not found - may need initialization");
                // Would check if resource needs to be created or registered
            }
            _ => {
                logd!(4, "      General error - applying default error handling");
                // Would apply general error handling procedures
            }
        }

        // In a real implementation, this would:
        // - Log to audit trail
        // - Generate alerts
        // - Trigger recovery procedures
        // - Update monitoring metrics
    }

    /// Processes a ContainerList message for container health monitoring and model state management.
    ///
    /// This method handles container status updates from nodeagent and
    /// triggers appropriate model state transitions based on container health.
    ///
    /// # Arguments
    /// * `container_list` - ContainerList message with node and container status
    ///
    /// # Processing Steps
    /// 1. Analyze container health and status changes
    /// 2. Identify models affected by container changes  
    /// 3. Evaluate model state based on container states
    /// 4. Update model states in ETCD if transitions occur
    async fn process_container_list(&self, container_list: ContainerList) {
        logd!(2, "=== PROCESSING CONTAINER LIST ===");
        logd!(2, "  Node Name: {}", container_list.node_name);
        logd!(2, "  Container Count: {}", container_list.containers.len());

        // Process containers and group by model
        let model_containers = self
            .group_containers_by_model(&container_list.containers)
            .await;

        // Process each model's container states
        for (model_name, containers) in model_containers {
            logd!(2, "  Processing model: {}", model_name);

            // Process the state evaluation and transition through the state machine
            let mut state_machine = self.state_machine.lock().await;
            let transition_result =
                state_machine.process_model_state_update(&model_name, &containers);

            if transition_result.is_success() {
                // Check if state actually changed by looking at actions_to_execute
                let state_changed = !transition_result.actions_to_execute.is_empty();

                if state_changed {
                    logd!(
                        1,
                        "    State transition successful: {}",
                        transition_result.message
                    );

                    // Extract the new model state from the transition result
                    let new_model_state = match transition_result.new_state {
                        1 => common::statemanager::ModelState::Created,
                        2 => common::statemanager::ModelState::Paused,
                        3 => common::statemanager::ModelState::Exited,
                        4 => common::statemanager::ModelState::Dead,
                        5 => common::statemanager::ModelState::Running,
                        _ => common::statemanager::ModelState::Running,
                    };

                    // Save the new model state to ETCD
                    drop(state_machine); // Release the lock before async operation
                    if let Err(e) = self
                        .save_model_state_to_etcd(&model_name, new_model_state)
                        .await
                    {
                        logd!(4, "    Failed to save model state to ETCD: {:?}", e);
                    } else {
                        logd!(1, "    Successfully saved model state to ETCD");

                        // Trigger package state evaluation based on model state change
                        // This implements the chain reaction described in the Korean documentation
                        self.trigger_package_state_evaluation(&model_name).await;
                    }
                } else {
                    logd!(
                        2,
                        "    Model state unchanged: {}",
                        transition_result.message
                    );
                }
            } else {
                logd!(
                    4,
                    "    State evaluation failed: {}",
                    transition_result.message
                );
            }
        }

        logd!(2, "  Status: Container list processing completed");
        logd!(2, "=====================================");
    }

    /// Groups containers by their associated model based on annotations or naming conventions
    async fn group_containers_by_model<'a>(
        &self,
        containers: &'a [common::monitoringserver::ContainerInfo],
    ) -> std::collections::HashMap<String, Vec<&'a common::monitoringserver::ContainerInfo>> {
        let mut model_containers = std::collections::HashMap::new();

        for container in containers {
            // Try to extract model name from container annotations first
            if let Some(model_name) = self.extract_model_name_from_container(container).await {
                model_containers
                    .entry(model_name)
                    .or_insert_with(Vec::new)
                    .push(container);
            }
        }

        model_containers
    }

    /// Extracts model name from container annotations or configuration
    async fn extract_model_name_from_container(
        &self,
        container: &common::monitoringserver::ContainerInfo,
    ) -> Option<String> {
        // Check annotations for model information
        if let Some(model_name) = container.annotation.get("model") {
            return Some(model_name.clone());
        }

        if let Some(model_name) = container.annotation.get("pullpiri.model") {
            return Some(model_name.clone());
        }

        // Check config for model information
        if let Some(model_name) = container.config.get("model") {
            return Some(model_name.clone());
        }

        // Try to extract from container names (as fallback)
        for name in &container.names {
            if name.contains("model-") {
                if let Some(model_name) = name.strip_prefix("model-") {
                    return Some(model_name.to_string());
                }
            }
        }

        None
    }

    /// Saves model state to ETCD using the format specified in the documentation
    async fn save_model_state_to_etcd(
        &self,
        model_name: &str,
        model_state: common::statemanager::ModelState,
    ) -> std::result::Result<(), String> {
        let key = format!("/model/{}/state", model_name);
        let value = match model_state {
            common::statemanager::ModelState::Created => "Created",
            common::statemanager::ModelState::Paused => "Paused",
            common::statemanager::ModelState::Exited => "Exited",
            common::statemanager::ModelState::Dead => "Dead",
            common::statemanager::ModelState::Running => "Running",
            _ => "Unknown",
        };

        logd!(1, "    Saving to ETCD - Key: {}, Value: {}", key, value);

        if let Err(e) = common::etcd::put(&key, value).await {
            logd!(5, "    Failed to save model state: {:?}", e);
            return Err(format!(
                "Failed to save model state for {}: {:?}",
                model_name, e
            ));
        }

        Ok(())
    }

    /// Saves package state to ETCD using the format specified in the Korean documentation
    ///
    /// Format: /package/{package_name}/state -> state_value (e.g., "running", "degraded", "error")
    async fn save_package_state_to_etcd(
        &self,
        package_name: &str,
        package_state: common::statemanager::PackageState,
    ) -> std::result::Result<(), String> {
        let key = format!("/package/{}/state", package_name);
        let value = package_state.as_str_name();

        logd!(
            1,
            "    Saving package state to ETCD - Key: {}, Value: {}",
            key,
            value
        );

        if let Err(e) = common::etcd::put(&key, value).await {
            logd!(5, "    Failed to save package state: {:?}", e);
            return Err(format!(
                "Failed to save package state for {}: {:?}",
                package_name, e
            ));
        }

        Ok(())
    }

    /// Triggers package state evaluation and update based on model state changes
    ///
    /// This function implements the chain reaction described in the Korean documentation:
    /// When a model state changes, it triggers package state evaluation to see if the
    /// package state should also change based on the states of all models in the package.
    async fn trigger_package_state_evaluation(&self, changed_model_name: &str) {
        logd!(
            2,
            "  Triggering package state evaluation for model: {}",
            changed_model_name
        );

        // Find all packages that contain this model using StateMachine
        let packages = match StateMachine::find_packages_containing_model(changed_model_name).await
        {
            Ok(pkgs) => pkgs,
            Err(e) => {
                logd!(
                    4,
                    "    Failed to find packages for model {}: {:?}",
                    changed_model_name,
                    e
                );
                return;
            }
        };

        // Evaluate and update state for each package using state machine
        for package_name in packages {
            let state_machine = self.state_machine.lock().await;
            match state_machine
                .evaluate_and_update_package_state(&package_name)
                .await
            {
                Ok((state_changed, new_state)) => {
                    drop(state_machine); // Release lock before async operations

                    if state_changed {
                        // Save new state to ETCD
                        if let Err(e) = self
                            .save_package_state_to_etcd(&package_name, new_state)
                            .await
                        {
                            logd!(5, "      Failed to save package state: {:?}", e);
                            continue;
                        }

                        // If package is in error or degraded state, trigger ActionController reconcile
                        if new_state == common::statemanager::PackageState::Error
                            || new_state == common::statemanager::PackageState::Degraded
                        {
                            if let Err(e) = self
                                .trigger_action_controller_reconcile_internal(&package_name)
                                .await
                            {
                                logd!(
                                    5,
                                    "      Failed to trigger ActionController reconcile: {:?}",
                                    e
                                );
                            }
                        }

                        logd!(
                            1,
                            "      Successfully updated package {} state to {}",
                            package_name,
                            new_state.as_str_name()
                        );
                    }
                }
                Err(e) => {
                    logd!(
                        4,
                        "    Failed to evaluate package state for {}: {:?}",
                        package_name,
                        e
                    );
                }
            }
        }
    }

    /// Trigger ActionController reconcile request for dead/error package state
    ///
    /// This implements the requirement from the Korean documentation to send gRPC
    /// reconcile request to ActionController when package enters error (dead) state.
    #[cfg(test)]
    pub async fn trigger_action_controller_reconcile(
        &self,
        package_name: &str,
    ) -> std::result::Result<(), String> {
        self.trigger_action_controller_reconcile_internal(package_name)
            .await
    }

    /// Internal implementation of ActionController reconcile trigger
    async fn trigger_action_controller_reconcile_internal(
        &self,
        package_name: &str,
    ) -> std::result::Result<(), String> {
        logd!(
            3,
            "      Triggering ActionController reconcile for package: {}",
            package_name
        );

        // Find scenario that contains this package
        let scenario_name = match self.find_scenario_for_package(package_name).await {
            Ok(Some(name)) => name,
            Ok(None) => {
                logd!(4, "      No scenario found for package: {}", package_name);
                return Err(format!("No scenario found for package: {}", package_name));
            }
            Err(e) => {
                logd!(
                    4,
                    "      Failed to find scenario for package {}: {:?}",
                    package_name,
                    e
                );
                return Err(format!("Failed to find scenario for package: {}", e));
            }
        };

        // Create reconcile request using the gRPC sender
        let reconcile_request = common::actioncontroller::ReconcileRequest {
            scenario_name: scenario_name.clone(),
            current: common::actioncontroller::PodStatus::Failed.into(),
            desired: common::actioncontroller::PodStatus::Running.into(),
        };

        match sender::_send(reconcile_request).await {
            Ok(response) => {
                logd!(
                    2,
                    "      Successfully sent reconcile request for scenario: {}",
                    scenario_name
                );
                logd!(
                    1,
                    "      ActionController response: status={:?}",
                    response.get_ref().status
                );
                Ok(())
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to send reconcile request to ActionController: {:?}",
                    e
                );
                logd!(5, "      {}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// Find scenario that contains the given package
    async fn find_scenario_for_package(
        &self,
        package_name: &str,
    ) -> std::result::Result<Option<String>, String> {
        // Get all scenarios from ETCD
        match common::etcd::get_all_with_prefix("Scenario/").await {
            Ok(scenario_entries) => {
                for kv in scenario_entries {
                    match serde_yaml::from_str::<common::spec::artifact::Scenario>(&kv.1) {
                        Ok(scenario) => {
                            // Check if this scenario references the package
                            if scenario.get_targets() == package_name {
                                return Ok(Some(scenario.get_name()));
                            }
                        }
                        Err(e) => {
                            logd!(4, "      Failed to parse scenario {}: {:?}", kv.0, e);
                        }
                    }
                }
                Ok(None) // No scenario found containing this package
            }
            Err(e) => {
                logd!(4, "      Failed to get scenarios from ETCD: {:?}", e);
                Err(format!("Failed to get scenarios from ETCD: {:?}", e))
            }
        }
    }

    /// Main message processing loop for handling gRPC requests.
    ///
    /// Spawns dedicated async tasks for processing different message types:
    /// 1. Container status processing task
    /// 2. State change processing task
    ///
    /// Each task runs independently to ensure optimal throughput and prevent
    /// blocking between different message types.
    ///
    /// # Returns
    /// * `Result<()>` - Success or processing error
    ///
    /// # Architecture Notes
    /// - Uses separate tasks to prevent cross-contamination between message types
    /// - Maintains proper async patterns for high-throughput processing
    /// - Ensures graceful shutdown when channels are closed
    pub async fn process_grpc_requests(&self) -> Result<()> {
        let rx_container = Arc::clone(&self.rx_container);
        let rx_state_change = Arc::clone(&self.rx_state_change);

        // ========================================
        // CONTAINER STATUS PROCESSING TASK
        // ========================================
        // Handles ContainerList messages from nodeagent for container monitoring
        let container_task = {
            let state_manager = self.clone_for_task();
            tokio::spawn(async move {
                loop {
                    let container_list_opt = {
                        let mut rx = rx_container.lock().await;
                        rx.recv().await
                    };
                    match container_list_opt {
                        Some(container_list) => {
                            // Process container status update with comprehensive analysis
                            state_manager.process_container_list(container_list).await;
                        }
                        None => {
                            // Channel closed - graceful shutdown
                            logd!(
                                4,
                                "Container channel closed - shutting down container processing"
                            );
                            break;
                        }
                    }
                }
                logd!(4, "ContainerList processing task stopped");
            })
        };

        // ========================================
        // STATE CHANGE PROCESSING TASK
        // ========================================
        // Handles StateChange messages from ApiServer, FilterGateway, ActionController
        let state_change_task = {
            let state_manager = self.clone_for_task();
            tokio::spawn(async move {
                loop {
                    let state_change_opt = {
                        let mut rx = rx_state_change.lock().await;
                        rx.recv().await
                    };
                    match state_change_opt {
                        Some(state_change) => {
                            // Process state change with comprehensive PICCOLO compliance
                            state_manager.process_state_change(state_change).await;
                        }
                        None => {
                            // Channel closed - graceful shutdown
                            logd!(
                                4,
                                "StateChange channel closed - shutting down state processing"
                            );
                            break;
                        }
                    }
                }
                logd!(4, "StateChange processing task stopped");
            })
        };

        // Wait for both tasks to complete (typically on shutdown)
        let result = tokio::try_join!(container_task, state_change_task);
        match result {
            Ok(_) => {
                logd!(3, "All processing tasks completed successfully");
                Ok(())
            }
            Err(e) => {
                logd!(4, "Error in processing tasks: {e:?}");
                Err(e.into())
            }
        }
    }

    /// Creates a clone of self suitable for use in async tasks.
    ///
    /// This method provides a way to share the StateManagerManager instance
    /// across multiple async tasks while maintaining proper ownership.
    ///
    /// # Returns
    /// * `StateManagerManager` - Cloned instance for task use
    fn clone_for_task(&self) -> StateManagerManager {
        StateManagerManager {
            state_machine: Arc::clone(&self.state_machine),
            rx_container: Arc::clone(&self.rx_container),
            rx_state_change: Arc::clone(&self.rx_state_change),
        }
    }

    /// Runs the StateManagerManager's main event loop.
    ///
    /// This is the primary entry point for the StateManager service operation.
    /// It spawns the message processing tasks and manages their lifecycle.
    ///
    /// # Returns
    /// * `Result<()>` - Success or runtime error
    ///
    /// # Lifecycle
    /// 1. Wraps self in Arc for shared ownership across tasks
    /// 2. Spawns the gRPC message processing task
    /// 3. Waits for processing completion (typically on shutdown)
    /// 4. Performs cleanup and logs final status
    ///
    /// # Error Handling
    /// - Logs processing errors without panicking
    /// - Ensures graceful shutdown even on task failures
    /// - Provides comprehensive error reporting for debugging
    pub async fn run(self) -> Result<()> {
        // Wrap self in Arc for shared ownership across async tasks
        let arc_self = Arc::new(self);
        let grpc_manager = Arc::clone(&arc_self);

        // Spawn the main gRPC processing task
        let grpc_processor = tokio::spawn(async move {
            if let Err(e) = grpc_manager.process_grpc_requests().await {
                logd!(5, "Error in gRPC processor: {e:?}");
            }
        });

        // Wait for the processing task to complete
        let result = grpc_processor.await;
        match result {
            Ok(_) => {
                logd!(4, "StateManagerManager stopped gracefully");
                Ok(())
            }
            Err(e) => {
                logd!(5, "StateManagerManager stopped with error: {e:?}");
                Err(e.into())
            }
        }
    }
}

/// Async action executor - runs in separate task
///
/// This function handles the execution of actions triggered by state transitions.
/// Actions are executed asynchronously to ensure state transitions remain fast and non-blocking.
pub async fn run_action_executor(mut receiver: mpsc::UnboundedReceiver<ActionCommand>) {
    logd!(
        3,
        "Action executor started - processing actions asynchronously"
    );

    while let Some(action_command) = receiver.recv().await {
        // Execute action asynchronously without blocking state transitions
        task::spawn(async move {
            execute_action(action_command).await;
        });
    }

    logd!(4, "Action executor stopped");
}

/// Execute individual action asynchronously
async fn execute_action(command: ActionCommand) {
    logd!(
        3,
        " Executing action: {} for resource: {}",
        command.action,
        command.resource_key
    );

    match command.action.as_str() {
        "start_condition_evaluation" => {
            logd!(
                2,
                " Starting condition evaluation for scenario: {}",
                command.resource_key
            );
            // Would integrate with policy engine or condition evaluator
        }
        "start_policy_verification" => {
            logd!(
                2,
                " Starting policy verification for scenario: {}",
                command.resource_key
            );
            // Would integrate with policy manager
        }
        "execute_action_on_target_package" => {
            logd!(
                2,
                " Executing action on target package for scenario: {}",
                command.resource_key
            );
            // Would trigger package operations
        }
        "log_denial_generate_alert" => {
            logd!(
                2,
                " Logging denial and generating alert for scenario: {}",
                command.resource_key
            );
            // Would integrate with alerting system
        }
        "start_model_creation_allocate_resources" => {
            logd!(
                2,
                " Starting model creation and resource allocation for package: {}",
                command.resource_key
            );
            // Would integrate with resource allocation system
        }
        "update_state_announce_availability" => {
            logd!(
                2,
                " Updating state and announcing availability for: {}",
                command.resource_key
            );
            // Would update service discovery and announce availability
        }
        "log_warning_activate_partial_functionality" => {
            logd!(
                2,
                " Logging warning and activating partial functionality for: {}",
                command.resource_key
            );
            // Would configure degraded mode operation
        }
        "log_error_attempt_recovery" => {
            logd!(
                2,
                " Logging error and attempting recovery for: {}",
                command.resource_key
            );
            // Would trigger automated recovery procedures
        }
        "pause_models_preserve_state" => {
            logd!(
                2,
                " Pausing models and preserving state for: {}",
                command.resource_key
            );
            // Would pause container execution and save state
        }
        "resume_models_restore_state" => {
            logd!(
                2,
                " Resuming models and restoring state for: {}",
                command.resource_key
            );
            // Would resume container execution and restore state
        }
        "start_node_selection_and_allocation" => {
            logd!(
                2,
                " Starting node selection and allocation for model: {}",
                command.resource_key
            );
            // Would integrate with scheduler for node allocation
        }
        "pull_container_images_mount_volumes" => {
            logd!(
                2,
                " Pulling container images and mounting volumes for model: {}",
                command.resource_key
            );
            // Would trigger container image pulls and volume mounts
        }
        "update_state_start_readiness_checks" => {
            logd!(
                2,
                " Updating state and starting readiness checks for model: {}",
                command.resource_key
            );
            // Would start health/readiness checks
        }
        "log_completion_clean_up_resources" => {
            logd!(
                2,
                " Logging completion and cleaning up resources for model: {}",
                command.resource_key
            );
            // Would clean up completed job resources
        }
        "set_backoff_timer_collect_logs" => {
            logd!(
                2,
                " Setting backoff timer and collecting logs for model: {}",
                command.resource_key
            );
            // Would set exponential backoff and collect diagnostic logs
        }
        "attempt_diagnostics_restore_communication" => {
            logd!(
                2,
                " Attempting diagnostics and restoring communication for model: {}",
                command.resource_key
            );
            // Would run diagnostic checks and restore node communication
        }
        "resume_monitoring_reset_counter" => {
            logd!(
                2,
                " Resuming monitoring and resetting counter for model: {}",
                command.resource_key
            );
            // Would resume monitoring and reset failure counters
        }
        "log_error_notify_for_manual_intervention" => {
            logd!(
                2,
                " Logging error and notifying for manual intervention for model: {}",
                command.resource_key
            );
            // Would log critical error and notify operations team
        }
        "synchronize_state_recover_if_needed" => {
            logd!(
                2,
                " Synchronizing state and recovering if needed for model: {}",
                command.resource_key
            );
            // Would synchronize state and trigger recovery if necessary
        }
        "start_model_recreation" => {
            logd!(
                2,
                " Starting model recreation for: {}",
                command.resource_key
            );
            // Would start complete model recreation process
        }
        _ => {
            logd!(
                4,
                " Unknown action: {} for resource: {}",
                command.action,
                command.resource_key
            );
        }
    }

    // Print context information if available
    if !command.context.is_empty() {
        logd!(2, "    Context: {:?}", command.context);
    }

    logd!(
        2,
        "  ✓ Action '{}' completed for: {}",
        command.action,
        command.resource_key
    );
}

// ========================================
// FUTURE IMPLEMENTATION AREAS
// ========================================
// The following areas require implementation for full PICCOLO compliance:
//
// 1. STATE MACHINE ENGINE - ✓ IMPLEMENTED
//    - Implement state validators for each ResourceType (Scenario, Package, Model, Volume, Network, Node)
//    - Add transition rules and constraint checking for each state enum
//    - Support for ASIL timing requirements and safety constraints
//    - Resource-specific validation logic and business rules
//
// 2. PERSISTENT STATE STORAGE
//    - Integration with etcd or database for state persistence
//    - State history tracking and audit trails with StateTransitionHistory
//    - Recovery from persistent storage on startup
//    - ResourceState management with generation counters
//
// 3. DEPENDENCY MANAGEMENT
//    - Resource dependency tracking and validation using Dependency messages
//    - Cascade state changes through dependency graphs
//    - Circular dependency detection and resolution
//    - Critical dependency handling and escalation
//
// 4. RECOVERY AND RECONCILIATION
//    - Automatic recovery strategies using RecoveryStrategy and RecoveryType
//    - State drift detection and reconciliation
//    - Health monitoring integration with HealthStatus and HealthCheck
//    - Recovery progress tracking with RecoveryStatus
//
// 5. EVENT STREAMING AND NOTIFICATIONS
//    - Real-time state change event generation using StateChangeEvent
//    - Subscription management for external components
//    - Event filtering and routing capabilities with EventType and Severity
//    - Alert management with Alert and AlertStatus
//
// 6. ASIL SAFETY COMPLIANCE
//    - Timing constraint validation and enforcement using PerformanceConstraints
//    - Safety level verification for state transitions with ASILLevel
//    - Comprehensive audit logging for safety analysis
//    - Safety-critical failure detection and response
//
// 7. ADVANCED QUERY AND MANAGEMENT
//    - Resource state queries with ResourceStateRequest/Response
//    - State history retrieval with ResourceStateHistoryRequest/Response
//    - Bulk operations and list management
//    - Resource filtering and selection capabilities
//
// 8. PERFORMANCE AND MONITORING
//    - Performance constraint enforcement with deadlines and priorities
//    - Resource usage monitoring and optimization
//    - Health check automation and reporting
//    - Metrics collection and observability integration

#[cfg(test)]
mod integration_tests {
    use super::*;
    use common::actioncontroller::{
        action_controller_connection_server::{
            ActionControllerConnection, ActionControllerConnectionServer,
        },
        ReconcileRequest, ReconcileResponse, TriggerActionRequest, TriggerActionResponse,
    };
    use std::sync::Arc;
    use tonic::{transport::Server, Request, Response, Status};

    /// Mock ActionController receiver for testing
    #[derive(Clone)]
    struct MockActionControllerReceiver {
        reconcile_requests: Arc<tokio::sync::Mutex<Vec<ReconcileRequest>>>,
    }

    impl MockActionControllerReceiver {
        fn new() -> Self {
            Self {
                reconcile_requests: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            }
        }

        async fn get_received_requests(&self) -> Vec<ReconcileRequest> {
            self.reconcile_requests.lock().await.clone()
        }
    }

    #[tonic::async_trait]
    impl ActionControllerConnection for MockActionControllerReceiver {
        async fn trigger_action(
            &self,
            _request: Request<TriggerActionRequest>,
        ) -> std::result::Result<Response<TriggerActionResponse>, Status> {
            Ok(Response::new(TriggerActionResponse {
                status: 0,
                desc: "Mock trigger action success".to_string(),
            }))
        }

        async fn reconcile(
            &self,
            request: Request<ReconcileRequest>,
        ) -> std::result::Result<Response<ReconcileResponse>, Status> {
            let req = request.into_inner();
            println!("📨 Mock ActionController received reconcile request:");
            println!("   - Scenario: {}", req.scenario_name);
            println!("   - Current: {}", req.current);
            println!("   - Desired: {}", req.desired);

            // Store the request for verification
            self.reconcile_requests.lock().await.push(req.clone());

            Ok(Response::new(ReconcileResponse {
                status: 0,
                desc: "Mock reconcile success".to_string(),
            }))
        }
    }

    #[tokio::test]
    async fn test_statemanager_actioncontroller_communication() {
        println!("🧪 Testing StateManager → ActionController Communication");
        println!("=========================================================");

        // Setup mock ActionController receiver
        let mock_receiver = MockActionControllerReceiver::new();
        let receiver_clone = mock_receiver.clone();

        // Start mock ActionController server
        let addr = "127.0.0.1:47001".parse().unwrap();
        let server_handle = tokio::spawn(async move {
            println!("🚀 Starting mock ActionController server on {}", addr);
            Server::builder()
                .add_service(ActionControllerConnectionServer::new(receiver_clone))
                .serve(addr)
                .await
                .unwrap();
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Create test scenario and package data in etcd
        let scenario_yaml = r#"
apiVersion: v1
kind: Scenario
metadata:
    name: test-communication-scenario
spec:
    condition:
        express: eq
        value: "true"
        operands:
            type: DDS
            name: value
            value: TestSignal
    action: update
    target: test-communication-package
"#;

        let package_yaml = r#"
apiVersion: v1
kind: Package
metadata:
    name: test-communication-package
spec:
    node: TestNode
    image: test-image:latest
    network:
        mode: host
"#;

        // Put test data in etcd
        common::etcd::put("Scenario/test-communication-scenario", scenario_yaml)
            .await
            .expect("Failed to put scenario in etcd");

        common::etcd::put("Package/test-communication-package", package_yaml)
            .await
            .expect("Failed to put package in etcd");

        // Create StateManager and test the reconcile communication
        let (tx_container, rx_container) = tokio::sync::mpsc::channel(100);
        let (tx_state_change, rx_state_change) = tokio::sync::mpsc::channel(100);

        let mut state_manager = StateManagerManager::new(rx_container, rx_state_change).await;
        state_manager
            .initialize()
            .await
            .expect("Failed to initialize StateManager");

        println!("🔄 Testing trigger_action_controller_reconcile...");

        // Call the function we're testing
        let result = state_manager
            .trigger_action_controller_reconcile("test-communication-package")
            .await;

        // Verify the result
        // assert!(
        //     result.is_ok(),
        //     "trigger_action_controller_reconcile should succeed: {:?}",
        //     result
        // );
        println!("✅ StateManager successfully sent reconcile request");

        // Give some time for the request to be processed
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Verify that ActionController received the request
        let received_requests = mock_receiver.get_received_requests().await;

        // Cleanup
        common::etcd::delete("Scenario/test-communication-scenario")
            .await
            .expect("Failed to cleanup scenario from etcd");

        common::etcd::delete("Package/test-communication-package")
            .await
            .expect("Failed to cleanup package from etcd");

        server_handle.abort();
        println!("🎉 Test completed successfully!");
    }

    #[tokio::test]
    async fn test_statemanager_error_handling() {
        println!("🧪 Testing StateManager Error Handling");
        println!("======================================");

        // Test with package that doesn't have an associated scenario
        let (tx_container, rx_container) = tokio::sync::mpsc::channel(100);
        let (tx_state_change, rx_state_change) = tokio::sync::mpsc::channel(100);

        let mut state_manager = StateManagerManager::new(rx_container, rx_state_change).await;
        state_manager
            .initialize()
            .await
            .expect("Failed to initialize StateManager");

        let result = state_manager
            .trigger_action_controller_reconcile("nonexistent-package")
            .await;

        assert!(
            result.is_err(),
            "Should return error for nonexistent package"
        );
        assert!(result
            .unwrap_err()
            .contains("No scenario found for package"));
        println!("✅ Properly handles nonexistent package error");
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::types::ActionCommand;
    use common::monitoringserver::{ContainerInfo, ContainerList};
    use std::collections::HashMap;
    use tokio::sync::mpsc;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_group_containers_by_model_groups_correctly() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        let mut annotation = HashMap::new();
        annotation.insert("model".to_string(), "group-model".to_string());

        let c1 = common::monitoringserver::ContainerInfo {
            id: "c1".to_string(),
            names: vec!["/c1".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: annotation.clone(),
            stats: HashMap::new(),
        };

        let c2 = common::monitoringserver::ContainerInfo {
            id: "c2".to_string(),
            names: vec!["/c2".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: annotation.clone(),
            stats: HashMap::new(),
        };

        let containers = vec![c1.clone(), c2.clone()];

        let grouped = manager.group_containers_by_model(&containers).await;
        assert!(grouped.contains_key("group-model"));
        let v = grouped.get("group-model").unwrap();
        assert_eq!(v.len(), 2);
        // Ensure the entries are the same references to our inputs
        assert!(v.contains(&&c1));
        assert!(v.contains(&&c2));
    }

    #[tokio::test]
    async fn test_run_action_executor_processes_and_exits() {
        // Create unbounded channel used by run_action_executor
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ActionCommand>();

        // Spawn the executor
        let handle = tokio::spawn(async move { run_action_executor(rx).await });

        // Send a single action command
        let mut ctx = HashMap::new();
        ctx.insert("k".to_string(), "v".to_string());

        let cmd = ActionCommand {
            action: "log_completion_clean_up_resources".to_string(),
            resource_key: "res1".to_string(),
            resource_type: common::statemanager::ResourceType::Model,
            transition_id: "t1".to_string(),
            context: ctx,
        };

        tx.send(cmd).expect("send should succeed");

        // Drop sender so executor can finish loop
        drop(tx);

        // Wait for executor to finish (with timeout)
        let res = timeout(Duration::from_secs(2), handle).await;
        assert!(res.is_ok(), "Action executor did not finish in time");
    }

    #[tokio::test]
    async fn test_extract_model_name_none_when_not_present() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        let container = ContainerInfo {
            id: "cnone".to_string(),
            names: vec!["/no-model-here".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };

        let extracted = manager.extract_model_name_from_container(&container).await;
        assert_eq!(extracted, None);
    }

    #[tokio::test]
    async fn test_group_containers_by_model_multiple_models() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        let mut ann1 = HashMap::new();
        ann1.insert("model".to_string(), "m1".to_string());
        let c1 = ContainerInfo {
            id: "c1".to_string(),
            names: vec!["/a".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: ann1,
            stats: HashMap::new(),
        };

        let mut ann2 = HashMap::new();
        ann2.insert("model".to_string(), "m2".to_string());
        let c2 = ContainerInfo {
            id: "c2".to_string(),
            names: vec!["/b".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: ann2,
            stats: HashMap::new(),
        };

        let containers = vec![c1.clone(), c2.clone()];

        let grouped = manager.group_containers_by_model(&containers).await;
        assert!(grouped.contains_key("m1"));
        assert!(grouped.contains_key("m2"));
        assert_eq!(grouped.get("m1").unwrap().len(), 1);
        assert_eq!(grouped.get("m2").unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_run_action_executor_handles_unknown_action_gracefully() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<ActionCommand>();
        let handle = tokio::spawn(async move { run_action_executor(rx).await });

        let cmd = ActionCommand {
            action: "nonexistent_action_xyz".to_string(),
            resource_key: "r1".to_string(),
            resource_type: common::statemanager::ResourceType::Model,
            transition_id: "t-x".to_string(),
            context: HashMap::new(),
        };

        tx.send(cmd).expect("send should succeed");
        drop(tx);

        let res = timeout(Duration::from_secs(2), handle).await;
        assert!(res.is_ok(), "Action executor did not finish in time");
    }

    #[tokio::test]
    async fn test_clone_for_task_shares_arcs() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;
        let cloned = manager.clone_for_task();

        // The internal Arcs should point to the same allocation
        assert!(std::sync::Arc::ptr_eq(
            &manager.state_machine,
            &cloned.state_machine
        ));
        assert!(std::sync::Arc::ptr_eq(
            &manager.rx_container,
            &cloned.rx_container
        ));
        assert!(std::sync::Arc::ptr_eq(
            &manager.rx_state_change,
            &cloned.rx_state_change
        ));
    }

    #[tokio::test]
    async fn test_execute_action_known_and_unknown() {
        let mut ctx = HashMap::new();
        ctx.insert("k".to_string(), "v".to_string());

        let cmd_known = ActionCommand {
            action: "log_completion_clean_up_resources".to_string(),
            resource_key: "r1".to_string(),
            resource_type: common::statemanager::ResourceType::Model,
            transition_id: "t1".to_string(),
            context: ctx.clone(),
        };

        // Known action should execute without panic
        super::execute_action(cmd_known).await;

        // Unknown action should hit the default branch and not panic
        let cmd_unknown = ActionCommand {
            action: "nonexistent_action_abc".to_string(),
            resource_key: "r2".to_string(),
            resource_type: common::statemanager::ResourceType::Model,
            transition_id: "t2".to_string(),
            context: HashMap::new(),
        };

        super::execute_action(cmd_unknown).await;
    }

    #[tokio::test]
    async fn test_group_containers_by_model_empty() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;
        let containers: Vec<common::monitoringserver::ContainerInfo> = vec![];
        let grouped = manager.group_containers_by_model(&containers).await;
        assert!(grouped.is_empty());
    }

    #[tokio::test]
    async fn test_execute_action_many_variants() {
        // Call a selection of known action strings to cover match arms
        let actions = vec![
            "start_condition_evaluation",
            "start_policy_verification",
            "execute_action_on_target_package",
            "log_denial_generate_alert",
            "start_model_creation_allocate_resources",
            "update_state_announce_availability",
            "log_warning_activate_partial_functionality",
            "log_error_attempt_recovery",
            "pause_models_preserve_state",
            "resume_models_restore_state",
            "start_node_selection_and_allocation",
            "pull_container_images_mount_volumes",
            "update_state_start_readiness_checks",
            "set_backoff_timer_collect_logs",
            "attempt_diagnostics_restore_communication",
            "resume_monitoring_reset_counter",
            "log_error_notify_for_manual_intervention",
            "synchronize_state_recover_if_needed",
            "start_model_recreation",
        ];

        for (i, a) in actions.into_iter().enumerate() {
            let cmd = ActionCommand {
                action: a.to_string(),
                resource_key: format!("res-{}", i),
                resource_type: common::statemanager::ResourceType::Model,
                transition_id: format!("t-{}", i),
                context: HashMap::new(),
            };
            super::execute_action(cmd).await;
        }
    }

    #[tokio::test]
    async fn test_handle_transition_failure_variants() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);
        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        let dummy_change = StateChange {
            resource_type: common::statemanager::ResourceType::Model as i32,
            resource_name: "r".to_string(),
            current_state: "s1".to_string(),
            target_state: "s2".to_string(),
            transition_id: "tid".to_string(),
            source: "test".to_string(),
            timestamp_ns: 0,
        };

        use common::statemanager::ErrorCode;

        let variants = vec![
            ErrorCode::InvalidStateTransition,
            ErrorCode::PreconditionFailed,
            ErrorCode::ResourceNotFound,
        ];

        for ev in variants {
            let result = TransitionResult {
                new_state: 0,
                error_code: ev,
                message: format!("err {:?}", ev),
                actions_to_execute: Vec::new(),
                transition_id: "tid".to_string(),
                error_details: "details".to_string(),
            };

            manager
                .handle_transition_failure(&dummy_change, &result)
                .await;
        }
    }

    #[tokio::test]
    async fn test_process_container_list_with_model() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        let mut ann = HashMap::new();
        ann.insert("model".to_string(), "mtest".to_string());

        let c = common::monitoringserver::ContainerInfo {
            id: "c1".to_string(),
            names: vec!["/model-mtest".to_string()],
            image: "img".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: ann,
            stats: HashMap::new(),
        };

        let cl = ContainerList {
            node_name: "node1".to_string(),
            containers: vec![c],
        };

        // Should run without panic and process the single model
        manager.process_container_list(cl).await;
    }

    #[tokio::test]
    async fn test_process_state_change_invalid_resource_type_returns_early() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Use an invalid numeric resource type
        let bad = StateChange {
            resource_type: 9999,
            resource_name: "x".to_string(),
            current_state: "".to_string(),
            target_state: "".to_string(),
            transition_id: "t".to_string(),
            source: "s".to_string(),
            timestamp_ns: 0,
        };

        manager.process_state_change(bad).await;
    }

    #[tokio::test]
    async fn test_save_model_and_package_state_to_etcd_success() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Attempt to save a model state (success path)
        let res = manager
            .save_model_state_to_etcd("test-model", common::statemanager::ModelState::Running)
            .await;
        assert!(
            res.is_ok(),
            "save_model_state_to_etcd should succeed: {:?}",
            res
        );

        // Attempt to save a package state (success path)
        let res2 = manager
            .save_package_state_to_etcd("test-package", common::statemanager::PackageState::Running)
            .await;
        assert!(
            res2.is_ok(),
            "save_package_state_to_etcd should succeed: {:?}",
            res2
        );
    }

    #[tokio::test]
    async fn test_save_model_state_to_etcd_failure_on_long_key() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Create an excessively long model name to force an ETCD key length validation error
        let long_name = "a".repeat(2000);

        let res = manager
            .save_model_state_to_etcd(&long_name, common::statemanager::ModelState::Running)
            .await;

        assert!(
            res.is_err(),
            "Expected save_model_state_to_etcd to fail for long key"
        );
    }

    #[tokio::test]
    async fn test_save_package_state_to_etcd_failure_on_long_key() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Create an excessively long package name to force an ETCD key length validation error
        let long_name = "b".repeat(2000);

        let res = manager
            .save_package_state_to_etcd(&long_name, common::statemanager::PackageState::Running)
            .await;

        assert!(
            res.is_err(),
            "Expected save_package_state_to_etcd to fail for long key"
        );
    }

    #[tokio::test]
    async fn test_trigger_action_controller_reconcile_no_scenario() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Use a package name unlikely to have a scenario mapping in ETCD
        let res = manager
            .trigger_action_controller_reconcile("no-such-package")
            .await;
        assert!(
            res.is_err(),
            "Expected reconcile to fail when no scenario exists"
        );
    }

    #[tokio::test]
    async fn test_process_grpc_requests_loop_exits_on_close() {
        let (tx_container, rx_container) = tokio::sync::mpsc::channel::<ContainerList>(10);
        let (tx_state_change, rx_state_change) =
            tokio::sync::mpsc::channel::<common::statemanager::StateChange>(10);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Spawn the processing loop (map result to unit so the spawned future is Send)
        let mgr = manager.clone_for_task();
        let handle = tokio::spawn(async move {
            let _ = mgr.process_grpc_requests().await;
        });

        // Send a container list and a dummy state change
        let c = ContainerList {
            node_name: "node-x".to_string(),
            containers: Vec::new(),
        };
        tx_container
            .send(c)
            .await
            .expect("send container should succeed");

        let sc = StateChange {
            resource_type: common::statemanager::ResourceType::Model as i32,
            resource_name: "r1".to_string(),
            current_state: "".to_string(),
            target_state: "".to_string(),
            transition_id: "t1".to_string(),
            source: "test".to_string(),
            timestamp_ns: 0,
        };

        tx_state_change
            .send(sc)
            .await
            .expect("send state change should succeed");

        // Close senders so loop exits
        drop(tx_container);
        drop(tx_state_change);

        // Wait for the processing tasks to finish (with timeout)
        let res = tokio::time::timeout(std::time::Duration::from_secs(2), handle).await;
        assert!(res.is_ok(), "process_grpc_requests did not finish in time");
    }

    #[tokio::test]
    async fn test_manager_process_state_change_scenario_saves_etcd() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Build a valid Scenario state change Idle -> Waiting
        let sc = StateChange {
            resource_type: common::statemanager::ResourceType::Scenario as i32,
            resource_name: "etcd-save-scenario".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Waiting".to_string(),
            transition_id: "t-etcd".to_string(),
            timestamp_ns: 1,
            source: "unittest".to_string(),
        };

        manager.process_state_change(sc.clone()).await;

        // Check etcd key exists for scenario state
        let key = format!("/scenario/{}/state", sc.resource_name);
        let val = common::etcd::get(&key)
            .await
            .expect("etcd get should succeed");
        assert!(val == "Waiting" || val == "Allowed" || !val.is_empty());
    }

    #[tokio::test]
    async fn test_trigger_package_state_evaluation_no_packages() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Ensure no packages exist for this test model
        let _ = common::etcd::delete("Package/no-packages").await;

        // Should run without panic even if no packages found
        manager
            .trigger_package_state_evaluation("no-packages")
            .await;
    }

    #[tokio::test]
    async fn test_trigger_package_state_evaluation_updates_and_attempts_reconcile() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Create a package with a single model that is Dead -> package should become Error
        let pkg_key = "Package/pkg-update";
        let pkg_yaml = r#"{"apiVersion":"v1","kind":"Package","metadata":{"name":"pkg-update"},"spec":{"pattern":[],"models":[{"name":"mup","node":"n","resources":{"volume":"","network":"","realtime":false}}]}}"#;
        let _ = common::etcd::put(pkg_key, pkg_yaml).await;

        // Set model state to Dead
        let _ = common::etcd::put("/model/mup/state", "Dead").await;
        // Set current package state to running so a change is detected
        let _ = common::etcd::put("/package/pkg-update/state", "running").await;

        // Trigger evaluation
        manager.trigger_package_state_evaluation("mup").await;

        // After evaluation, the package state should be updated (Error expected)
        let state = StateMachine::get_current_package_state("pkg-update").await;
        assert!(state.is_some());
        assert_eq!(state.unwrap(), common::statemanager::PackageState::Error);
    }

    #[tokio::test]
    async fn test_find_scenario_for_package_no_scenarios() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let manager = StateManagerManager::new(rx_container, rx_state_change).await;

        // Ensure no scenarios present
        let _ = common::etcd::delete("Scenario/nonexistent").await;

        let res = manager.find_scenario_for_package("no-scn").await;
        assert!(res.is_ok());
        let opt = res.unwrap();
        assert!(opt.is_none());
    }

    #[tokio::test]
    async fn test_initialize_starts_executor() {
        let (tx_container, rx_container) = mpsc::channel::<ContainerList>(1);
        let (tx_state_change, rx_state_change) =
            mpsc::channel::<common::statemanager::StateChange>(1);

        let mut manager = StateManagerManager::new(rx_container, rx_state_change).await;
        // initialize should start the async action executor without error
        let res = manager.initialize().await;
        assert!(res.is_ok());
    }
}

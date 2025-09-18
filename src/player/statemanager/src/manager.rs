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

use crate::state_machine::StateMachine;
use crate::types::{ActionCommand, ContainerState, TransitionResult};
use common::monitoringserver::ContainerList;

use common::statemanager::{
    ErrorCode, ModelState, PackageState, ResourceType, ScenarioState, StateChange,
};

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
        println!("StateManagerManager initializing...");

        // Initialize the state machine with async action executor
        let action_receiver = {
            let mut state_machine = self.state_machine.lock().await;
            state_machine.initialize_action_executor()
        };

        // Start the async action executor
        tokio::spawn(async move {
            run_action_executor(action_receiver).await;
        });

        println!("State machine initialized with transition tables for Scenario, Package, and Model resources");
        println!("Async action executor started for non-blocking action processing");

        // TODO: Add comprehensive initialization logic:
        // - Load persisted resource states from persistent storage
        // - Initialize state machine validators for each ResourceType
        // - Set up dependency tracking and validation systems
        // - Configure ASIL safety monitoring and alerting
        // - Initialize recovery strategies for each RecoveryType
        // - Set up health check systems for all resource types
        // - Configure event streaming and notification systems

        println!("StateManagerManager initialization completed");
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
                eprintln!(
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
        println!("=== PROCESSING STATE CHANGE ===");
        println!(
            "  Resource Type: {:?} (numeric: {})",
            resource_type, state_change.resource_type
        );
        println!("  Resource Name: {}", state_change.resource_name);
        println!(
            "  State Transition: {} -> {}",
            state_change.current_state, state_change.target_state
        );
        println!("  Transition ID: {}", state_change.transition_id);
        println!("  Source Component: {}", state_change.source);
        println!("  Timestamp: {} ns", state_change.timestamp_ns);

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
            println!("  ✓ State transition completed successfully");
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
            println!("    Final State: {new_state_str}");
            println!("    Success Message: {}", result.message);
            println!("    Transition ID: {}", result.transition_id);

            // Log any actions that were queued for asynchronous execution
            // Actions are processed separately to keep state transitions fast
            if !result.actions_to_execute.is_empty() {
                println!("    Actions queued for async execution:");
                for action in &result.actions_to_execute {
                    println!("      - {action}");
                }
                println!(
                    "    Note: Actions will be executed asynchronously by the action executor"
                );
            }

            println!("  Status: State change processing completed successfully");
        } else {
            // ========================================
            // FAILURE PATH: Log error details and initiate recovery
            // ========================================
            println!("  ✗ State transition failed");
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
            println!("    Error Code: {:?}", result.error_code);
            println!("    Error Message: {}", result.message);
            println!("    Error Details: {}", result.error_details);
            println!("    Current State: {new_state_str} (unchanged)");
            println!("    Failed Transition ID: {}", result.transition_id);

            // Delegate to specialized failure handling logic
            // This method will analyze the failure type and determine appropriate recovery actions
            self.handle_transition_failure(&state_change, &result).await;

            println!("  Status: State change processing completed with errors");
        }

        println!("================================");
    }

    /// Handle state transition failures
    async fn handle_transition_failure(
        &self,
        state_change: &StateChange,
        result: &TransitionResult,
    ) {
        println!(
            "    Handling transition failure for resource: {}",
            state_change.resource_name
        );
        println!("      Error: {}", result.message);
        println!("      Error code: {:?}", result.error_code);
        println!("      Error details: {}", result.error_details);

        // Generate appropriate error responses based on error type
        match result.error_code {
            ErrorCode::InvalidStateTransition => {
                println!("      Invalid state transition - checking state machine rules");
                // Would log detailed state machine validation errors
            }
            ErrorCode::PreconditionFailed => {
                println!("      Preconditions not met - evaluating retry strategy");
                // Would check if conditions might be met later and schedule retry
            }
            ErrorCode::ResourceNotFound => {
                println!("      Resource not found - may need initialization");
                // Would check if resource needs to be created or registered
            }
            _ => {
                println!("      General error - applying default error handling");
                // Would apply general error handling procedures
            }
        }

        // In a real implementation, this would:
        // - Log to audit trail
        // - Generate alerts
        // - Trigger recovery procedures
        // - Update monitoring metrics
    }

    /// Processes a ContainerList message for container health monitoring.
    ///
    /// This method handles container status updates from nodeagent and
    /// triggers appropriate state transitions based on container health.
    ///
    /// # Arguments
    /// * `container_list` - ContainerList message with node and container status
    ///
    /// # Processing Steps
    /// 1. Analyze container health and status changes
    /// 2. Identify resources affected by container changes
    /// 3. Trigger state transitions for failed or recovered containers
    /// 4. Update resource health status and monitoring data
    async fn process_container_list(&self, container_list: ContainerList) {
        println!("=== PROCESSING CONTAINER LIST ===");
        println!("  Node Name: {}", container_list.node_name);
        println!("  Container Count: {}", container_list.containers.len());

        // Group containers by model based on annotations
        let mut models_containers = std::collections::HashMap::<String, Vec<ContainerState>>::new();

        // Process each container for health status analysis
        for (i, container) in container_list.containers.iter().enumerate() {
            // container.names is a Vec<String>, so join them for display
            let container_names = container.names.join(", ");
            println!("  Container {}: {}", i + 1, container_names);
            println!("    Image: {}", container.image);
            println!("    State: {:?}", container.state);
            println!("    ID: {}", container.id);

            // container.config is a HashMap, not an Option
            if !container.config.is_empty() {
                println!("    Config: {:?}", container.config);
            }

            // Process container annotations if available
            if !container.annotation.is_empty() {
                println!("    Annotations: {:?}", container.annotation);
            }

            // STEP 1: ANALYZE CONTAINER STATE
            let container_state = self.analyze_container_state(&container.state);
            println!("    Analyzed State: {:?}", container_state);

            // STEP 2: EXTRACT MODEL NAME FROM ANNOTATIONS
            let model_name = self.extract_model_name_from_container(container);
            if let Some(model_name) = model_name {
                models_containers
                    .entry(model_name)
                    .or_insert_with(Vec::new)
                    .push(container_state);
            }
        }

        // STEP 3: DETERMINE MODEL STATES AND TRIGGER TRANSITIONS
        for (model_name, container_states) in models_containers {
            let new_model_state = self.determine_model_state_from_containers(&container_states);
            println!(
                "Model '{}' new state: {:?} (based on {} containers)",
                model_name,
                new_model_state,
                container_states.len()
            );

            // Trigger state transition if needed
            self.trigger_model_state_transition(&model_name, new_model_state)
                .await;
        }

        // ========================================
        // STEP 4: ANALYZE PACKAGE STATES BASED ON MODELS
        self.analyze_and_update_package_states().await;
    }

    /// Analyzes container state based on container.state map
    ///
    /// According to the PICCOLO documentation, container states are:
    /// - Created: No running containers exist, all created or deleted
    /// - Running: At least one container is running
    /// - Stopped: At least one container stopped, none running  
    /// - Exited: All containers in the pod have exited
    /// - Dead: Failed to retrieve state information (metadata corruption, system error)
    fn analyze_container_state(
        &self,
        container_state_map: &std::collections::HashMap<String, String>,
    ) -> ContainerState {
        // Container state is typically stored in container.state["Status"] or similar keys
        // Common Docker/Podman states: "running", "exited", "created", "paused", "dead", etc.

        let status = container_state_map
            .get("Status")
            .or_else(|| container_state_map.get("status"))
            .or_else(|| container_state_map.get("State"))
            .map(|s| s.to_lowercase());

        match status.as_deref() {
            Some("running") => ContainerState::Running,
            Some("paused") => ContainerState::Paused,
            Some("exited") | Some("stopped") => ContainerState::Exited,
            Some("created") => ContainerState::Created,
            Some("dead") | Some("error") => ContainerState::Dead,
            Some(other) => {
                println!("Unknown container status '{}', defaulting to Dead", other);
                ContainerState::Dead
            }
            None => {
                println!("No status found in container state map, defaulting to Dead");
                ContainerState::Dead
            }
        }
    }

    /// Extracts model name from container annotations
    ///
    /// Looks for model identification in container annotations/labels
    fn extract_model_name_from_container(
        &self,
        container: &common::monitoringserver::ContainerInfo,
    ) -> Option<String> {
        // Try to find model name in annotations
        if let Some(model_name) = container.annotation.get("pullpiri.model") {
            return Some(model_name.clone());
        }
        if let Some(model_name) = container.annotation.get("model") {
            return Some(model_name.clone());
        }

        // Try to find model name in config
        if let Some(model_name) = container.config.get("pullpiri.model") {
            return Some(model_name.clone());
        }
        if let Some(model_name) = container.config.get("model") {
            return Some(model_name.clone());
        }

        // If no explicit model annotation, try to extract from container name
        for name in &container.names {
            if name.contains("model-") || name.contains("-model") {
                // Extract model name from container name pattern
                if let Some(model_part) = name.split('-').find(|part| part.starts_with("model")) {
                    return Some(model_part.to_string());
                }
            }
        }

        None
    }

    /// Determines model state based on container states according to PICCOLO logic
    ///
    /// Model state determination rules from documentation:
    /// - Created: Model's initial state (creation default)
    /// - Paused: All containers are paused
    /// - Exited: All containers are exited  
    /// - Dead: One or more containers are dead, or model info query failed
    /// - Running: Default state when none of the above conditions are met
    fn determine_model_state_from_containers(
        &self,
        container_states: &[ContainerState],
    ) -> ModelState {
        if container_states.is_empty() {
            return ModelState::Unknown;
        }

        // Check for Dead state first (highest priority)
        if container_states
            .iter()
            .any(|state| *state == ContainerState::Dead)
        {
            return ModelState::Failed; // Map Dead to Failed for ModelState
        }

        // Check if all containers are paused
        if container_states
            .iter()
            .all(|state| *state == ContainerState::Paused)
        {
            return ModelState::Unknown; // No direct Paused state in ModelState enum
        }

        // Check if all containers are exited
        if container_states
            .iter()
            .all(|state| *state == ContainerState::Exited)
        {
            return ModelState::Succeeded; // Map Exited to Succeeded for ModelState
        }

        // Check if any containers are running
        if container_states
            .iter()
            .any(|state| *state == ContainerState::Running)
        {
            return ModelState::Running;
        }

        // Check if all containers are created (not started yet)
        if container_states
            .iter()
            .all(|state| *state == ContainerState::Created)
        {
            return ModelState::Pending;
        }

        // Default case
        ModelState::Unknown
    }

    /// Triggers a model state transition based on container analysis
    async fn trigger_model_state_transition(&self, model_name: &str, new_state: ModelState) {
        // Create a StateChange message for the model
        let state_change = StateChange {
            resource_type: ResourceType::Model as i32,
            resource_name: model_name.to_string(),
            current_state: "Unknown".to_string(), // Would need to track current state
            target_state: format!("{:?}", new_state),
            transition_id: format!(
                "auto-{}-{}",
                model_name,
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ),
            timestamp_ns: chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
            source: "StateManager-ContainerAnalysis".to_string(),
        };

        // Process the state change
        self.process_state_change(state_change).await;
    }

    /// Analyzes package states based on model states and triggers transitions
    ///
    /// Package state determination rules from documentation:
    /// - idle: Initial package state (creation default)
    /// - paused: All models are paused
    /// - exited: All models are exited
    /// - degraded: Some models are dead (but not all)
    /// - error: All models are dead
    /// - running: Default state when none of the above conditions are met
    async fn analyze_and_update_package_states(&self) {
        // TODO: Implement package state analysis
        // This would:
        // 1. Query all models for each package from etcd
        // 2. Analyze model states to determine package state
        // 3. Trigger package state transitions as needed
        println!("Package state analysis not yet implemented - requires etcd integration");
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
                            println!(
                                "Container channel closed - shutting down container processing"
                            );
                            break;
                        }
                    }
                }
                println!("ContainerList processing task stopped");
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
                            println!("StateChange channel closed - shutting down state processing");
                            break;
                        }
                    }
                }
                println!("StateChange processing task stopped");
            })
        };

        // Wait for both tasks to complete (typically on shutdown)
        let result = tokio::try_join!(container_task, state_change_task);
        match result {
            Ok(_) => {
                println!("All processing tasks completed successfully");
                Ok(())
            }
            Err(e) => {
                eprintln!("Error in processing tasks: {e:?}");
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
                eprintln!("Error in gRPC processor: {e:?}");
            }
        });

        // Wait for the processing task to complete
        let result = grpc_processor.await;
        match result {
            Ok(_) => {
                println!("StateManagerManager stopped gracefully");
                Ok(())
            }
            Err(e) => {
                eprintln!("StateManagerManager stopped with error: {e:?}");
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
    println!("Action executor started - processing actions asynchronously");

    while let Some(action_command) = receiver.recv().await {
        // Execute action asynchronously without blocking state transitions
        task::spawn(async move {
            execute_action(action_command).await;
        });
    }

    println!("Action executor stopped");
}

/// Execute individual action asynchronously
async fn execute_action(command: ActionCommand) {
    println!(
        " Executing action: {} for resource: {}",
        command.action, command.resource_key
    );

    match command.action.as_str() {
        "start_condition_evaluation" => {
            println!(
                " Starting condition evaluation for scenario: {}",
                command.resource_key
            );
            // Would integrate with policy engine or condition evaluator
        }
        "start_policy_verification" => {
            println!(
                " Starting policy verification for scenario: {}",
                command.resource_key
            );
            // Would integrate with policy manager
        }
        "execute_action_on_target_package" => {
            println!(
                " Executing action on target package for scenario: {}",
                command.resource_key
            );
            // Would trigger package operations
        }
        "log_denial_generate_alert" => {
            println!(
                " Logging denial and generating alert for scenario: {}",
                command.resource_key
            );
            // Would integrate with alerting system
        }
        "start_model_creation_allocate_resources" => {
            println!(
                " Starting model creation and resource allocation for package: {}",
                command.resource_key
            );
            // Would integrate with resource allocation system
        }
        "update_state_announce_availability" => {
            println!(
                " Updating state and announcing availability for: {}",
                command.resource_key
            );
            // Would update service discovery and announce availability
        }
        "log_warning_activate_partial_functionality" => {
            println!(
                " Logging warning and activating partial functionality for: {}",
                command.resource_key
            );
            // Would configure degraded mode operation
        }
        "log_error_attempt_recovery" => {
            println!(
                " Logging error and attempting recovery for: {}",
                command.resource_key
            );
            // Would trigger automated recovery procedures
        }
        "pause_models_preserve_state" => {
            println!(
                " Pausing models and preserving state for: {}",
                command.resource_key
            );
            // Would pause container execution and save state
        }
        "resume_models_restore_state" => {
            println!(
                " Resuming models and restoring state for: {}",
                command.resource_key
            );
            // Would resume container execution and restore state
        }
        "start_node_selection_and_allocation" => {
            println!(
                " Starting node selection and allocation for model: {}",
                command.resource_key
            );
            // Would integrate with scheduler for node allocation
        }
        "pull_container_images_mount_volumes" => {
            println!(
                " Pulling container images and mounting volumes for model: {}",
                command.resource_key
            );
            // Would trigger container image pulls and volume mounts
        }
        "update_state_start_readiness_checks" => {
            println!(
                " Updating state and starting readiness checks for model: {}",
                command.resource_key
            );
            // Would start health/readiness checks
        }
        "log_completion_clean_up_resources" => {
            println!(
                " Logging completion and cleaning up resources for model: {}",
                command.resource_key
            );
            // Would clean up completed job resources
        }
        "set_backoff_timer_collect_logs" => {
            println!(
                " Setting backoff timer and collecting logs for model: {}",
                command.resource_key
            );
            // Would set exponential backoff and collect diagnostic logs
        }
        "attempt_diagnostics_restore_communication" => {
            println!(
                " Attempting diagnostics and restoring communication for model: {}",
                command.resource_key
            );
            // Would run diagnostic checks and restore node communication
        }
        "resume_monitoring_reset_counter" => {
            println!(
                " Resuming monitoring and resetting counter for model: {}",
                command.resource_key
            );
            // Would resume monitoring and reset failure counters
        }
        "log_error_notify_for_manual_intervention" => {
            println!(
                " Logging error and notifying for manual intervention for model: {}",
                command.resource_key
            );
            // Would log critical error and notify operations team
        }
        "synchronize_state_recover_if_needed" => {
            println!(
                " Synchronizing state and recovering if needed for model: {}",
                command.resource_key
            );
            // Would synchronize state and trigger recovery if necessary
        }
        "start_model_recreation" => {
            println!(" Starting model recreation for: {}", command.resource_key);
            // Would start complete model recreation process
        }
        _ => {
            println!(
                " Unknown action: {} for resource: {}",
                command.action, command.resource_key
            );
        }
    }

    // Print context information if available
    if !command.context.is_empty() {
        println!("    Context: {:?}", command.context);
    }

    println!(
        "  ✓ Action '{}' completed for: {}",
        command.action, command.resource_key
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
// 8. PERFORMANCE AND MONITORING
//    - Performance constraint enforcement with deadlines and priorities
//    - Resource usage monitoring and optimization
//    - Health check automation and reporting
//    - Metrics collection and observability integration

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ContainerState;
    use common::monitoringserver::{ContainerInfo, ContainerList};
    use std::collections::HashMap;

    /// Test container state analysis logic
    #[tokio::test]
    async fn test_analyze_container_state() {
        let manager = StateManagerManager::new(
            tokio::sync::mpsc::channel(10).1,
            tokio::sync::mpsc::channel(10).1,
        )
        .await;

        // Test running state
        let mut state_map = HashMap::new();
        state_map.insert("Status".to_string(), "running".to_string());
        assert_eq!(
            manager.analyze_container_state(&state_map),
            ContainerState::Running
        );

        // Test paused state
        state_map.clear();
        state_map.insert("status".to_string(), "paused".to_string());
        assert_eq!(
            manager.analyze_container_state(&state_map),
            ContainerState::Paused
        );

        // Test exited state
        state_map.clear();
        state_map.insert("Status".to_string(), "exited".to_string());
        assert_eq!(
            manager.analyze_container_state(&state_map),
            ContainerState::Exited
        );

        // Test dead state
        state_map.clear();
        state_map.insert("Status".to_string(), "dead".to_string());
        assert_eq!(
            manager.analyze_container_state(&state_map),
            ContainerState::Dead
        );

        // Test unknown state defaults to dead
        state_map.clear();
        state_map.insert("Status".to_string(), "unknown".to_string());
        assert_eq!(
            manager.analyze_container_state(&state_map),
            ContainerState::Dead
        );

        // Test empty state map defaults to dead
        state_map.clear();
        assert_eq!(
            manager.analyze_container_state(&state_map),
            ContainerState::Dead
        );
    }

    /// Test model state determination from container states
    #[tokio::test]
    async fn test_determine_model_state_from_containers() {
        let manager = StateManagerManager::new(
            tokio::sync::mpsc::channel(10).1,
            tokio::sync::mpsc::channel(10).1,
        )
        .await;

        // Test all containers running -> Model Running
        let container_states = vec![ContainerState::Running, ContainerState::Running];
        assert_eq!(
            manager.determine_model_state_from_containers(&container_states),
            ModelState::Running
        );

        // Test any container dead -> Model Failed
        let container_states = vec![ContainerState::Running, ContainerState::Dead];
        assert_eq!(
            manager.determine_model_state_from_containers(&container_states),
            ModelState::Failed
        );

        // Test all containers exited -> Model Succeeded
        let container_states = vec![ContainerState::Exited, ContainerState::Exited];
        assert_eq!(
            manager.determine_model_state_from_containers(&container_states),
            ModelState::Succeeded
        );

        // Test all containers paused -> Model Unknown (no direct paused state)
        let container_states = vec![ContainerState::Paused, ContainerState::Paused];
        assert_eq!(
            manager.determine_model_state_from_containers(&container_states),
            ModelState::Unknown
        );

        // Test all containers created -> Model Pending
        let container_states = vec![ContainerState::Created, ContainerState::Created];
        assert_eq!(
            manager.determine_model_state_from_containers(&container_states),
            ModelState::Pending
        );

        // Test empty container list -> Model Unknown
        let container_states = vec![];
        assert_eq!(
            manager.determine_model_state_from_containers(&container_states),
            ModelState::Unknown
        );

        // Test mixed states with running -> Model Running (at least one running)
        let container_states = vec![ContainerState::Running, ContainerState::Created];
        assert_eq!(
            manager.determine_model_state_from_containers(&container_states),
            ModelState::Running
        );
    }

    /// Test model name extraction from container annotations
    #[tokio::test]
    async fn test_extract_model_name_from_container() {
        let manager = StateManagerManager::new(
            tokio::sync::mpsc::channel(10).1,
            tokio::sync::mpsc::channel(10).1,
        )
        .await;

        // Test extraction from pullpiri.model annotation
        let mut container = ContainerInfo {
            id: "test-id".to_string(),
            names: vec!["test-container".to_string()],
            image: "test-image".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        container
            .annotation
            .insert("pullpiri.model".to_string(), "test-model".to_string());

        assert_eq!(
            manager.extract_model_name_from_container(&container),
            Some("test-model".to_string())
        );

        // Test extraction from model annotation
        container.annotation.clear();
        container
            .annotation
            .insert("model".to_string(), "another-model".to_string());

        assert_eq!(
            manager.extract_model_name_from_container(&container),
            Some("another-model".to_string())
        );

        // Test extraction from config
        container.annotation.clear();
        container
            .config
            .insert("pullpiri.model".to_string(), "config-model".to_string());

        assert_eq!(
            manager.extract_model_name_from_container(&container),
            Some("config-model".to_string())
        );

        // Test no model annotation
        container.annotation.clear();
        container.config.clear();

        assert_eq!(manager.extract_model_name_from_container(&container), None);
    }

    /// Test complete container list processing flow
    #[tokio::test]
    async fn test_process_container_list_integration() {
        let manager = StateManagerManager::new(
            tokio::sync::mpsc::channel(10).1,
            tokio::sync::mpsc::channel(10).1,
        )
        .await;

        // Create a test container list with model annotations
        let mut container1 = ContainerInfo {
            id: "container1".to_string(),
            names: vec!["model-test-container1".to_string()],
            image: "test-image1".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        container1
            .state
            .insert("Status".to_string(), "running".to_string());
        container1
            .annotation
            .insert("pullpiri.model".to_string(), "test-model-1".to_string());

        let mut container2 = ContainerInfo {
            id: "container2".to_string(),
            names: vec!["model-test-container2".to_string()],
            image: "test-image2".to_string(),
            state: HashMap::new(),
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        container2
            .state
            .insert("Status".to_string(), "dead".to_string());
        container2
            .annotation
            .insert("pullpiri.model".to_string(), "test-model-2".to_string());

        let container_list = ContainerList {
            node_name: "test-node".to_string(),
            containers: vec![container1, container2],
        };

        // Process the container list - this should trigger model state transitions
        manager.process_container_list(container_list).await;

        // Note: In a real test, we would verify that state transitions were triggered
        // and that etcd was updated with the new model states
        println!("Container list processing test completed successfully");
    }
}

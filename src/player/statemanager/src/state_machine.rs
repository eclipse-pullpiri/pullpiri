/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! State Machine Implementation for PICCOLO Resource State Management
//!
//! This module implements the core state transition logic for Scenario, Package, and Model resources
//! according to the PICCOLO specification. It provides efficient data structures and algorithms
//! for managing state changes and enforcing the defined state transition tables.
//!
//! # Architecture Overview
//!
//! The state machine follows a table-driven approach where each resource type (Scenario, Package, Model)
//! has its own transition table defining valid state changes. The system supports:
//! - Conditional transitions based on resource state
//! - Action execution during state changes non-blocking
//! - Health monitoring and failure handling
//! - Backoff mechanisms for failed transitions
//!
//! # Usage Example
//!
//! ```rust
//! let mut state_machine = StateMachine::new();
//! let state_change = StateChange { /* ... */ };
//! let result = state_machine.process_state_change(state_change);
//! ```

use common::statemanager::{ErrorCode, ResourceType, StateChange};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};

// ========================================
// CONSTANTS AND CONFIGURATION
// ========================================

/// Default backoff duration for CrashLoopBackOff states
const BACKOFF_DURATION_SECS: u64 = 30;

/// Maximum consecutive failures before marking resource as unhealthy
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

// ========================================
// STATE DEFINITIONS
// ========================================

/// Scenario state enumeration aligned with PICCOLO specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScenarioState {
    Idle,
    Waiting,
    Playing,
    Allowed,
    Denied,
    Error,
}

impl ScenarioState {
    /// Convert enum to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ScenarioState::Idle => "idle",
            ScenarioState::Waiting => "waiting",
            ScenarioState::Playing => "playing",
            ScenarioState::Allowed => "allowed",
            ScenarioState::Denied => "denied",
            ScenarioState::Error => "error",
        }
    }

    /// Create enum from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "idle" => Some(ScenarioState::Idle),
            "waiting" => Some(ScenarioState::Waiting),
            "playing" => Some(ScenarioState::Playing),
            "allowed" => Some(ScenarioState::Allowed),
            "denied" => Some(ScenarioState::Denied),
            "error" => Some(ScenarioState::Error),
            _ => None,
        }
    }
}

/// Package state enumeration aligned with PICCOLO specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PackageState {
    NotAvailable,
    Initializing,
    Running,
    Degraded,
    Error,
    Paused,
    Updating,
}

impl PackageState {
    /// Convert enum to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PackageState::NotAvailable => "N/A",
            PackageState::Initializing => "initializing",
            PackageState::Running => "running",
            PackageState::Degraded => "degraded",
            PackageState::Error => "error",
            PackageState::Paused => "paused",
            PackageState::Updating => "updating",
        }
    }

    /// Create enum from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "N/A" => Some(PackageState::NotAvailable),
            "initializing" => Some(PackageState::Initializing),
            "running" => Some(PackageState::Running),
            "degraded" => Some(PackageState::Degraded),
            "error" => Some(PackageState::Error),
            "paused" => Some(PackageState::Paused),
            "updating" => Some(PackageState::Updating),
            _ => None,
        }
    }
}

/// Model state enumeration aligned with PICCOLO specification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ModelState {
    NotAvailable,
    Pending,
    ContainerCreating,
    Running,
    Failed,
    Succeeded,
    CrashLoopBackOff,
    Unknown,
}

impl ModelState {
    /// Convert enum to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelState::NotAvailable => "N/A",
            ModelState::Pending => "Pending",
            ModelState::ContainerCreating => "ContainerCreating",
            ModelState::Running => "Running",
            ModelState::Failed => "Failed",
            ModelState::Succeeded => "Succeeded",
            ModelState::CrashLoopBackOff => "CrashLoopBackOff",
            ModelState::Unknown => "Unknown",
        }
    }

    /// Create enum from string representation
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "N/A" => Some(ModelState::NotAvailable),
            "Pending" => Some(ModelState::Pending),
            "ContainerCreating" => Some(ModelState::ContainerCreating),
            "Running" => Some(ModelState::Running),
            "Failed" => Some(ModelState::Failed),
            "Succeeded" => Some(ModelState::Succeeded),
            "CrashLoopBackOff" => Some(ModelState::CrashLoopBackOff),
            "Unknown" => Some(ModelState::Unknown),
            _ => None,
        }
    }
}

// ========================================
// CORE DATA STRUCTURES
// ========================================

/// Action execution command for async processing
#[derive(Debug, Clone)]
pub struct ActionCommand {
    pub action: String,
    pub resource_key: String,
    pub resource_type: ResourceType,
    pub transition_id: String,
    pub context: HashMap<String, String>,
}

/// Represents a state transition in the state machine
#[derive(Debug, Clone, PartialEq)]
pub struct StateTransition {
    pub from_state: String,
    pub event: String,
    pub to_state: String,
    pub condition: Option<String>,
    pub action: String,
}

/// Health status tracking for resources
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub status_message: String,
    pub last_check: Instant,
    pub consecutive_failures: u32,
}

/// Represents the current state of a resource with metadata
#[derive(Debug, Clone)]
pub struct ResourceState {
    pub resource_type: ResourceType,
    pub resource_name: String,
    pub current_state: String,
    pub desired_state: Option<String>,
    pub last_transition_time: Instant,
    pub transition_count: u64,
    pub metadata: HashMap<String, String>,
    pub health_status: HealthStatus,
}

/// Result of a state transition attempt - aligned with proto StateChangeResponse
#[derive(Debug, Clone)]
pub struct TransitionResult {
    pub new_state: String,
    pub error_code: ErrorCode,
    pub message: String,
    pub actions_to_execute: Vec<String>,
    pub transition_id: String,
    pub error_details: String,
}

impl TransitionResult {
    /// Check if the transition was successful
    pub fn is_success(&self) -> bool {
        matches!(self.error_code, ErrorCode::Success)
    }

    /// Check if the transition failed
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Convert TransitionResult to StateChangeResponse for proto compatibility
    pub fn to_state_change_response(&self) -> common::statemanager::StateChangeResponse {
        common::statemanager::StateChangeResponse {
            message: self.message.clone(),
            transition_id: self.transition_id.clone(),
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as i64,
            error_code: self.error_code as i32,
            error_details: self.error_details.clone(),
        }
    }
}

/// Core state machine implementation for PICCOLO resource management
///
/// This is the central component that manages all resource state transitions,
/// enforces business rules, and maintains the current state of all resources
/// in the system.
///
/// # Design Principles
/// - **Deterministic**: Same inputs always produce same outputs
/// - **Auditable**: All state changes are tracked with timestamps
/// - **Resilient**: Handles failures gracefully with backoff mechanisms
/// - **Extensible**: New resource types can be added with their own transition tables
///
/// # Thread Safety
/// This implementation is not thread-safe. External synchronization is required
/// for concurrent access across multiple threads.
pub struct StateMachine {
    /// State transition tables indexed by resource type
    ///
    /// Each resource type has its own set of valid transitions, allowing
    /// for type-specific state management rules and behaviors.
    transition_tables: HashMap<ResourceType, Vec<StateTransition>>,

    /// Current state tracking for all managed resources
    ///
    /// Resources are keyed by a unique identifier (typically resource name)
    /// and contain complete state information including metadata and health status.
    resource_states: HashMap<String, ResourceState>,

    /// Backoff timers for CrashLoopBackOff and retry management
    ///
    /// Tracks when resources that have failed transitions can be retried,
    /// implementing exponential backoff to prevent resource thrashing.
    backoff_timers: HashMap<String, Instant>,

    /// Action command sender for async execution
    action_sender: Option<mpsc::UnboundedSender<ActionCommand>>,
}

impl StateMachine {
    /// Creates a new StateMachine with predefined transition tables
    ///
    /// Initializes the state machine with empty resource tracking and
    /// populates the transition tables for all supported resource types.
    ///
    /// # Returns
    /// A fully configured StateMachine ready to process state changes
    ///
    /// # Examples
    /// ```rust
    /// let state_machine = StateMachine::new();
    /// ```
    pub fn new() -> Self {
        let mut state_machine = StateMachine {
            transition_tables: HashMap::new(),
            resource_states: HashMap::new(),
            backoff_timers: HashMap::new(),
            action_sender: None,
        };

        // Initialize transition tables for each resource type
        state_machine.initialize_scenario_transitions();
        state_machine.initialize_package_transitions();
        state_machine.initialize_model_transitions();

        state_machine
    }

    /// Initialize async action executor
    pub fn initialize_action_executor(&mut self) -> mpsc::UnboundedReceiver<ActionCommand> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.action_sender = Some(sender);
        receiver
    }

    // ========================================
    // STATE TRANSITION TABLE INITIALIZATION
    // ========================================

    /// Initialize the state transition table for Scenario resources
    ///
    /// Populates the transition table with all valid state changes for Scenario resources
    /// according to the PICCOLO specification. This includes transitions for:
    /// - Creation and initialization
    /// - Activation and deactivation
    /// - Error handling and recovery
    /// - Cleanup and termination
    ///
    /// # Implementation Note
    /// This method should define transitions like:
    /// - "Inactive" -> "Active" on "activate" event
    /// - "Active" -> "Inactive" on "deactivate" event
    /// - Any state -> "Failed" on "error" event
    fn initialize_scenario_transitions(&mut self) {
        let scenario_transitions = vec![
            StateTransition {
                from_state: ScenarioState::Idle.as_str().to_string(),
                event: "scenario_activation".to_string(),
                to_state: ScenarioState::Waiting.as_str().to_string(),
                condition: None,
                action: "start_condition_evaluation".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Waiting.as_str().to_string(),
                event: "condition_met".to_string(),
                to_state: ScenarioState::Allowed.as_str().to_string(),
                condition: None,
                action: "start_policy_verification".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Allowed.as_str().to_string(),
                event: "policy_verification_success".to_string(),
                to_state: ScenarioState::Playing.as_str().to_string(),
                condition: None,
                action: "execute_action_on_target_package".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Allowed.as_str().to_string(),
                event: "policy_verification_failure".to_string(),
                to_state: ScenarioState::Denied.as_str().to_string(),
                condition: None,
                action: "log_denial_generate_alert".to_string(),
            },
        ];

        self.transition_tables
            .insert(ResourceType::Scenario, scenario_transitions);
    }

    /// Initialize the state transition table for Package resources
    ///
    /// Configures all valid state transitions for Package resources, including:
    /// - Download and installation states
    /// - Verification and validation phases
    /// - Update and rollback mechanisms
    /// - Cleanup and removal operations
    ///
    /// # Implementation Note
    /// Package transitions typically involve more complex workflows due to
    /// dependency management and rollback requirements.
    fn initialize_package_transitions(&mut self) {
        let package_transitions = vec![
            StateTransition {
                from_state: PackageState::NotAvailable.as_str().to_string(),
                event: "launch_request".to_string(),
                to_state: PackageState::Initializing.as_str().to_string(),
                condition: None,
                action: "start_model_creation_allocate_resources".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing.as_str().to_string(),
                event: "initialization_complete".to_string(),
                to_state: PackageState::Running.as_str().to_string(),
                condition: Some("all_models_normal".to_string()),
                action: "update_state_announce_availability".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing.as_str().to_string(),
                event: "partial_initialization_failure".to_string(),
                to_state: PackageState::Degraded.as_str().to_string(),
                condition: Some("critical_models_normal".to_string()),
                action: "log_warning_activate_partial_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing.as_str().to_string(),
                event: "critical_initialization_failure".to_string(),
                to_state: PackageState::Error.as_str().to_string(),
                condition: Some("critical_models_failed".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running.as_str().to_string(),
                event: "model_issue_detected".to_string(),
                to_state: PackageState::Degraded.as_str().to_string(),
                condition: Some("non_critical_model_issues".to_string()),
                action: "log_warning_maintain_partial_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running.as_str().to_string(),
                event: "critical_issue_detected".to_string(),
                to_state: PackageState::Error.as_str().to_string(),
                condition: Some("critical_model_issues".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running.as_str().to_string(),
                event: "pause_request".to_string(),
                to_state: PackageState::Paused.as_str().to_string(),
                condition: None,
                action: "pause_models_preserve_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded.as_str().to_string(),
                event: "model_recovery".to_string(),
                to_state: PackageState::Running.as_str().to_string(),
                condition: Some("all_models_recovered".to_string()),
                action: "update_state_restore_full_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded.as_str().to_string(),
                event: "additional_model_issues".to_string(),
                to_state: PackageState::Error.as_str().to_string(),
                condition: Some("critical_models_affected".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded.as_str().to_string(),
                event: "pause_request".to_string(),
                to_state: PackageState::Paused.as_str().to_string(),
                condition: None,
                action: "pause_models_preserve_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Error.as_str().to_string(),
                event: "recovery_successful".to_string(),
                to_state: PackageState::Running.as_str().to_string(),
                condition: Some("depends_on_recovery_level".to_string()),
                action: "update_state_announce_functionality_restoration".to_string(),
            },
            StateTransition {
                from_state: PackageState::Paused.as_str().to_string(),
                event: "resume_request".to_string(),
                to_state: PackageState::Running.as_str().to_string(),
                condition: Some("depends_on_previous_state".to_string()),
                action: "resume_models_restore_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running.as_str().to_string(),
                event: "update_request".to_string(),
                to_state: PackageState::Updating.as_str().to_string(),
                condition: None,
                action: "start_update_process".to_string(),
            },
            StateTransition {
                from_state: PackageState::Updating.as_str().to_string(),
                event: "update_successful".to_string(),
                to_state: PackageState::Running.as_str().to_string(),
                condition: None,
                action: "activate_new_version_update_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Updating.as_str().to_string(),
                event: "update_failed".to_string(),
                to_state: PackageState::Error.as_str().to_string(),
                condition: Some("depends_on_rollback_settings".to_string()),
                action: "rollback_or_error_handling".to_string(),
            },
        ];

        self.transition_tables
            .insert(ResourceType::Package, package_transitions);
    }

    /// Initialize the state transition table for Model resources
    ///
    /// Sets up state transitions specific to Model resources, covering:
    /// - Model loading and initialization
    /// - Training and inference states
    /// - Model versioning and updates
    /// - Resource allocation and cleanup
    ///
    /// # Implementation Note
    /// Model transitions may include resource-intensive operations and
    /// should account for memory and compute constraints.
    fn initialize_model_transitions(&mut self) {
        let model_transitions = vec![
            StateTransition {
                from_state: ModelState::NotAvailable.as_str().to_string(),
                event: "creation_request".to_string(),
                to_state: ModelState::Pending.as_str().to_string(),
                condition: None,
                action: "start_node_selection_and_allocation".to_string(),
            },
            StateTransition {
                from_state: ModelState::Pending.as_str().to_string(),
                event: "node_allocation_complete".to_string(),
                to_state: ModelState::ContainerCreating.as_str().to_string(),
                condition: Some("sufficient_resources".to_string()),
                action: "pull_container_images_mount_volumes".to_string(),
            },
            StateTransition {
                from_state: ModelState::Pending.as_str().to_string(),
                event: "node_allocation_failed".to_string(),
                to_state: ModelState::Failed.as_str().to_string(),
                condition: Some("timeout_or_error".to_string()),
                action: "log_error_retry_or_reschedule".to_string(),
            },
            StateTransition {
                from_state: ModelState::ContainerCreating.as_str().to_string(),
                event: "container_creation_complete".to_string(),
                to_state: ModelState::Running.as_str().to_string(),
                condition: Some("all_containers_started".to_string()),
                action: "update_state_start_readiness_checks".to_string(),
            },
            StateTransition {
                from_state: ModelState::ContainerCreating.as_str().to_string(),
                event: "container_creation_failed".to_string(),
                to_state: ModelState::Failed.as_str().to_string(),
                condition: None,
                action: "log_error_retry_or_reschedule".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running.as_str().to_string(),
                event: "temporary_task_complete".to_string(),
                to_state: ModelState::Succeeded.as_str().to_string(),
                condition: Some("one_time_task".to_string()),
                action: "log_completion_clean_up_resources".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running.as_str().to_string(),
                event: "container_termination".to_string(),
                to_state: ModelState::Failed.as_str().to_string(),
                condition: Some("unexpected_termination".to_string()),
                action: "log_error_evaluate_automatic_restart".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running.as_str().to_string(),
                event: "repeated_crash_detection".to_string(),
                to_state: ModelState::CrashLoopBackOff.as_str().to_string(),
                condition: Some("consecutive_restart_failures".to_string()),
                action: "set_backoff_timer_collect_logs".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running.as_str().to_string(),
                event: "monitoring_failure".to_string(),
                to_state: ModelState::Unknown.as_str().to_string(),
                condition: Some("node_communication_issues".to_string()),
                action: "attempt_diagnostics_restore_communication".to_string(),
            },
            StateTransition {
                from_state: ModelState::CrashLoopBackOff.as_str().to_string(),
                event: "backoff_time_elapsed".to_string(),
                to_state: ModelState::Running.as_str().to_string(),
                condition: Some("restart_successful".to_string()),
                action: "resume_monitoring_reset_counter".to_string(),
            },
            StateTransition {
                from_state: ModelState::CrashLoopBackOff.as_str().to_string(),
                event: "maximum_retries_exceeded".to_string(),
                to_state: ModelState::Failed.as_str().to_string(),
                condition: Some("retry_limit_reached".to_string()),
                action: "log_error_notify_for_manual_intervention".to_string(),
            },
            StateTransition {
                from_state: ModelState::Unknown.as_str().to_string(),
                event: "state_check_recovered".to_string(),
                to_state: ModelState::Running.as_str().to_string(),
                condition: Some("depends_on_actual_state".to_string()),
                action: "synchronize_state_recover_if_needed".to_string(),
            },
            StateTransition {
                from_state: ModelState::Failed.as_str().to_string(),
                event: "manual_automatic_recovery".to_string(),
                to_state: ModelState::Pending.as_str().to_string(),
                condition: Some("according_to_restart_policy".to_string()),
                action: "start_model_recreation".to_string(),
            },
        ];

        self.transition_tables
            .insert(ResourceType::Model, model_transitions);
    }

    // ========================================
    // CORE STATE PROCESSING
    // ========================================
    /// Process a state change request with non-blocking action execution
    pub fn process_state_change(&mut self, state_change: StateChange) -> TransitionResult {
        // Validate input parameters
        if let Err(validation_error) = self.validate_state_change(&state_change) {
            return TransitionResult {
                new_state: state_change.current_state.clone(),
                error_code: ErrorCode::InvalidRequest,
                message: format!("Invalid state change request: {}", validation_error),
                actions_to_execute: vec![],
                transition_id: state_change.transition_id.clone(),
                error_details: validation_error,
            };
        }

        // Convert i32 to ResourceType enum
        let resource_type = match ResourceType::try_from(state_change.resource_type) {
            Ok(rt) => rt,
            Err(_) => {
                return TransitionResult {
                    new_state: state_change.current_state.clone(),
                    error_code: ErrorCode::InvalidStateTransition,
                    message: format!("Invalid resource type: {}", state_change.resource_type),
                    actions_to_execute: vec![],
                    transition_id: state_change.transition_id.clone(),
                    error_details: format!(
                        "Unsupported resource type ID: {}",
                        state_change.resource_type
                    ),
                };
            }
        };

        let resource_key = self.generate_resource_key(resource_type, &state_change.resource_name);

        // Get current state - use provided current_state for new resources
        let current_state = match self.resource_states.get(&resource_key) {
            Some(existing_state) => existing_state.current_state.clone(),
            None => {
                // For new resources, use the current_state from the StateChange message
                state_change.current_state.clone()
            }
        };

        // Check for special CrashLoopBackOff handling
        if current_state == ModelState::CrashLoopBackOff.as_str() {
            if let Some(backoff_time) = self.backoff_timers.get(&resource_key) {
                if backoff_time.elapsed() < Duration::from_secs(BACKOFF_DURATION_SECS) {
                    return TransitionResult {
                        new_state: current_state,
                        error_code: ErrorCode::PreconditionFailed,
                        message: "Resource is in backoff period".to_string(),
                        actions_to_execute: vec![],
                        transition_id: state_change.transition_id.clone(),
                        error_details: "Backoff timer has not elapsed yet".to_string(),
                    };
                }
            }
        }

        // Find valid transition
        let transition_event =
            self.infer_event_from_states(&current_state, &state_change.target_state);

        if let Some(transition) = self.find_valid_transition(
            resource_type,
            &current_state,
            &transition_event,
            &state_change.target_state,
        ) {
            // Check conditions if any
            if let Some(ref condition) = transition.condition {
                if !self.evaluate_condition(condition, &state_change) {
                    return TransitionResult {
                        new_state: current_state,
                        error_code: ErrorCode::PreconditionFailed,
                        message: format!("Condition not met: {}", condition),
                        actions_to_execute: vec![],
                        transition_id: state_change.transition_id.clone(),
                        error_details: format!("Failed condition evaluation: {}", condition),
                    };
                }
            }

            // Execute transition - this is immediate and non-blocking
            self.update_resource_state(
                &resource_key,
                &state_change,
                &transition.to_state,
                resource_type,
            );

            // **NON-BLOCKING ACTION EXECUTION** - Queue action for async execution
            if let Some(ref sender) = self.action_sender {
                let action_command = ActionCommand {
                    action: transition.action.clone(),
                    resource_key: resource_key.clone(),
                    resource_type,
                    transition_id: state_change.transition_id.clone(),
                    context: self.build_action_context(&state_change, &transition),
                };

                // Send action for async execution (non-blocking)
                if let Err(e) = sender.send(action_command) {
                    eprintln!("Warning: Failed to queue action for execution: {}", e);
                }
            }

            // Create successful transition result
            let transition_result = TransitionResult {
                new_state: transition.to_state.clone(),
                error_code: ErrorCode::Success,
                message: format!("Successfully transitioned to {}", transition.to_state),
                actions_to_execute: vec![transition.action.clone()],
                transition_id: state_change.transition_id.clone(),
                error_details: String::new(),
            };

            self.update_health_status(&resource_key, &transition_result);

            // Handle special state-specific logic
            if transition.to_state == ModelState::CrashLoopBackOff.as_str() {
                self.backoff_timers
                    .insert(resource_key.clone(), Instant::now());
            }

            transition_result
        } else {
            let transition_result = TransitionResult {
                new_state: current_state.clone(),
                error_code: ErrorCode::InvalidStateTransition,
                message: format!(
                    "No valid transition from {} to {} for resource type {:?}",
                    current_state, state_change.target_state, resource_type
                ),
                actions_to_execute: vec![],
                transition_id: state_change.transition_id.clone(),
                error_details: format!(
                    "Invalid state transition attempted: {} -> {}",
                    current_state, state_change.target_state
                ),
            };

            self.update_health_status(&resource_key, &transition_result);
            transition_result
        }
    }

    // ========================================
    // VALIDATION AND UTILITY METHODS
    // ========================================

    /// Find a valid transition rule for the given parameters
    ///
    /// Searches the appropriate transition table for a rule that matches
    /// the specified resource type, current state, event, and target state.
    ///
    /// # Parameters
    /// - `resource_type`: The type of resource to check transitions for
    /// - `from_state`: The current state of the resource
    /// - `event`: The event triggering the transition
    /// - `to_state`: The desired target state
    ///
    /// # Returns
    /// - `Some(StateTransition)`: If a valid transition rule is found
    /// - `None`: If no valid transition exists for the given parameters
    ///
    /// # Implementation Details
    /// This method performs exact matching on all transition parameters.
    /// Wildcard or pattern matching is not currently supported.
    fn find_valid_transition(
        &self,
        resource_type: ResourceType,
        from_state: &str,
        event: &str,
        to_state: &str,
    ) -> Option<StateTransition> {
        if let Some(transitions) = self.transition_tables.get(&resource_type) {
            for transition in transitions {
                if transition.from_state == from_state
                    && transition.event == event
                    && transition.to_state == to_state
                {
                    return Some(transition.clone());
                }
            }
        }
        None
    }

    /// Validate state change request parameters
    fn validate_state_change(&self, state_change: &StateChange) -> Result<(), String> {
        if state_change.resource_name.trim().is_empty() {
            return Err("Resource name cannot be empty".to_string());
        }

        if state_change.transition_id.trim().is_empty() {
            return Err("Transition ID cannot be empty".to_string());
        }

        if state_change.current_state == state_change.target_state {
            return Err("Current and target states cannot be the same".to_string());
        }

        if state_change.source.trim().is_empty() {
            return Err("Source cannot be empty".to_string());
        }

        Ok(())
    }

    /// Generate a unique resource key
    fn generate_resource_key(&self, resource_type: ResourceType, resource_name: &str) -> String {
        format!(
            "{}::{}",
            self.resource_type_to_string(resource_type),
            resource_name
        )
    }

    /// Build context for action execution
    fn build_action_context(
        &self,
        state_change: &StateChange,
        transition: &StateTransition,
    ) -> HashMap<String, String> {
        let mut context = HashMap::new();
        context.insert("from_state".to_string(), transition.from_state.clone());
        context.insert("to_state".to_string(), transition.to_state.clone());
        context.insert("event".to_string(), transition.event.clone());
        context.insert(
            "resource_name".to_string(),
            state_change.resource_name.clone(),
        );
        context.insert("source".to_string(), state_change.source.clone());
        context.insert(
            "timestamp_ns".to_string(),
            state_change.timestamp_ns.to_string(),
        );

        context
    }

    /// Updates health status based on transition result
    fn update_health_status(&mut self, resource_key: &str, transition_result: &TransitionResult) {
        if let Some(resource_state) = self.resource_states.get_mut(resource_key) {
            let now = Instant::now();
            resource_state.health_status.last_check = now;

            if transition_result.is_success() {
                resource_state.health_status.healthy = true;
                resource_state.health_status.consecutive_failures = 0;
                resource_state.health_status.status_message = "Healthy".to_string();
            } else {
                resource_state.health_status.consecutive_failures += 1;
                resource_state.health_status.status_message = transition_result.message.clone();

                // Mark as unhealthy if we have multiple consecutive failures
                if resource_state.health_status.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                    resource_state.health_status.healthy = false;
                }
            }
        }
    }

    /// Infer the appropriate event type from state transition
    ///
    /// When an explicit event is not provided, this method attempts to
    /// determine the most appropriate event based on the current and target states.
    ///
    /// # Parameters
    /// - `current_state`: The current state of the resource
    /// - `target_state`: The desired target state
    ///
    /// # Returns
    /// A string representing the inferred event type
    ///
    /// # Examples
    /// - "Inactive" -> "Active" might infer "activate"
    /// - "Running" -> "Stopped" might infer "stop"
    /// - Any state -> "Failed" might infer "error"
    ///
    /// # Fallback Behavior
    /// If no specific event can be inferred, returns a generic event name
    /// based on the target state (e.g., "transition_to_active").
    fn infer_event_from_states(&self, current_state: &str, target_state: &str) -> String {
        match (current_state, target_state) {
            // Scenario events
            ("idle", "waiting") => "scenario_activation".to_string(),
            ("waiting", "allowed") => "condition_met".to_string(),
            ("allowed", "playing") => "policy_verification_success".to_string(),
            ("allowed", "denied") => "policy_verification_failure".to_string(),

            // Package events
            ("N/A", "initializing") => "launch_request".to_string(),
            ("initializing", "running") => "initialization_complete".to_string(),
            ("initializing", "degraded") => "partial_initialization_failure".to_string(),
            ("initializing", "error") => "critical_initialization_failure".to_string(),
            ("running", "degraded") => "model_issue_detected".to_string(),
            ("running", "error") => "critical_issue_detected".to_string(),
            ("running", "paused") => "pause_request".to_string(),
            ("running", "updating") => "update_request".to_string(),
            ("degraded", "running") => "model_recovery".to_string(),
            ("degraded", "error") => "additional_model_issues".to_string(),
            ("degraded", "paused") => "pause_request".to_string(),
            ("error", "running") => "recovery_successful".to_string(),
            ("paused", "running") => "resume_request".to_string(),
            ("updating", "running") => "update_successful".to_string(),
            ("updating", "error") => "update_failed".to_string(),

            // Model events
            ("N/A", "Pending") => "creation_request".to_string(),
            ("Pending", "ContainerCreating") => "node_allocation_complete".to_string(),
            ("Pending", "Failed") => "node_allocation_failed".to_string(),
            ("ContainerCreating", "Running") => "container_creation_complete".to_string(),
            ("ContainerCreating", "Failed") => "container_creation_failed".to_string(),
            ("Running", "Succeeded") => "temporary_task_complete".to_string(),
            ("Running", "Failed") => "container_termination".to_string(),
            ("Running", "CrashLoopBackOff") => "repeated_crash_detection".to_string(),
            ("Running", "Unknown") => "monitoring_failure".to_string(),
            ("CrashLoopBackOff", "Running") => "backoff_time_elapsed".to_string(),
            ("CrashLoopBackOff", "Failed") => "maximum_retries_exceeded".to_string(),
            ("Unknown", "Running") => "state_check_recovered".to_string(),
            ("Failed", "Pending") => "manual_automatic_recovery".to_string(),

            // Default case
            _ => format!("transition_{}_{}", current_state, target_state),
        }
    }

    /// Evaluate whether a transition condition is satisfied
    ///
    /// Processes conditional logic attached to state transitions to determine
    /// if the transition should be allowed to proceed.
    ///
    /// # Parameters
    /// - `condition`: The condition string to evaluate (e.g., "resource_count > 0")
    /// - `_state_change`: The state change request providing context for evaluation
    ///
    /// # Returns
    /// - `true`: If the condition is satisfied or no condition exists
    /// - `false`: If the condition fails evaluation
    ///
    /// # Supported Conditions
    /// The condition language should support:
    /// - Resource property comparisons
    /// - Metadata key existence checks
    /// - Numeric and string comparisons
    /// - Boolean logic operators
    ///
    /// # Error Handling
    /// Malformed conditions should be logged and default to `false` for safety.

    fn evaluate_condition(&self, condition: &str, _state_change: &StateChange) -> bool {
        // TODO: Implement real condition evaluation logic
        match condition {
            "all_models_normal" => true,
            "critical_models_normal" => true,
            "critical_models_failed" => false,
            "non_critical_model_issues" => true,
            "critical_model_issues" => false,
            "all_models_recovered" => true,
            "critical_models_affected" => false,
            "depends_on_recovery_level" => true,
            "depends_on_previous_state" => true,
            "depends_on_rollback_settings" => true,
            "sufficient_resources" => true,
            "timeout_or_error" => false,
            "all_containers_started" => true,
            "one_time_task" => true,
            "unexpected_termination" => false,
            "consecutive_restart_failures" => false,
            "node_communication_issues" => false,
            "restart_successful" => true,
            "retry_limit_reached" => false,
            "depends_on_actual_state" => true,
            "according_to_restart_policy" => true,
            _ => true, // Default to allow transition for unknown conditions
        }
    }

    /// Update the internal resource state after a successful transition
    ///
    /// Performs all necessary bookkeeping when a state transition succeeds,
    /// including updating timestamps, incrementing counters, and managing metadata.
    ///
    /// # Parameters
    /// - `resource_key`: Unique identifier for the resource
    /// - `state_change`: The original state change request
    /// - `new_state`: The state the resource has transitioned to
    /// - `resource_type`: The type of the resource
    ///
    /// # Side Effects
    /// - Updates or creates the resource state entry
    /// - Increments transition counter
    /// - Updates last transition timestamp
    /// - Clears any active backoff timers on successful transition
    /// - Updates health status if applicable
    fn update_resource_state(
        &mut self,
        resource_key: &str,
        state_change: &StateChange,
        new_state: &str,
        resource_type: ResourceType,
    ) {
        let now = Instant::now();

        let resource_state = self
            .resource_states
            .entry(resource_key.to_string())
            .or_insert_with(|| ResourceState {
                resource_type,
                resource_name: state_change.resource_name.clone(),
                current_state: state_change.current_state.clone(),
                desired_state: Some(state_change.target_state.clone()),
                last_transition_time: now,
                transition_count: 0,
                metadata: HashMap::new(),
                health_status: HealthStatus {
                    healthy: true,
                    status_message: "Healthy".to_string(),
                    last_check: now,
                    consecutive_failures: 0,
                },
            });

        resource_state.current_state = new_state.to_string();
        resource_state.last_transition_time = now;
        resource_state.transition_count += 1;
        resource_state.metadata.insert(
            "last_transition_id".to_string(),
            state_change.transition_id.clone(),
        );
        resource_state
            .metadata
            .insert("source".to_string(), state_change.source.clone());
    }

    // ========================================
    // PUBLIC QUERY METHODS
    // ========================================

    /// Retrieve the current state information for a specific resource
    ///
    /// Provides read-only access to the complete state information for
    /// a resource, including metadata and health status.
    ///
    /// # Parameters
    /// - `resource_name`: The unique name of the resource
    /// - `resource_type`: The type of the resource (for validation)
    ///
    /// # Returns
    /// - `Some(&ResourceState)`: If the resource exists and types match
    /// - `None`: If the resource doesn't exist or type mismatch
    ///
    /// # Usage
    /// This method is primarily used for:
    /// - Status queries from external systems
    /// - Health check implementations
    /// - Audit and monitoring purposes
    pub fn get_resource_state(
        &self,
        resource_name: &str,
        resource_type: ResourceType,
    ) -> Option<&ResourceState> {
        let resource_key = self.generate_resource_key(resource_type, resource_name);
        self.resource_states.get(&resource_key)
    }

    /// List all resources currently in a specific state
    ///
    /// Provides a filtered view of all managed resources based on their
    /// current state, optionally filtered by resource type.
    ///
    /// # Parameters
    /// - `resource_type`: Optional filter for resource type (None = all types)
    /// - `state`: The state to filter by
    ///
    /// # Returns
    /// A vector of references to all matching resource states
    ///
    /// # Performance Note
    /// This method performs a linear scan of all resources. For large numbers
    /// of resources, consider implementing indexed lookups by state.
    ///
    /// # Usage Examples
    /// - Find all failed resources: `list_resources_by_state(None, "Failed")`
    /// - Find active scenarios: `list_resources_by_state(Some(ResourceType::Scenario), "Active")`
    pub fn list_resources_by_state(
        &self,
        resource_type: Option<ResourceType>,
        state: &str,
    ) -> Vec<&ResourceState> {
        self.resource_states
            .values()
            .filter(|resource| {
                resource.current_state == state
                    && (resource_type.is_none() || resource_type == Some(resource.resource_type))
            })
            .collect()
    }

    /// Convert ResourceType enum to string representation for logging
    ///
    /// Provides consistent string representation of resource types for
    /// logging, debugging, and external API responses.
    ///
    /// # Parameters
    /// - `resource_type`: The resource type to convert
    ///
    /// # Returns
    /// A static string slice representing the resource type
    ///
    /// # Design Note
    /// Using static string slices avoids unnecessary string allocations
    /// for this frequently-called utility function.
    fn resource_type_to_string(&self, resource_type: ResourceType) -> &'static str {
        match resource_type {
            ResourceType::Scenario => "Scenario",
            ResourceType::Package => "Package",
            ResourceType::Model => "Model",
            ResourceType::Volume => "Volume",
            ResourceType::Network => "Network",
            ResourceType::Node => "Node",
            _ => "Unknown",
        }
    }
}

/// Default implementation that creates a new StateMachine
///
/// Provides a convenient way to create a StateMachine with default
/// configuration using the `Default` trait.
impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

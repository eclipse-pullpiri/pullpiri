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

use crate::types::{
    ActionCommand, ContainerState, HealthStatus, ResourceState, StateTransition, TransitionResult,
};
use common::spec::artifact::Artifact;
use common::statemanager::{
    ErrorCode, ModelState, PackageState, ResourceType, ScenarioState, StateChange,
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::Instant;

// ========================================
// CONSTANTS AND CONFIGURATION
// ========================================

/// Default backoff duration for CrashLoopBackOff states
#[allow(dead_code)]
const BACKOFF_DURATION_SECS: u64 = 30;

/// Maximum consecutive failures before marking resource as unhealthy
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

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
    #[allow(dead_code)]
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
                from_state: ScenarioState::Idle as i32,
                event: "scenario_activation".to_string(),
                to_state: ScenarioState::Waiting as i32,
                condition: None,
                action: "start_condition_evaluation".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Waiting as i32,
                event: "condition_met".to_string(),
                to_state: ScenarioState::Satisfied as i32,
                condition: None,
                action: "start_policy_verification".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Satisfied as i32,
                event: "policy_verification_success".to_string(),
                to_state: ScenarioState::Allowed as i32,
                condition: None,
                action: "execute_action_on_target_package".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Satisfied as i32,
                event: "policy_verification_failure".to_string(),
                to_state: ScenarioState::Denied as i32,
                condition: None,
                action: "log_denial_generate_alert".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Allowed as i32,
                event: "scenario_completion".to_string(),
                to_state: ScenarioState::Completed as i32,
                condition: None,
                action: "finalize_scenario".to_string(),
            },
        ];
        self.transition_tables
            .insert(ResourceType::Scenario, scenario_transitions);
    }

    // ========================================
    // CORE STATE PROCESSING
    // ========================================
    /// Process a state change request with non-blocking action execution
    pub fn process_state_change(&mut self, state_change: StateChange) -> TransitionResult {
        // Validate input parameters
        if let Err(validation_error) = self.validate_state_change(&state_change) {
            return TransitionResult {
                new_state: Self::state_str_to_enum(
                    state_change.current_state.as_str(),
                    state_change.resource_type,
                ),
                error_code: ErrorCode::InvalidRequest,
                message: format!("Invalid state change request: {validation_error}"),
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
                    new_state: Self::state_str_to_enum(
                        state_change.current_state.as_str(),
                        state_change.resource_type,
                    ),
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
            Some(existing_state) => existing_state.current_state,
            None => Self::state_str_to_enum(
                state_change.current_state.as_str(),
                state_change.resource_type,
            ),
        };

        // Special state-specific handling removed - using simplified state model

        // Find valid transition
        let transition_event = self.infer_event_from_states(
            current_state,
            Self::state_str_to_enum(
                state_change.target_state.as_str(),
                state_change.resource_type,
            ),
            resource_type,
        );

        if let Some(transition) = self.find_valid_transition(
            resource_type,
            current_state,
            &transition_event,
            Self::state_str_to_enum(
                state_change.target_state.as_str(),
                state_change.resource_type,
            ),
        ) {
            // Check conditions if any
            if let Some(ref condition) = transition.condition {
                if !self.evaluate_condition(condition, &state_change) {
                    return TransitionResult {
                        new_state: current_state,
                        error_code: ErrorCode::PreconditionFailed,
                        message: format!("Condition not met: {condition}"),
                        actions_to_execute: vec![],
                        transition_id: state_change.transition_id.clone(),
                        error_details: format!("Failed condition evaluation: {condition}"),
                    };
                }
            }

            // Execute transition - this is immediate and non-blocking
            self.update_resource_state(
                &resource_key,
                &state_change,
                transition.to_state,
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
                    eprintln!("Warning: Failed to queue action for execution: {e}");
                }
            }

            let transitioned_state_str = match resource_type {
                ResourceType::Scenario => ScenarioState::try_from(transition.to_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Package => PackageState::try_from(transition.to_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Model => ModelState::try_from(transition.to_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                _ => "UNKNOWN",
            };

            // Create successful transition result
            let transition_result = TransitionResult {
                new_state: transition.to_state,
                error_code: ErrorCode::Success,
                message: format!("Successfully transitioned to {transitioned_state_str}"),
                actions_to_execute: vec![transition.action.clone()],
                transition_id: state_change.transition_id.clone(),
                error_details: String::new(),
            };

            self.update_health_status(&resource_key, &transition_result);

            // State-specific logic removed for simplified state model

            transition_result
        } else {
            let current_state_str = match resource_type {
                ResourceType::Scenario => ScenarioState::try_from(current_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Package => PackageState::try_from(current_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                ResourceType::Model => ModelState::try_from(current_state)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN"),
                _ => "UNKNOWN",
            };

            let target_state_str = match resource_type {
                ResourceType::Scenario => {
                    let normalized = format!(
                        "SCENARIO_STATE_{}",
                        state_change
                            .target_state
                            .trim()
                            .to_ascii_uppercase()
                            .replace('-', "_")
                    );
                    ScenarioState::from_str_name(&normalized)
                        .map(|s| s.as_str_name())
                        .unwrap_or("UNKNOWN")
                }
                ResourceType::Package => {
                    let normalized = format!(
                        "PACKAGE_STATE_{}",
                        state_change
                            .target_state
                            .trim()
                            .to_ascii_uppercase()
                            .replace('-', "_")
                    );
                    PackageState::from_str_name(&normalized)
                        .map(|s| s.as_str_name())
                        .unwrap_or("UNKNOWN")
                }
                ResourceType::Model => {
                    let normalized = format!(
                        "MODEL_STATE_{}",
                        state_change
                            .target_state
                            .trim()
                            .to_ascii_uppercase()
                            .replace('-', "_")
                    );
                    ModelState::from_str_name(&normalized)
                        .map(|s| s.as_str_name())
                        .unwrap_or("UNKNOWN")
                }
                _ => "UNKNOWN",
            };

            let transition_result = TransitionResult {
                new_state: current_state,
                error_code: ErrorCode::InvalidStateTransition,
                message: format!(
                    "No valid transition from {current_state_str} to {target_state_str} for resource type {resource_type:?}",
                ),
                actions_to_execute: vec![],
                transition_id: state_change.transition_id.clone(),
                error_details: format!(
                    "Invalid state transition attempted: {current_state_str} -> {target_state_str}"
                ),
            };

            self.update_health_status(&resource_key, &transition_result);
            transition_result
        }
    }

    /// Process model state update based on container states
    ///
    /// This method handles model state evaluation and transitions triggered by container state changes,
    /// implementing the business logic defined in the StateManager_Model.md documentation.
    ///
    /// # Parameters
    /// - `model_name`: The name of the model to update
    /// - `containers`: List of containers associated with this model
    ///
    /// # Returns
    /// - `TransitionResult`: Results of the state evaluation and transition attempt
    ///   - Contains whether state changed, the new state, and transition details
    pub fn process_model_state_update(
        &mut self,
        model_name: &str,
        containers: &[&common::monitoringserver::ContainerInfo],
    ) -> TransitionResult {
        let resource_key = self.generate_resource_key(ResourceType::Model, model_name);
        let timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64;

        // Evaluate the new model state based on container states
        let new_model_state = self.evaluate_model_state_from_containers(containers);

        // Create a pseudo state change for internal processing
        let state_change = StateChange {
            resource_type: ResourceType::Model as i32,
            resource_name: model_name.to_string(),
            current_state: self
                .resource_states
                .get(&resource_key)
                .map(|rs| self.state_enum_to_str(rs.current_state, ResourceType::Model))
                .unwrap_or_else(|| "Created".to_string()),
            target_state: self.model_state_to_str(new_model_state),
            transition_id: format!("model_update_{}_{}", model_name, timestamp_ns),
            timestamp_ns,
            source: "container_analysis".to_string(),
        };

        // Get current state from existing resource or default to Created
        let current_state = self
            .resource_states
            .get(&resource_key)
            .map(|rs| rs.current_state)
            .unwrap_or(ModelState::Created as i32);

        let target_state = new_model_state as i32;

        // Check if state change is needed
        if current_state == target_state {
            return TransitionResult {
                new_state: target_state,
                error_code: ErrorCode::Success,
                message: "Model already in target state".to_string(),
                actions_to_execute: vec![],
                transition_id: state_change.transition_id.clone(),
                error_details: String::new(),
            };
        }

        // Update internal state tracking
        self.update_resource_state(
            &resource_key,
            &state_change,
            target_state,
            ResourceType::Model,
        );

        // Return successful transition result indicating state changed
        TransitionResult {
            new_state: target_state,
            error_code: ErrorCode::Success,
            message: format!(
                "Model state successfully transitioned from {} to {}",
                self.state_enum_to_str(current_state, ResourceType::Model),
                self.model_state_to_str(new_model_state)
            ),
            actions_to_execute: vec!["update_etcd".to_string()],
            transition_id: state_change.transition_id,
            error_details: String::new(),
        }
    }

    /// Evaluates the model state based on container states according to the state transition rules
    fn evaluate_model_state_from_containers(
        &self,
        containers: &[&common::monitoringserver::ContainerInfo],
    ) -> ModelState {
        if containers.is_empty() {
            return ModelState::Created;
        }

        let mut _running_count = 0;
        let mut paused_count = 0;
        let mut exited_count = 0;
        let mut dead_count = 0;
        let mut _stopped_count = 0;

        for container in containers {
            match self.parse_container_state(container) {
                ContainerState::Running => _running_count += 1,
                ContainerState::Paused => paused_count += 1,
                ContainerState::Exited => exited_count += 1,
                ContainerState::Dead => dead_count += 1,
                ContainerState::Stopped => _stopped_count += 1,
                ContainerState::Created => {} // Created containers don't affect model state
            }
        }

        let total_containers = containers.len();

        // Apply state transition rules from documentation
        // Rule 1: Dead - if one or more containers are dead
        if dead_count > 0 {
            return ModelState::Dead;
        }

        // Rule 2: Paused - if all containers are paused
        if paused_count == total_containers {
            return ModelState::Paused;
        }

        // Rule 3: Exited - if all containers are exited
        if exited_count == total_containers {
            return ModelState::Exited;
        }

        // Rule 4: Running - default state (none of above conditions met)
        ModelState::Running
    }

    /// Evaluates package state based on model states according to Korean documentation requirements
    ///
    /// This function implements the package state transition rules defined in StateManager_Package.md:
    /// - idle: Initial package state (creation default)
    /// - paused: All models are in paused state
    /// - exited: All models are in exited state
    /// - degraded: Some (1+) models are in dead state, but not all models are dead
    /// - error: All models are in dead state
    /// - running: Default state when none of the above conditions are met
    ///
    /// # Parameters
    /// - `model_states`: List of (model_name, model_state) tuples
    ///
    /// # Returns
    /// - `PackageState`: The determined package state based on model states
    pub fn evaluate_package_state_from_models(
        &self,
        model_states: &[(String, ModelState)],
    ) -> PackageState {
        if model_states.is_empty() {
            return PackageState::Idle;
        }

        let total_models = model_states.len();
        let mut paused_count = 0;
        let mut exited_count = 0;
        let mut dead_count = 0;

        // Count models in each relevant state
        for (_, model_state) in model_states {
            match model_state {
                ModelState::Paused => paused_count += 1,
                ModelState::Exited => exited_count += 1,
                ModelState::Dead => dead_count += 1,
                _ => {} // Other states don't directly impact package state rules
            }
        }

        // Apply package state transition rules from documentation
        // Rule 1: error - All models are in dead state
        if dead_count == total_models {
            return PackageState::Error;
        }

        // Rule 2: degraded - Some (1+) models are in dead state, but not all
        if dead_count > 0 && dead_count < total_models {
            return PackageState::Degraded;
        }

        // Rule 3: paused - All models are in paused state
        if paused_count == total_models {
            return PackageState::Paused;
        }

        // Rule 4: exited - All models are in exited state
        if exited_count == total_models {
            return PackageState::Exited;
        }

        // Rule 5: running - Default state when none of above conditions are met
        PackageState::Running
    }

    /// Retrieves all model states for models that belong to a given package
    ///
    /// This function queries ETCD to get all model states and filters them
    /// to find models that belong to the specified package.
    pub async fn get_models_for_package(
        package_name: &str,
    ) -> std::result::Result<Vec<(String, common::statemanager::ModelState)>, String> {
        // Get package definition from ETCD to find its models
        let package_key = format!("Package/{}", package_name);
        let package_yaml = match common::etcd::get(&package_key).await {
            Ok(yaml) => yaml,
            Err(e) => {
                println!("    Failed to get package definition: {:?}", e);
                return Ok(Vec::new());
            }
        };

        // Parse package YAML to extract model names
        let package: common::spec::artifact::Package = match serde_yaml::from_str(&package_yaml) {
            Ok(pkg) => pkg,
            Err(e) => {
                println!("    Failed to parse package YAML: {:?}", e);
                return Ok(Vec::new());
            }
        };

        let mut model_states = Vec::new();

        // Get state for each model in the package
        for model_info in package.get_models() {
            let model_name = model_info.get_name();
            let model_state_key = format!("/model/{}/state", model_name);

            match common::etcd::get(&model_state_key).await {
                Ok(state_str) => {
                    let model_state = match state_str.as_str() {
                        "Created" => common::statemanager::ModelState::Created,
                        "Paused" => common::statemanager::ModelState::Paused,
                        "Exited" => common::statemanager::ModelState::Exited,
                        "Dead" => common::statemanager::ModelState::Dead,
                        "Running" => common::statemanager::ModelState::Running,
                        _ => common::statemanager::ModelState::Running, // Default to Running
                    };
                    model_states.push((model_name, model_state));
                }
                Err(_) => {
                    // If model state not found, assume it's in Created state
                    model_states.push((model_name, common::statemanager::ModelState::Created));
                }
            }
        }

        Ok(model_states)
    }

    /// Find all packages that contain the given model
    pub async fn find_packages_containing_model(
        model_name: &str,
    ) -> std::result::Result<Vec<String>, String> {
        let mut packages = Vec::new();

        // Get all packages from ETCD with prefix
        match common::etcd::get_all_with_prefix("Package/").await {
            Ok(package_entries) => {
                for kv in package_entries {
                    match serde_yaml::from_str::<common::spec::artifact::Package>(&kv.value) {
                        Ok(package) => {
                            // Check if this package contains the model
                            for model_info in package.get_models() {
                                if model_info.get_name() == model_name {
                                    packages.push(package.get_name());
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            println!("    Failed to parse package {}: {:?}", kv.key, e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("    Failed to get packages from ETCD: {:?}", e);
                return Err(format!("Failed to get packages from ETCD: {:?}", e));
            }
        }

        Ok(packages)
    }

    /// Get current package state from ETCD
    pub async fn get_current_package_state(
        package_name: &str,
    ) -> Option<common::statemanager::PackageState> {
        let key = format!("/package/{}/state", package_name);
        match common::etcd::get(&key).await {
            Ok(state_str) => match state_str.as_str() {
                "PACKAGE_STATE_IDLE" | "idle" => Some(common::statemanager::PackageState::Idle),
                "PACKAGE_STATE_PAUSED" | "paused" => {
                    Some(common::statemanager::PackageState::Paused)
                }
                "PACKAGE_STATE_EXITED" | "exited" => {
                    Some(common::statemanager::PackageState::Exited)
                }
                "PACKAGE_STATE_DEGRADED" | "degraded" => {
                    Some(common::statemanager::PackageState::Degraded)
                }
                "PACKAGE_STATE_ERROR" | "error" => Some(common::statemanager::PackageState::Error),
                "PACKAGE_STATE_RUNNING" | "running" => {
                    Some(common::statemanager::PackageState::Running)
                }
                _ => Some(common::statemanager::PackageState::Idle),
            },
            Err(_) => None,
        }
    }

    /// Evaluate and update package state based on current model states
    pub async fn evaluate_and_update_package_state(
        &self,
        package_name: &str,
    ) -> std::result::Result<(bool, common::statemanager::PackageState), String> {
        println!("    Evaluating package state for: {}", package_name);

        // Get model states for this package
        let model_states = Self::get_models_for_package(package_name).await?;

        if model_states.is_empty() {
            println!("      No models found for package {}", package_name);
            return Ok((false, common::statemanager::PackageState::Idle));
        }

        // Convert to format expected by state machine
        let model_states_for_evaluation: Vec<(String, ModelState)> = model_states
            .iter()
            .map(|(name, state)| {
                let converted_state = match state {
                    common::statemanager::ModelState::Created => ModelState::Created,
                    common::statemanager::ModelState::Paused => ModelState::Paused,
                    common::statemanager::ModelState::Exited => ModelState::Exited,
                    common::statemanager::ModelState::Dead => ModelState::Dead,
                    common::statemanager::ModelState::Running => ModelState::Running,
                    _ => ModelState::Running,
                };
                (name.clone(), converted_state)
            })
            .collect();

        // Get current package state
        let current_package_state = Self::get_current_package_state(package_name)
            .await
            .unwrap_or(common::statemanager::PackageState::Idle);

        // Evaluate new package state using state machine
        let evaluated_state = self.evaluate_package_state_from_models(&model_states_for_evaluation);

        // Convert back to common::statemanager::PackageState
        let new_package_state = match evaluated_state {
            PackageState::Idle => common::statemanager::PackageState::Idle,
            PackageState::Paused => common::statemanager::PackageState::Paused,
            PackageState::Exited => common::statemanager::PackageState::Exited,
            PackageState::Degraded => common::statemanager::PackageState::Degraded,
            PackageState::Error => common::statemanager::PackageState::Error,
            PackageState::Running => common::statemanager::PackageState::Running,
            _ => common::statemanager::PackageState::Running,
        };

        // Check if package state changed
        let state_changed = new_package_state != current_package_state;
        if state_changed {
            println!(
                "      Package state changed: {} -> {}",
                current_package_state.as_str_name(),
                new_package_state.as_str_name()
            );
        } else {
            println!(
                "      Package {} state unchanged: {}",
                package_name,
                current_package_state.as_str_name()
            );
        }

        Ok((state_changed, new_package_state))
    }

    /// Parses container state from the state HashMap
    fn parse_container_state(
        &self,
        container: &common::monitoringserver::ContainerInfo,
    ) -> ContainerState {
        // Check the "Status" field first
        if let Some(status) = container.state.get("Status") {
            match status.to_lowercase().as_str() {
                "running" => return ContainerState::Running,
                "paused" => return ContainerState::Paused,
                "exited" => return ContainerState::Exited,
                "dead" => return ContainerState::Dead,
                "stopped" => return ContainerState::Stopped,
                "created" => return ContainerState::Created,
                _ => {}
            }
        }

        // Check "Running" boolean field as fallback
        if let Some(running) = container.state.get("Running") {
            if running == "true" {
                return ContainerState::Running;
            }
        }

        // Default to Created if state cannot be determined
        ContainerState::Created
    }

    /// Convert ModelState enum to string representation
    fn model_state_to_str(&self, state: ModelState) -> String {
        match state {
            ModelState::Created => "Created".to_string(),
            ModelState::Paused => "Paused".to_string(),
            ModelState::Exited => "Exited".to_string(),
            ModelState::Dead => "Dead".to_string(),
            ModelState::Running => "Running".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Convert PackageState enum to string representation
    #[allow(dead_code)]
    fn package_state_to_str(&self, state: PackageState) -> String {
        match state {
            PackageState::Idle => "idle".to_string(),
            PackageState::Paused => "paused".to_string(),
            PackageState::Exited => "exited".to_string(),
            PackageState::Degraded => "degraded".to_string(),
            PackageState::Error => "error".to_string(),
            PackageState::Running => "running".to_string(),
            _ => "unknown".to_string(),
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
        from_state: i32,
        event: &str,
        to_state: i32,
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
        format!("{resource_type:?}::{resource_name}")
    }

    /// Build context for action execution
    fn build_action_context(
        &self,
        state_change: &StateChange,
        transition: &StateTransition,
    ) -> HashMap<String, String> {
        let mut context = HashMap::new();

        let resource_type = match ResourceType::try_from(state_change.resource_type) {
            Ok(rt) => rt,
            Err(_) => ResourceType::Scenario, // fallback, adjust as needed
        };

        let from_state_str = match resource_type {
            ResourceType::Scenario => ScenarioState::try_from(transition.from_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Package => PackageState::try_from(transition.from_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Model => ModelState::try_from(transition.from_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            _ => "UNKNOWN",
        };

        let to_state_str = match resource_type {
            ResourceType::Scenario => ScenarioState::try_from(transition.to_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Package => PackageState::try_from(transition.to_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Model => ModelState::try_from(transition.to_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            _ => "UNKNOWN",
        };

        context.insert("from_state".to_string(), from_state_str.to_string());
        context.insert("to_state".to_string(), to_state_str.to_string());
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
    fn infer_event_from_states(
        &self,
        current_state: i32,
        target_state: i32,
        resource_type: ResourceType,
    ) -> String {
        match resource_type {
            ResourceType::Scenario => match (current_state, target_state) {
                (x, y) if x == ScenarioState::Idle as i32 && y == ScenarioState::Waiting as i32 => {
                    "scenario_activation".to_string()
                }
                (x, y)
                    if x == ScenarioState::Waiting as i32
                        && y == ScenarioState::Satisfied as i32 =>
                {
                    "condition_met".to_string()
                }
                (x, y)
                    if x == ScenarioState::Satisfied as i32
                        && y == ScenarioState::Allowed as i32 =>
                {
                    "policy_verification_success".to_string()
                }
                (x, y)
                    if x == ScenarioState::Satisfied as i32
                        && y == ScenarioState::Denied as i32 =>
                {
                    "policy_verification_failure".to_string()
                }
                (x, y)
                    if x == ScenarioState::Allowed as i32
                        && y == ScenarioState::Completed as i32 =>
                {
                    "scenario_completion".to_string()
                }
                _ => format!("transition_{current_state}_{target_state}"),
            },
            ResourceType::Package => match (current_state, target_state) {
                (x, y)
                    if x == PackageState::Unspecified as i32 && y == PackageState::Idle as i32 =>
                {
                    "launch_request".to_string()
                }
                (x, y) if x == PackageState::Idle as i32 && y == PackageState::Running as i32 => {
                    "initialization_complete".to_string()
                }
                (x, y) if x == PackageState::Idle as i32 && y == PackageState::Degraded as i32 => {
                    "partial_initialization_failure".to_string()
                }
                (x, y) if x == PackageState::Idle as i32 && y == PackageState::Error as i32 => {
                    "critical_initialization_failure".to_string()
                }
                (x, y)
                    if x == PackageState::Running as i32 && y == PackageState::Degraded as i32 =>
                {
                    "model_issue_detected".to_string()
                }
                (x, y) if x == PackageState::Running as i32 && y == PackageState::Error as i32 => {
                    "critical_issue_detected".to_string()
                }
                (x, y) if x == PackageState::Running as i32 && y == PackageState::Paused as i32 => {
                    "pause_request".to_string()
                }
                (x, y) if x == PackageState::Running as i32 && y == PackageState::Exited as i32 => {
                    "all_models_exited".to_string()
                }
                (x, y)
                    if x == PackageState::Degraded as i32 && y == PackageState::Running as i32 =>
                {
                    "model_recovery".to_string()
                }
                (x, y) if x == PackageState::Degraded as i32 && y == PackageState::Error as i32 => {
                    "additional_model_issues".to_string()
                }
                (x, y)
                    if x == PackageState::Degraded as i32 && y == PackageState::Paused as i32 =>
                {
                    "pause_request".to_string()
                }
                (x, y) if x == PackageState::Error as i32 && y == PackageState::Running as i32 => {
                    "recovery_successful".to_string()
                }
                (x, y) if x == PackageState::Paused as i32 && y == PackageState::Running as i32 => {
                    "resume_request".to_string()
                }
                (x, y) if x == PackageState::Paused as i32 && y == PackageState::Exited as i32 => {
                    "all_models_exited".to_string()
                }
                (x, y) if x == PackageState::Exited as i32 && y == PackageState::Running as i32 => {
                    "restart_request".to_string()
                }
                _ => format!("transition_{current_state}_{target_state}"),
            },
            ResourceType::Model => match (current_state, target_state) {
                (x, y)
                    if x == ModelState::Unspecified as i32 && y == ModelState::Created as i32 =>
                {
                    "creation_request".to_string()
                }
                (x, y) if x == ModelState::Created as i32 && y == ModelState::Running as i32 => {
                    "node_allocation_complete".to_string()
                }
                (x, y) if x == ModelState::Created as i32 && y == ModelState::Dead as i32 => {
                    "node_allocation_failed".to_string()
                }
                (x, y) if x == ModelState::Running as i32 && y == ModelState::Paused as i32 => {
                    "all_containers_paused".to_string()
                }
                (x, y) if x == ModelState::Running as i32 && y == ModelState::Exited as i32 => {
                    "all_containers_exited".to_string()
                }
                (x, y) if x == ModelState::Running as i32 && y == ModelState::Dead as i32 => {
                    "container_dead_or_info_failure".to_string()
                }
                (x, y) if x == ModelState::Paused as i32 && y == ModelState::Running as i32 => {
                    "resume_request".to_string()
                }
                (x, y) if x == ModelState::Paused as i32 && y == ModelState::Exited as i32 => {
                    "all_containers_exited".to_string()
                }
                (x, y) if x == ModelState::Paused as i32 && y == ModelState::Dead as i32 => {
                    "container_dead_or_info_failure".to_string()
                }
                (x, y) if x == ModelState::Exited as i32 && y == ModelState::Running as i32 => {
                    "restart_request".to_string()
                }
                (x, y) if x == ModelState::Dead as i32 && y == ModelState::Created as i32 => {
                    "manual_automatic_recovery".to_string()
                }
                _ => format!("transition_{current_state}_{target_state}"),
            },
            _ => format!("transition_{current_state}_{target_state}"),
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

        new_state: i32,
        resource_type: ResourceType,
    ) {
        let now = Instant::now();

        let resource_state = self
            .resource_states
            .entry(resource_key.to_string())
            .or_insert_with(|| ResourceState {
                resource_type,
                resource_name: state_change.resource_name.clone(),
                current_state: Self::state_str_to_enum(
                    state_change.current_state.as_str(),
                    state_change.resource_type,
                ),
                desired_state: Some(Self::state_str_to_enum(
                    state_change.target_state.as_str(),
                    state_change.resource_type,
                )),
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

        resource_state.current_state = new_state;
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

        state: i32,
    ) -> Vec<&ResourceState> {
        self.resource_states
            .values()
            .filter(|resource| {
                resource.current_state == state
                    && (resource_type.is_none() || resource_type == Some(resource.resource_type))
            })
            .collect()
    }

    // Utility: Convert state string to proto enum value
    fn state_str_to_enum(state: &str, resource_type: i32) -> i32 {
        // Map "idle" -> "SCENARIO_STATE_IDLE", etc.
        let normalized = match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => format!(
                "SCENARIO_STATE_{}",
                state.trim().to_ascii_uppercase().replace('-', "_")
            ),
            Ok(ResourceType::Package) => format!(
                "PACKAGE_STATE_{}",
                state.trim().to_ascii_uppercase().replace('-', "_")
            ),
            Ok(ResourceType::Model) => format!(
                "MODEL_STATE_{}",
                state.trim().to_ascii_uppercase().replace('-', "_")
            ),
            _ => state.trim().to_ascii_uppercase().replace('-', "_"),
        };
        match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => ScenarioState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or(ScenarioState::Unspecified as i32),
            Ok(ResourceType::Package) => PackageState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or(PackageState::Unspecified as i32),
            Ok(ResourceType::Model) => ModelState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or(ModelState::Unspecified as i32),
            _ => 0,
        }
    }

    // Utility: Convert proto enum value to state string
    fn state_enum_to_str(&self, state: i32, resource_type: ResourceType) -> String {
        match resource_type {
            ResourceType::Scenario => ScenarioState::try_from(state)
                .map(|s| {
                    s.as_str_name()
                        .strip_prefix("SCENARIO_STATE_")
                        .unwrap_or(s.as_str_name())
                        .to_string()
                })
                .unwrap_or_else(|_| "Unknown".to_string()),
            ResourceType::Package => PackageState::try_from(state)
                .map(|s| {
                    s.as_str_name()
                        .strip_prefix("PACKAGE_STATE_")
                        .unwrap_or(s.as_str_name())
                        .to_string()
                })
                .unwrap_or_else(|_| "Unknown".to_string()),
            ResourceType::Model => ModelState::try_from(state)
                .map(|s| {
                    s.as_str_name()
                        .strip_prefix("MODEL_STATE_")
                        .unwrap_or(s.as_str_name())
                        .to_string()
                })
                .unwrap_or_else(|_| "Unknown".to_string()),
            _ => "Unknown".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use common::statemanager::{ModelState, PackageState};

    #[test]
    fn test_evaluate_package_state_from_models_empty() {
        let state_machine = StateMachine::new();
        let model_states = vec![];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Idle);
    }

    #[test]
    fn test_evaluate_package_state_all_dead() {
        let state_machine = StateMachine::new();
        let model_states = vec![
            ("model1".to_string(), ModelState::Dead),
            ("model2".to_string(), ModelState::Dead),
        ];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Error);
    }

    #[test]
    fn test_evaluate_package_state_some_dead() {
        let state_machine = StateMachine::new();
        let model_states = vec![
            ("model1".to_string(), ModelState::Dead),
            ("model2".to_string(), ModelState::Running),
        ];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Degraded);
    }

    #[test]
    fn test_evaluate_package_state_all_paused() {
        let state_machine = StateMachine::new();
        let model_states = vec![
            ("model1".to_string(), ModelState::Paused),
            ("model2".to_string(), ModelState::Paused),
        ];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Paused);
    }

    #[test]
    fn test_evaluate_package_state_all_exited() {
        let state_machine = StateMachine::new();
        let model_states = vec![
            ("model1".to_string(), ModelState::Exited),
            ("model2".to_string(), ModelState::Exited),
        ];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Exited);
    }

    #[test]
    fn test_evaluate_package_state_mixed_running() {
        let state_machine = StateMachine::new();
        let model_states = vec![
            ("model1".to_string(), ModelState::Running),
            ("model2".to_string(), ModelState::Created),
        ];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Running);
    }

    #[test]
    fn test_evaluate_package_state_priority_dead_over_paused() {
        let state_machine = StateMachine::new();
        let model_states = vec![
            ("model1".to_string(), ModelState::Dead),
            ("model2".to_string(), ModelState::Paused),
            ("model3".to_string(), ModelState::Paused),
        ];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Degraded);
    }

    #[test]
    fn test_evaluate_package_state_priority_dead_over_exited() {
        let state_machine = StateMachine::new();
        let model_states = vec![
            ("model1".to_string(), ModelState::Dead),
            ("model2".to_string(), ModelState::Exited),
            ("model3".to_string(), ModelState::Exited),
        ];
        let result = state_machine.evaluate_package_state_from_models(&model_states);
        assert_eq!(result, PackageState::Degraded);
    }
}

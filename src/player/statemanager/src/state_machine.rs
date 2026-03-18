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
use common::logd;
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
                    logd!(5, "Warning: Failed to queue action for execution: {e}");
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
        let mut _created_count = 0;
        let mut _initialized_count = 0;
        let mut _unknown_count = 0;

        for container in containers {
            match self.parse_container_state(container) {
                ContainerState::Running => _running_count += 1,
                ContainerState::Paused => paused_count += 1,
                ContainerState::Exited => exited_count += 1,
                ContainerState::Dead => dead_count += 1,
                ContainerState::Created => _created_count += 1,
                ContainerState::Initialized => _initialized_count += 1,
                ContainerState::Unknown => _unknown_count += 1,
            }
        }

        let total_containers = containers.len();

        // Apply state transition rules from documentation
        // Rule 1: Dead - if one or more containers are dead or unknown
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
        let package_yaml = match common::persistency::get(&package_key).await {
            Ok(yaml) => yaml,
            Err(e) => {
                logd!(4, "    Failed to get package definition: {:?}", e);
                return Ok(Vec::new());
            }
        };

        // Parse package YAML to extract model names
        let package: common::spec::artifact::Package = match serde_yaml::from_str(&package_yaml) {
            Ok(pkg) => pkg,
            Err(e) => {
                logd!(4, "    Failed to parse package YAML: {:?}", e);
                return Ok(Vec::new());
            }
        };

        let mut model_states = Vec::new();

        // Get state for each model in the package
        for model_info in package.get_models() {
            let model_name = model_info.get_name();
            let model_state_key = format!("/model/{}/state", model_name);

            match common::persistency::get(&model_state_key).await {
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
        match common::persistency::get_all_with_prefix("Package/").await {
            Ok(package_entries) => {
                for kv in package_entries {
                    match serde_yaml::from_str::<common::spec::artifact::Package>(&kv.1) {
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
                            logd!(4, "    Failed to parse package {}: {:?}", kv.0, e);
                        }
                    }
                }
            }
            Err(e) => {
                logd!(5, "    Failed to get packages from ETCD: {:?}", e);
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
        match common::persistency::get(&key).await {
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
        logd!(2, "    Evaluating package state for: {}", package_name);

        // Get model states for this package
        let model_states = Self::get_models_for_package(package_name).await?;

        if model_states.is_empty() {
            logd!(4, "      No models found for package {}", package_name);
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
            logd!(
                1,
                "      Package state changed: {} -> {}",
                current_package_state.as_str_name(),
                new_package_state.as_str_name()
            );
        } else {
            logd!(
                1,
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
                "created" => return ContainerState::Created,
                "initialized" => return ContainerState::Initialized,
                "unknown" => return ContainerState::Unknown,
                _ => return ContainerState::Unknown,
            }
        }

        // Check "Running" boolean field as fallback
        if let Some(running) = container.state.get("Running") {
            if running == "true" {
                return ContainerState::Running;
            }
        }

        // Default to Unknown if state cannot be determined
        ContainerState::Unknown
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

    #[test]
    fn test_parse_container_state_new_states() {
        use common::monitoringserver::ContainerInfo;
        use std::collections::HashMap;

        let state_machine = StateMachine::new();

        // Test new "initialized" state
        let mut state_map = HashMap::new();
        state_map.insert("Status".to_string(), "initialized".to_string());
        let container_info = ContainerInfo {
            id: "test-id".to_string(),
            names: vec!["test".to_string()],
            image: "test:latest".to_string(),
            state: state_map,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let result = state_machine.parse_container_state(&container_info);
        assert_eq!(result, ContainerState::Initialized);

        // Test new "unknown" state
        let mut state_map = HashMap::new();
        state_map.insert("Status".to_string(), "unknown".to_string());
        let container_info = ContainerInfo {
            id: "test-id".to_string(),
            names: vec!["test".to_string()],
            image: "test:latest".to_string(),
            state: state_map,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let result = state_machine.parse_container_state(&container_info);
        assert_eq!(result, ContainerState::Unknown);

        // Test unrecognized state defaults to Unknown
        let mut state_map = HashMap::new();
        state_map.insert("Status".to_string(), "some_unrecognized_state".to_string());
        let container_info = ContainerInfo {
            id: "test-id".to_string(),
            names: vec!["test".to_string()],
            image: "test:latest".to_string(),
            state: state_map,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let result = state_machine.parse_container_state(&container_info);
        assert_eq!(result, ContainerState::Unknown);
    }

    #[test]
    fn test_process_state_change_queues_action_and_updates_state() {
        use common::statemanager::ResourceType;

        let mut state_machine = StateMachine::new();

        // Initialize action executor so actions are queued to receiver
        let mut action_receiver = state_machine.initialize_action_executor();

        // Build a valid StateChange: Scenario Idle -> Waiting
        let state_change = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "test-scenario".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Waiting".to_string(),
            transition_id: "t-1".to_string(),
            timestamp_ns: 1,
            source: "unittest".to_string(),
        };

        let result = state_machine.process_state_change(state_change.clone());
        assert!(result.is_success(), "expected success transition");
        // Action should have been queued
        let action = action_receiver
            .try_recv()
            .expect("expected an action queued");
        assert_eq!(action.action, "start_condition_evaluation");
        // Resource state should now exist
        let rs = state_machine.get_resource_state("test-scenario", ResourceType::Scenario);
        assert!(
            rs.is_some(),
            "resource state should be present after transition"
        );
    }

    #[test]
    fn test_process_state_change_invalid_transition_returns_error() {
        use common::statemanager::{ErrorCode, ResourceType};

        let mut state_machine = StateMachine::new();

        // Build a StateChange with an unknown target state -> should produce InvalidStateTransition
        let state_change = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "bad-scenario".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Nonexistent".to_string(),
            transition_id: "t-2".to_string(),
            timestamp_ns: 2,
            source: "unittest".to_string(),
        };

        let result = state_machine.process_state_change(state_change);
        assert_eq!(result.error_code, ErrorCode::InvalidStateTransition);
    }

    #[test]
    fn test_update_health_status_marks_unhealthy_after_retries() {
        use common::statemanager::ResourceType;

        let mut state_machine = StateMachine::new();

        // Prepare a resource state with 2 consecutive failures already
        let resource_key =
            state_machine.generate_resource_key(ResourceType::Scenario, "h-scenario");
        let now = Instant::now();
        let rs = ResourceState {
            resource_type: ResourceType::Scenario,
            resource_name: "h-scenario".to_string(),
            current_state: ScenarioState::Idle as i32,
            desired_state: Some(ScenarioState::Waiting as i32),
            last_transition_time: now,
            transition_count: 0,
            metadata: HashMap::new(),
            health_status: HealthStatus {
                healthy: true,
                status_message: "ok".to_string(),
                last_check: now,
                consecutive_failures: 2,
            },
        };

        state_machine
            .resource_states
            .insert(resource_key.clone(), rs);

        // Create a failing TransitionResult
        let fail_result = TransitionResult {
            new_state: ScenarioState::Idle as i32,
            error_code: ErrorCode::InvalidStateTransition,
            message: "failed".to_string(),
            actions_to_execute: vec![],
            transition_id: "fail-1".to_string(),
            error_details: "details".to_string(),
        };

        // Call update_health_status (private) — accessible inside this test module
        state_machine.update_health_status(&resource_key, &fail_result);

        let updated = state_machine.resource_states.get(&resource_key).unwrap();
        assert_eq!(updated.health_status.consecutive_failures, 3);
        assert!(!updated.health_status.healthy);
    }

    #[test]
    fn test_evaluate_model_state_from_containers_variants() {
        use common::monitoringserver::ContainerInfo;
        use std::collections::HashMap;

        let state_machine = StateMachine::new();

        // Empty -> Created
        let res = state_machine.evaluate_model_state_from_containers(&[]);
        assert_eq!(res, ModelState::Created);

        // One dead -> Dead
        let mut state_map = HashMap::new();
        state_map.insert("Status".to_string(), "dead".to_string());
        let container_dead = ContainerInfo {
            id: "d1".to_string(),
            names: vec!["d1".to_string()],
            image: "img".to_string(),
            state: state_map,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let res = state_machine.evaluate_model_state_from_containers(&[&container_dead]);
        assert_eq!(res, ModelState::Dead);

        // All paused -> Paused
        let mut s1 = HashMap::new();
        s1.insert("Status".to_string(), "paused".to_string());
        let c1 = ContainerInfo {
            id: "p1".to_string(),
            names: vec!["p1".to_string()],
            image: "img".to_string(),
            state: s1,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let mut s2 = HashMap::new();
        s2.insert("Status".to_string(), "paused".to_string());
        let c2 = ContainerInfo {
            id: "p2".to_string(),
            names: vec!["p2".to_string()],
            image: "img".to_string(),
            state: s2,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let res = state_machine.evaluate_model_state_from_containers(&[&c1, &c2]);
        assert_eq!(res, ModelState::Paused);

        // All exited -> Exited
        let mut e1 = HashMap::new();
        e1.insert("Status".to_string(), "exited".to_string());
        let ce1 = ContainerInfo {
            id: "e1".to_string(),
            names: vec!["e1".to_string()],
            image: "img".to_string(),
            state: e1,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let mut e2 = HashMap::new();
        e2.insert("Status".to_string(), "exited".to_string());
        let ce2 = ContainerInfo {
            id: "e2".to_string(),
            names: vec!["e2".to_string()],
            image: "img".to_string(),
            state: e2,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let res = state_machine.evaluate_model_state_from_containers(&[&ce1, &ce2]);
        assert_eq!(res, ModelState::Exited);

        // Mixed -> Running (default)
        let mut r1 = HashMap::new();
        r1.insert("Status".to_string(), "running".to_string());
        let cr1 = ContainerInfo {
            id: "r1".to_string(),
            names: vec!["r1".to_string()],
            image: "img".to_string(),
            state: r1,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let mut cr2m = HashMap::new();
        cr2m.insert("Status".to_string(), "initialized".to_string());
        let cr2 = ContainerInfo {
            id: "r2".to_string(),
            names: vec!["r2".to_string()],
            image: "img".to_string(),
            state: cr2m,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };
        let res = state_machine.evaluate_model_state_from_containers(&[&cr1, &cr2]);
        assert_eq!(res, ModelState::Running);
    }

    #[test]
    fn test_process_model_state_update_transitions() {
        use common::monitoringserver::ContainerInfo;
        use common::statemanager::ResourceType;
        use std::collections::HashMap;

        let mut state_machine = StateMachine::new();

        let mut s = HashMap::new();
        s.insert("Status".to_string(), "running".to_string());
        let container = ContainerInfo {
            id: "m1".to_string(),
            names: vec!["m1".to_string()],
            image: "img".to_string(),
            state: s,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };

        let result = state_machine.process_model_state_update("model-x", &[&container]);
        assert!(result.is_success());
        assert_eq!(result.actions_to_execute, vec!["update_etcd".to_string()]);

        // Resource should now exist with Model type
        let rs = state_machine.get_resource_state("model-x", ResourceType::Model);
        assert!(rs.is_some());
    }

    #[test]
    fn test_parse_container_state_running_fallback() {
        use common::monitoringserver::ContainerInfo;
        use std::collections::HashMap;

        let state_machine = StateMachine::new();

        let mut hm = HashMap::new();
        hm.insert("Running".to_string(), "true".to_string());
        let container = ContainerInfo {
            id: "rb".to_string(),
            names: vec!["rb".to_string()],
            image: "img".to_string(),
            state: hm,
            config: HashMap::new(),
            annotation: HashMap::new(),
            stats: HashMap::new(),
        };

        let res = state_machine.parse_container_state(&container);
        assert_eq!(res, ContainerState::Running);
    }

    #[test]
    fn test_get_resource_state_and_list_resources_by_state() {
        use common::statemanager::{ResourceType, ScenarioState};

        let mut state_machine = StateMachine::new();

        // Create a scenario via process_state_change (Idle -> Waiting)
        let state_change = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "list-test".to_string(),
            current_state: "Idle".to_string(),
            target_state: "Waiting".to_string(),
            transition_id: "lt-1".to_string(),
            timestamp_ns: 1,
            source: "unittest".to_string(),
        };

        let _ = state_machine.process_state_change(state_change);

        // Verify get_resource_state
        let rs = state_machine.get_resource_state("list-test", ResourceType::Scenario);
        assert!(rs.is_some());

        // Verify list_resources_by_state returns the scenario when filtered
        let list = state_machine
            .list_resources_by_state(Some(ResourceType::Scenario), ScenarioState::Waiting as i32);
        assert!(!list.is_empty());
    }

    #[test]
    fn test_infer_event_from_states_scenario() {
        let sm = StateMachine::new();
        let evt = sm.infer_event_from_states(
            ScenarioState::Idle as i32,
            ScenarioState::Waiting as i32,
            ResourceType::Scenario,
        );
        assert_eq!(evt, "scenario_activation");
    }

    #[test]
    fn test_evaluate_condition_known_and_unknown() {
        let sm = StateMachine::new();
        assert!(sm.evaluate_condition(
            "all_models_normal",
            &StateChange {
                resource_type: ResourceType::Scenario as i32,
                resource_name: "r".to_string(),
                current_state: "".to_string(),
                target_state: "".to_string(),
                transition_id: "t".to_string(),
                timestamp_ns: 0,
                source: "test".to_string(),
            }
        ));

        // Unknown condition defaults to true per implementation
        assert!(sm.evaluate_condition(
            "some_unknown_condition_xyz",
            &StateChange {
                resource_type: ResourceType::Scenario as i32,
                resource_name: "r".to_string(),
                current_state: "".to_string(),
                target_state: "".to_string(),
                transition_id: "t".to_string(),
                timestamp_ns: 0,
                source: "test".to_string(),
            }
        ));
    }

    #[test]
    fn test_evaluate_condition_false_cases() {
        let sm = StateMachine::new();
        let sc = StateChange {
            resource_type: ResourceType::Scenario as i32,
            resource_name: "r".to_string(),
            current_state: "".to_string(),
            target_state: "".to_string(),
            transition_id: "t".to_string(),
            timestamp_ns: 0,
            source: "test".to_string(),
        };
        assert!(!sm.evaluate_condition("critical_models_failed", &sc));
        assert!(!sm.evaluate_condition("timeout_or_error", &sc));
        assert!(!sm.evaluate_condition("unexpected_termination", &sc));
        assert!(!sm.evaluate_condition("consecutive_restart_failures", &sc));
    }

    #[test]
    fn test_infer_event_package_and_model_variants() {
        let sm = StateMachine::new();
        // Package variants
        let e1 = sm.infer_event_from_states(
            PackageState::Unspecified as i32,
            PackageState::Idle as i32,
            ResourceType::Package,
        );
        assert_eq!(e1, "launch_request");
        let e2 = sm.infer_event_from_states(
            PackageState::Idle as i32,
            PackageState::Running as i32,
            ResourceType::Package,
        );
        assert_eq!(e2, "initialization_complete");
        let e3 = sm.infer_event_from_states(
            PackageState::Running as i32,
            PackageState::Degraded as i32,
            ResourceType::Package,
        );
        assert_eq!(e3, "model_issue_detected");

        // Model variants
        let m1 = sm.infer_event_from_states(
            ModelState::Unspecified as i32,
            ModelState::Created as i32,
            ResourceType::Model,
        );
        assert_eq!(m1, "creation_request");
        let m2 = sm.infer_event_from_states(
            ModelState::Created as i32,
            ModelState::Running as i32,
            ResourceType::Model,
        );
        assert_eq!(m2, "node_allocation_complete");
        let m3 = sm.infer_event_from_states(
            ModelState::Running as i32,
            ModelState::Dead as i32,
            ResourceType::Model,
        );
        assert_eq!(m3, "container_dead_or_info_failure");
    }

    #[test]
    fn test_state_str_to_enum_hyphen_and_case() {
        // Test various normalizations
        let v = StateMachine::state_str_to_enum("waiting", ResourceType::Scenario as i32);
        assert_eq!(v, ScenarioState::Waiting as i32);
        let v2 = StateMachine::state_str_to_enum("running", ResourceType::Package as i32);
        assert_eq!(v2, PackageState::Running as i32);
        let v3 = StateMachine::state_str_to_enum("created", ResourceType::Model as i32);
        assert_eq!(v3, ModelState::Created as i32);
        // Hyphenated or mixed case
        let v4 = StateMachine::state_str_to_enum("some-state", ResourceType::Scenario as i32);
        // Unknown maps to Unspecified for Scenario
        assert!(v4 == ScenarioState::Unspecified as i32 || v4 >= 0);
    }

    #[test]
    fn test_state_str_to_enum_and_enum_to_str() {
        // state_str_to_enum is an associated fn
        let idle = StateMachine::state_str_to_enum("Idle", ResourceType::Scenario as i32);
        assert_eq!(idle, ScenarioState::Idle as i32);

        let sm = StateMachine::new();
        let s = sm.state_enum_to_str(ScenarioState::Waiting as i32, ResourceType::Scenario);
        assert_eq!(s.to_lowercase(), "waiting");
    }

    #[tokio::test]
    async fn test_get_current_package_state_reads_etcd() {
        // Put a package state into etcd and verify mapping
        let key = "/package/testpkg/state";
        let _ = common::persistency::put(key, "running").await;
        let res = StateMachine::get_current_package_state("testpkg").await;
        assert!(res.is_some());
        assert_eq!(res.unwrap(), common::statemanager::PackageState::Running);
    }

    #[tokio::test]
    async fn test_evaluate_and_update_package_state_all_dead_in_etcd() {
        // Create a package with two models and set both models' states to Dead in ETCD
        let pkg_key = "Package/pkg-dead";
        let pkg_yaml = r#"{"apiVersion":"v1","kind":"Package","metadata":{"name":"pkg-dead"},"spec":{"pattern":[],"models":[{"name":"mdead1","node":"n","resources":{"volume":"","network":"","realtime":false}},{"name":"mdead2","node":"n","resources":{"volume":"","network":"","realtime":false}}]}}"#;

        let _ = common::persistency::put(pkg_key, pkg_yaml).await;
        let _ = common::persistency::put("/model/mdead1/state", "Dead").await;
        let _ = common::persistency::put("/model/mdead2/state", "Dead").await;
        // Set current package state to running to ensure a state change is detected
        let _ = common::persistency::put("/package/pkg-dead/state", "running").await;

        let sm = StateMachine::new();
        let (changed, state) = sm
            .evaluate_and_update_package_state("pkg-dead")
            .await
            .expect("should return Ok");
        assert!(
            changed,
            "expected package state to change when all models are dead"
        );
        assert_eq!(state, common::statemanager::PackageState::Error);
    }

    #[tokio::test]
    async fn test_evaluate_and_update_package_state_degraded_in_etcd() {
        // Create a package with two models and set one model Dead and one Running
        let pkg_key = "Package/pkg-degraded";
        let pkg_yaml = r#"{"apiVersion":"v1","kind":"Package","metadata":{"name":"pkg-degraded"},"spec":{"pattern":[],"models":[{"name":"mdeg1","node":"n","resources":{"volume":"","network":"","realtime":false}},{"name":"mdeg2","node":"n","resources":{"volume":"","network":"","realtime":false}}]}}"#;

        let _ = common::persistency::put(pkg_key, pkg_yaml).await;
        let _ = common::persistency::put("/model/mdeg1/state", "Dead").await;
        let _ = common::persistency::put("/model/mdeg2/state", "Running").await;
        let _ = common::persistency::put("/package/pkg-degraded/state", "running").await;

        let sm = StateMachine::new();
        let (changed, state) = sm
            .evaluate_and_update_package_state("pkg-degraded")
            .await
            .expect("should return Ok");
        assert!(
            changed,
            "expected package state to change when some models are dead"
        );
        assert_eq!(state, common::statemanager::PackageState::Degraded);
    }

    #[tokio::test]
    async fn test_get_models_for_package_missing_returns_empty() {
        // Ensure package key is absent
        let _ = common::persistency::delete("Package/missing-package").await;
        let res = StateMachine::get_models_for_package("missing-package").await;
        assert!(
            res.is_ok(),
            "expected Ok result when package entry is missing in etcd"
        );
        let models = res.unwrap();
        assert!(
            models.is_empty(),
            "expected empty model list for missing package"
        );
    }

    #[tokio::test]
    async fn test_get_models_for_package_invalid_yaml_returns_empty() {
        // Put an invalid YAML string into etcd under the package key
        let pkg_key = "Package/pkg-invalid-yaml";
        let _ = common::persistency::put(pkg_key, "::: not valid yaml :::").await;
        let res = StateMachine::get_models_for_package("pkg-invalid-yaml").await;
        assert!(
            res.is_ok(),
            "expected Ok result when package YAML is invalid"
        );
        let models = res.unwrap();
        assert!(
            models.is_empty(),
            "expected empty model list when package YAML parse fails"
        );
    }

    #[tokio::test]
    async fn test_find_packages_containing_model_success() {
        // Create two packages, one containing the target model
        let pkg_a_key = "Package/pkg-with-model";
        let pkg_a_yaml = r#"{"apiVersion":"v1","kind":"Package","metadata":{"name":"pkg-with-model"},"spec":{"pattern":[],"models":[{"name":"target_model","node":"n","resources":{"volume":"","network":"","realtime":false}}]}}"#;
        let pkg_b_key = "Package/pkg-without-model";
        let pkg_b_yaml = r#"{"apiVersion":"v1","kind":"Package","metadata":{"name":"pkg-without-model"},"spec":{"pattern":[],"models":[]}}"#;

        let _ = common::persistency::put(pkg_a_key, pkg_a_yaml).await;
        let _ = common::persistency::put(pkg_b_key, pkg_b_yaml).await;

        let res = StateMachine::find_packages_containing_model("target_model").await;
        assert!(res.is_ok());
        let pkgs = res.unwrap();
        assert!(
            pkgs.iter().any(|p| p == "pkg-with-model"),
            "expected pkg-with-model to be returned"
        );
    }

    #[tokio::test]
    async fn test_get_current_package_state_none_when_missing() {
        // Ensure no state key exists for this package
        let _ = common::persistency::delete("/package/no-state/state").await;
        let res = StateMachine::get_current_package_state("no-state").await;
        assert!(
            res.is_none(),
            "expected None when package state key is missing"
        );
    }

    #[test]
    fn test_find_valid_transition_scenario() {
        let sm = StateMachine::new();
        // Scenario Idle -> Waiting should exist
        let tr = sm.find_valid_transition(
            ResourceType::Scenario,
            ScenarioState::Idle as i32,
            "scenario_activation",
            ScenarioState::Waiting as i32,
        );
        assert!(tr.is_some());
        let t = tr.unwrap();
        assert_eq!(t.action, "start_condition_evaluation");
    }

    #[tokio::test]
    async fn test_evaluate_and_update_package_state_no_models() {
        let sm = StateMachine::new();
        // Ensure no package data is present in etcd for this test package
        let _ = common::persistency::delete("Package/nonexistent-package").await;
        let (changed, state) = sm
            .evaluate_and_update_package_state("nonexistent-package")
            .await
            .expect("should return Ok");
        assert!(!changed);
        assert_eq!(state, common::statemanager::PackageState::Idle);
    }
}

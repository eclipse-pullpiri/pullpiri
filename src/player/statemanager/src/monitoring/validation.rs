/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! State validation utilities and constraint checking

use crate::core::types::SerializableResourceState;
use common::statemanager::{ResourceType, StateChange, ScenarioState, PackageState, ModelState};
use tracing::{debug, warn};

pub struct StateValidator;

impl StateValidator {
    /// Validate state change request parameters
    pub fn validate_state_change(state_change: &StateChange) -> Result<(), String> {
        debug!("Validating state change request");

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

        debug!("State change validation passed");
        Ok(())
    }

    /// Validate a state loaded from etcd
    pub fn validate_loaded_state(state: &SerializableResourceState) -> bool {
        debug!("Validating loaded state for resource: {}", state.resource_name);

        if ResourceType::try_from(state.resource_type).is_err() {
            warn!("Invalid resource type: {}", state.resource_type);
            return false;
        }

        if state.resource_name.trim().is_empty() {
            warn!("Empty resource name in loaded state");
            return false;
        }

        if state.current_state.trim().is_empty() {
            warn!("Empty current state in loaded state");
            return false;
        }

        let is_valid_enum = match ResourceType::try_from(state.resource_type) {
            Ok(ResourceType::Scenario) => ScenarioState::from_str_name(&state.current_state).is_some(),
            Ok(ResourceType::Package) => PackageState::from_str_name(&state.current_state).is_some(),
            Ok(ResourceType::Model) => ModelState::from_str_name(&state.current_state).is_some(),
            _ => false,
        };

        if !is_valid_enum {
            warn!("Invalid current state enum: {}", state.current_state);
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        if state.last_transition_unix_timestamp > now + 3600 {
            warn!("Future timestamp detected: {}", state.last_transition_unix_timestamp);
            return false;
        }

        debug!("State validation passed");
        true
    }

    /// Evaluate whether a transition condition is satisfied
    pub fn evaluate_condition(condition: &str, _state_change: &StateChange) -> bool {
        debug!("Evaluating condition: {}", condition);

        let result = match condition {
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
            _ => {
                warn!("Unknown condition '{}', defaulting to true", condition);
                true
            }
        };

        debug!("Condition '{}' evaluated to: {}", condition, result);
        result
    }
}
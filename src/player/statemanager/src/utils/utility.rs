/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Utility functions for state management operations

use crate::core::types::{StateTransition, SerializableResourceState, ResourceState};
use common::statemanager::{
    ModelState, PackageState, ResourceType, ScenarioState, StateChange,
};
use std::collections::HashMap;
use tracing::trace;

pub struct StateUtilities;

impl StateUtilities {
    /// Generate a unique resource key
    pub fn generate_resource_key(resource_type: ResourceType, resource_name: &str) -> String {
        let key = format!("{:?}::{}", resource_type, resource_name);
        trace!("Generated resource key: {}", key);
        key
    }

    /// Build context for action execution
    pub fn build_action_context(
        state_change: &StateChange,
        transition: &StateTransition,
    ) -> HashMap<String, String> {
        trace!("Building action context");

        let mut context = HashMap::new();

        let resource_type = match ResourceType::try_from(state_change.resource_type) {
            Ok(rt) => rt,
            Err(_) => ResourceType::Scenario,
        };

        let from_state_str = Self::state_enum_to_str(transition.from_state, resource_type);
        let to_state_str = Self::state_enum_to_str(transition.to_state, resource_type);

        context.insert("from_state".to_string(), from_state_str.to_string());
        context.insert("to_state".to_string(), to_state_str.to_string());
        context.insert("event".to_string(), transition.event.clone());
        context.insert("resource_name".to_string(), state_change.resource_name.clone());
        context.insert("source".to_string(), state_change.source.clone());
        context.insert("timestamp_ns".to_string(), state_change.timestamp_ns.to_string());

        trace!("Action context built with {} entries", context.len());
        context
    }

    /// Convert RAW user input states (like "waiting", "idle") to enum integers
    /// Use ONLY for user-provided state strings from API calls
    pub fn state_str_to_enum(state: &str, resource_type: i32) -> i32 {
        let normalized = if state.contains("_STATE_") {
            // User provided full enum name - use as-is
            state.to_string()
        } else {
            match ResourceType::try_from(resource_type) {
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
            }
        };

        let result = match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => ScenarioState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Invalid scenario state '{}' (normalized: '{}')", state, normalized);
                    ScenarioState::Unspecified as i32
                }),
            Ok(ResourceType::Package) => PackageState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Invalid package state '{}' (normalized: '{}')", state, normalized);
                    PackageState::Unspecified as i32
                }),
            Ok(ResourceType::Model) => ModelState::from_str_name(&normalized)
                .map(|s| s as i32)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Invalid model state '{}' (normalized: '{}')", state, normalized);
                    ModelState::Unspecified as i32
                }),
            _ => {
                eprintln!("ERROR: Invalid resource type {}", resource_type);
                0
            }
        };

        result
    }

    /// Convert ETCD enum strings (like "SCENARIO_STATE_WAITING") to integers
    /// Use ONLY for data retrieved from etcd storage
    pub fn enum_str_to_int(state: &str, resource_type: i32) -> i32 {
        let result = match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => ScenarioState::from_str_name(state)
                .map(|s| s as i32)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Invalid etcd scenario state enum '{}'", state);
                    ScenarioState::Unspecified as i32
                }),
            Ok(ResourceType::Package) => PackageState::from_str_name(state)
                .map(|s| s as i32)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Invalid etcd package state enum '{}'", state);
                    PackageState::Unspecified as i32
                }),
            Ok(ResourceType::Model) => ModelState::from_str_name(state)
                .map(|s| s as i32)
                .unwrap_or_else(|| {
                    eprintln!("ERROR: Invalid etcd model state enum '{}'", state);
                    ModelState::Unspecified as i32
                }),
            _ => {
                eprintln!("ERROR: Invalid resource type {}", resource_type);
                0
            }
        };

        result
    }

    pub fn state_enum_to_str(state: i32, resource_type: ResourceType) -> &'static str {
        let result = match resource_type {
            ResourceType::Scenario => ScenarioState::try_from(state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Package => PackageState::try_from(state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            ResourceType::Model => ModelState::try_from(state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN"),
            _ => "UNKNOWN",
        };

        trace!("Enum to string conversion: {} -> {}", state, result);
        result
    }

    /// Check if a state is considered active for cache warming
    pub fn is_active_state(state: i32, resource_type: i32) -> bool {
        match ResourceType::try_from(resource_type) {
            Ok(ResourceType::Scenario) => {
                matches!(state, x if x == ScenarioState::Playing as i32 || x == ScenarioState::Waiting as i32)
            }
            Ok(ResourceType::Package) => {
                matches!(state, x if x == PackageState::Running as i32 || x == PackageState::Initializing as i32)
            }
            Ok(ResourceType::Model) => {
                matches!(state, x if x == ModelState::Running as i32 || x == ModelState::Pending as i32)
            }
            _ => false,
        }
    }
}
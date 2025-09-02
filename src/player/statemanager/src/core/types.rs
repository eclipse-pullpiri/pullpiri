use common::statemanager::{ErrorCode, ModelState, PackageState, ResourceType, ScenarioState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Instant;

// SerializableInstant conversion to preserve actual time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableInstant {
    pub unix_timestamp: u64,
}

impl SerializableInstant {
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        SerializableInstant {
            unix_timestamp: now,
        }
    }
}

// existing non-serializable types for runtime use
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub status_message: String,
    pub last_check: Instant,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone)]
pub struct ResourceState {
    pub resource_type: ResourceType,
    pub resource_name: String,
    pub current_state: i32,
    pub desired_state: Option<i32>,
    pub last_transition_time: Instant,
    pub transition_count: u32,
    pub metadata: HashMap<String, String>,
    pub health_status: HealthStatus,
}

// serializable versions for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableHealthStatus {
    pub healthy: bool,
    pub status_message: String,
    pub last_check_unix_timestamp: u64,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableResourceState {
    pub resource_type: i32,
    pub resource_name: String,
    pub current_state: String,
    pub desired_state: Option<String>,
    pub last_transition_unix_timestamp: u64,
    pub transition_count: u32,
    pub metadata: HashMap<String, String>,
    pub health_status: SerializableHealthStatus,
}

// Fix the conversion implementations
impl From<HealthStatus> for SerializableHealthStatus {
    fn from(status: HealthStatus) -> Self {
        SerializableHealthStatus {
            healthy: status.healthy,
            status_message: status.status_message,
            last_check_unix_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            consecutive_failures: status.consecutive_failures,
        }
    }
}

impl From<SerializableHealthStatus> for HealthStatus {
    fn from(status: SerializableHealthStatus) -> Self {
        HealthStatus {
            healthy: status.healthy,
            status_message: status.status_message,
            last_check: Instant::now(),
            consecutive_failures: status.consecutive_failures,
        }
    }
}

impl From<ResourceState> for SerializableResourceState {
    fn from(state: ResourceState) -> Self {
        let current_state_str = match state.resource_type {
            ResourceType::Scenario => ScenarioState::try_from(state.current_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN")
                .to_string(),
            ResourceType::Package => PackageState::try_from(state.current_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN")
                .to_string(),
            ResourceType::Model => ModelState::try_from(state.current_state)
                .map(|s| s.as_str_name())
                .unwrap_or("UNKNOWN")
                .to_string(),
            _ => "UNKNOWN".to_string(),
        };

        let desired_state_str = match (state.desired_state, state.resource_type) {
            (Some(desired), ResourceType::Scenario) => Some(
                ScenarioState::try_from(desired)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
            ),
            (Some(desired), ResourceType::Package) => Some(
                PackageState::try_from(desired)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
            ),
            (Some(desired), ResourceType::Model) => Some(
                ModelState::try_from(desired)
                    .map(|s| s.as_str_name())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
            ),
            _ => None,
        };

        SerializableResourceState {
            resource_type: state.resource_type as i32,
            resource_name: state.resource_name,
            current_state: current_state_str,
            desired_state: desired_state_str,
            last_transition_unix_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            transition_count: state.transition_count,
            metadata: state.metadata,
            health_status: SerializableHealthStatus::from(state.health_status),
        }
    }
}

impl From<SerializableResourceState> for ResourceState {
    fn from(state: SerializableResourceState) -> Self {
        let current_state_int = match ResourceType::try_from(state.resource_type) {
            Ok(ResourceType::Scenario) => ScenarioState::from_str_name(&state.current_state)
                .map(|s| s as i32)
                .unwrap_or(ScenarioState::Unspecified as i32),
            Ok(ResourceType::Package) => PackageState::from_str_name(&state.current_state)
                .map(|s| s as i32)
                .unwrap_or(PackageState::Unspecified as i32),
            Ok(ResourceType::Model) => ModelState::from_str_name(&state.current_state)
                .map(|s| s as i32)
                .unwrap_or(ModelState::Unspecified as i32),
            _ => 0,
        };

        let desired_state_int = match (
            state.desired_state.as_ref(),
            ResourceType::try_from(state.resource_type),
        ) {
            (Some(desired), Ok(ResourceType::Scenario)) => {
                ScenarioState::from_str_name(desired).map(|s| s as i32)
            }
            (Some(desired), Ok(ResourceType::Package)) => {
                PackageState::from_str_name(desired).map(|s| s as i32)
            }
            (Some(desired), Ok(ResourceType::Model)) => {
                ModelState::from_str_name(desired).map(|s| s as i32)
            }
            _ => None,
        };

        ResourceState {
            resource_type: ResourceType::try_from(state.resource_type)
                .unwrap_or(ResourceType::Scenario),
            resource_name: state.resource_name,
            current_state: current_state_int,
            desired_state: desired_state_int,
            last_transition_time: Instant::now(),
            transition_count: state.transition_count,
            metadata: state.metadata,
            health_status: HealthStatus::from(state.health_status),
        }
    }
}

// Your existing StateTransition
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from_state: i32,
    pub event: String,
    pub to_state: i32,
    pub condition: Option<String>,
    pub action: String,
}

// Enhanced TransitionResult with better proto compatibility
#[derive(Debug, Clone)]
pub struct TransitionResult {
    pub new_state: i32,
    pub error_code: ErrorCode,
    pub message: String,
    pub actions_to_execute: Vec<String>,
    pub transition_id: String,
    pub error_details: String,
    pub success: bool,
    pub timestamp_ns: i64,
}

impl TransitionResult {
    /// Create a successful transition result
    pub fn success(new_state: i32, transition_id: String, message: Option<String>) -> Self {
        Self {
            new_state,
            error_code: ErrorCode::Success,
            message: message.unwrap_or_else(|| "Transition completed successfully".to_string()),
            actions_to_execute: Vec::new(),
            transition_id,
            error_details: String::new(),
            success: true,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as i64,
        }
    }

    /// Create a failed transition result
    pub fn failure(
        current_state: i32,
        transition_id: String,
        error_code: ErrorCode,
        message: String,
        error_details: String,
    ) -> Self {
        Self {
            new_state: current_state, // Stay in current state on failure
            error_code,
            message,
            actions_to_execute: Vec::new(),
            transition_id,
            error_details,
            success: false,
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as i64,
        }
    }

    /// Check if the transition was successful
    pub fn is_success(&self) -> bool {
        self.success && self.error_code == ErrorCode::Success
    }

    /// Check if the transition failed
    pub fn is_failure(&self) -> bool {
        !self.success || self.error_code != ErrorCode::Success
    }

    /// Convert TransitionResult to StateChangeResponse for proto compatibility
    pub fn to_state_change_response(&self) -> common::statemanager::StateChangeResponse {
        common::statemanager::StateChangeResponse {
            message: self.message.clone(),
            transition_id: self.transition_id.clone(),
            timestamp_ns: self.timestamp_ns,
            error_code: self.error_code as i32,
            error_details: self.error_details.clone(),
        }
    }

    /// Add an action to be executed
    pub fn with_action(mut self, action: String) -> Self {
        self.actions_to_execute.push(action);
        self
    }

    /// Add multiple actions to be executed
    pub fn with_actions(mut self, actions: Vec<String>) -> Self {
        self.actions_to_execute.extend(actions);
        self
    }
}

// Your existing ActionCommand
#[derive(Debug, Clone)]
pub struct ActionCommand {
    pub action: String,
    pub resource_key: String,
    pub resource_type: ResourceType,
    pub transition_id: String,
    pub context: HashMap<String, String>,
}

impl ActionCommand {
    /// Create a new ActionCommand
    pub fn new(
        action: String,
        resource_key: String,
        resource_type: ResourceType,
        transition_id: String,
    ) -> Self {
        Self {
            action,
            resource_key,
            resource_type,
            transition_id,
            context: HashMap::new(),
        }
    }

    /// Add context information
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Add multiple context entries
    pub fn with_context_map(mut self, context: HashMap<String, String>) -> Self {
        self.context.extend(context);
        self
    }
}

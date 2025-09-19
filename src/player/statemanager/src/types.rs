use common::statemanager::{ErrorCode, ResourceType};
use std::collections::HashMap;
use tokio::time::Instant;
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
    pub from_state: i32,
    pub event: String,
    pub to_state: i32,
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
    pub current_state: i32,
    pub desired_state: Option<i32>,
    pub last_transition_time: Instant,
    pub transition_count: u64,
    pub metadata: HashMap<String, String>,
    pub health_status: HealthStatus,
}

/// Result of a state transition attempt - aligned with proto StateChangeResponse
#[derive(Debug, Clone)]
pub struct TransitionResult {
    pub new_state: i32,
    pub error_code: ErrorCode,
    pub message: String,
    pub actions_to_execute: Vec<String>,
    pub transition_id: String,
    pub error_details: String,
}

/// Container state aggregation for model state computation
///
/// This struct holds aggregated information about containers belonging to a model
/// to enable state computation according to the documented rules.
#[derive(Debug, Clone)]
pub struct ContainerStateAggregation {
    pub total_containers: usize,
    pub running_count: usize,
    pub paused_count: usize,
    pub exited_count: usize,
    pub dead_count: usize,
    pub created_count: usize,
    pub unknown_count: usize,
    pub any_failed: bool,
}

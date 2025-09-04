use common::statemanager::{ModelState, PackageState, ResourceType, ScenarioState};
use tracing::trace;

pub struct EventInference;

impl EventInference {
    pub fn infer_event_from_states(
        current_state: i32,
        target_state: i32,
        resource_type: ResourceType,
    ) -> String {
        trace!(
            "Inferring event for {:?}: {} -> {}",
            resource_type,
            current_state,
            target_state
        );

        let event = match resource_type {
            ResourceType::Scenario => Self::infer_scenario_event(current_state, target_state),
            ResourceType::Package => Self::infer_package_event(current_state, target_state),
            ResourceType::Model => Self::infer_model_event(current_state, target_state),
            _ => format!("transition_{}_{}", current_state, target_state),
        };

        trace!("Inferred event: {}", event);
        event
    }

    fn infer_scenario_event(current_state: i32, target_state: i32) -> String {
        match (current_state, target_state) {
            (x, y) if x == ScenarioState::Idle as i32 && y == ScenarioState::Waiting as i32 => {
                "scenario_activation".to_string()
            }
            (x, y) if x == ScenarioState::Waiting as i32 && y == ScenarioState::Allowed as i32 => {
                "condition_met".to_string()
            }
            (x, y) if x == ScenarioState::Allowed as i32 && y == ScenarioState::Playing as i32 => {
                "policy_verification_success".to_string()
            }
            (x, y) if x == ScenarioState::Allowed as i32 && y == ScenarioState::Denied as i32 => {
                "policy_verification_failure".to_string()
            }
            _ => format!("transition_{}_{}", current_state, target_state),
        }
    }

    fn infer_package_event(current_state: i32, target_state: i32) -> String {
        match (current_state, target_state) {
            (x, y)
                if x == PackageState::Unspecified as i32
                    && y == PackageState::Initializing as i32 =>
            {
                "launch_request".to_string()
            }
            (x, y)
                if x == PackageState::Initializing as i32 && y == PackageState::Running as i32 =>
            {
                "initialization_complete".to_string()
            }
            (x, y)
                if x == PackageState::Initializing as i32 && y == PackageState::Degraded as i32 =>
            {
                "partial_initialization_failure".to_string()
            }
            (x, y) if x == PackageState::Initializing as i32 && y == PackageState::Error as i32 => {
                "critical_initialization_failure".to_string()
            }
            (x, y) if x == PackageState::Running as i32 && y == PackageState::Degraded as i32 => {
                "model_issue_detected".to_string()
            }
            (x, y) if x == PackageState::Running as i32 && y == PackageState::Error as i32 => {
                "critical_issue_detected".to_string()
            }
            (x, y) if x == PackageState::Running as i32 && y == PackageState::Paused as i32 => {
                "pause_request".to_string()
            }
            (x, y) if x == PackageState::Running as i32 && y == PackageState::Updating as i32 => {
                "update_request".to_string()
            }
            (x, y) if x == PackageState::Degraded as i32 && y == PackageState::Running as i32 => {
                "model_recovery".to_string()
            }
            (x, y) if x == PackageState::Degraded as i32 && y == PackageState::Error as i32 => {
                "additional_model_issues".to_string()
            }
            (x, y) if x == PackageState::Degraded as i32 && y == PackageState::Paused as i32 => {
                "pause_request".to_string()
            }
            (x, y) if x == PackageState::Error as i32 && y == PackageState::Running as i32 => {
                "recovery_successful".to_string()
            }
            (x, y) if x == PackageState::Paused as i32 && y == PackageState::Running as i32 => {
                "resume_request".to_string()
            }
            (x, y) if x == PackageState::Updating as i32 && y == PackageState::Running as i32 => {
                "update_successful".to_string()
            }
            (x, y) if x == PackageState::Updating as i32 && y == PackageState::Error as i32 => {
                "update_failed".to_string()
            }
            _ => format!("transition_{}_{}", current_state, target_state),
        }
    }

    fn infer_model_event(current_state: i32, target_state: i32) -> String {
        match (current_state, target_state) {
            (x, y) if x == ModelState::Unspecified as i32 && y == ModelState::Pending as i32 => {
                "creation_request".to_string()
            }
            (x, y)
                if x == ModelState::Pending as i32 && y == ModelState::ContainerCreating as i32 =>
            {
                "node_allocation_complete".to_string()
            }
            (x, y) if x == ModelState::Pending as i32 && y == ModelState::Failed as i32 => {
                "node_allocation_failed".to_string()
            }
            (x, y)
                if x == ModelState::ContainerCreating as i32 && y == ModelState::Running as i32 =>
            {
                "container_creation_complete".to_string()
            }
            (x, y)
                if x == ModelState::ContainerCreating as i32 && y == ModelState::Failed as i32 =>
            {
                "container_creation_failed".to_string()
            }
            (x, y) if x == ModelState::Running as i32 && y == ModelState::Succeeded as i32 => {
                "temporary_task_complete".to_string()
            }
            (x, y) if x == ModelState::Running as i32 && y == ModelState::Failed as i32 => {
                "container_termination".to_string()
            }
            (x, y)
                if x == ModelState::Running as i32 && y == ModelState::CrashLoopBackOff as i32 =>
            {
                "repeated_crash_detection".to_string()
            }
            (x, y) if x == ModelState::Running as i32 && y == ModelState::Unknown as i32 => {
                "monitoring_failure".to_string()
            }
            (x, y)
                if x == ModelState::CrashLoopBackOff as i32 && y == ModelState::Running as i32 =>
            {
                "backoff_time_elapsed".to_string()
            }
            (x, y)
                if x == ModelState::CrashLoopBackOff as i32 && y == ModelState::Failed as i32 =>
            {
                "maximum_retries_exceeded".to_string()
            }
            (x, y) if x == ModelState::Unknown as i32 && y == ModelState::Running as i32 => {
                "state_check_recovered".to_string()
            }
            (x, y) if x == ModelState::Failed as i32 && y == ModelState::Pending as i32 => {
                "manual_automatic_recovery".to_string()
            }
            _ => format!("transition_{}_{}", current_state, target_state),
        }
    }
}

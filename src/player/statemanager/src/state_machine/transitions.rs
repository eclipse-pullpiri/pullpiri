use crate::core::types::StateTransition; // Fix: use core::types
use common::statemanager::{ModelState, PackageState, ScenarioState};

pub struct ScenarioTransitions;

impl ScenarioTransitions {
    pub fn get_transitions() -> Vec<StateTransition> {
        vec![
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
                to_state: ScenarioState::Allowed as i32,
                condition: None,
                action: "start_policy_verification".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Allowed as i32,
                event: "policy_verification_success".to_string(),
                to_state: ScenarioState::Playing as i32,
                condition: None,
                action: "execute_action_on_target_package".to_string(),
            },
            StateTransition {
                from_state: ScenarioState::Allowed as i32,
                event: "policy_verification_failure".to_string(),
                to_state: ScenarioState::Denied as i32,
                condition: None,
                action: "log_denial_generate_alert".to_string(),
            },
        ]
    }
}

pub struct PackageTransitions;

impl PackageTransitions {
    pub fn get_transitions() -> Vec<StateTransition> {
        vec![
            StateTransition {
                from_state: PackageState::Unspecified as i32,
                event: "launch_request".to_string(),
                to_state: PackageState::Initializing as i32,
                condition: None,
                action: "start_model_creation_allocate_resources".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing as i32,
                event: "initialization_complete".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("all_models_normal".to_string()),
                action: "update_state_announce_availability".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing as i32,
                event: "partial_initialization_failure".to_string(),
                to_state: PackageState::Degraded as i32,
                condition: Some("critical_models_normal".to_string()),
                action: "log_warning_activate_partial_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Initializing as i32,
                event: "critical_initialization_failure".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("critical_models_failed".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "model_issue_detected".to_string(),
                to_state: PackageState::Degraded as i32,
                condition: Some("non_critical_model_issues".to_string()),
                action: "log_warning_maintain_partial_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "critical_issue_detected".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("critical_model_issues".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "pause_request".to_string(),
                to_state: PackageState::Paused as i32,
                condition: None,
                action: "pause_models_preserve_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded as i32,
                event: "model_recovery".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("all_models_recovered".to_string()),
                action: "update_state_restore_full_functionality".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded as i32,
                event: "additional_model_issues".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("critical_models_affected".to_string()),
                action: "log_error_attempt_recovery".to_string(),
            },
            StateTransition {
                from_state: PackageState::Degraded as i32,
                event: "pause_request".to_string(),
                to_state: PackageState::Paused as i32,
                condition: None,
                action: "pause_models_preserve_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Error as i32,
                event: "recovery_successful".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("depends_on_recovery_level".to_string()),
                action: "update_state_announce_functionality_restoration".to_string(),
            },
            StateTransition {
                from_state: PackageState::Paused as i32,
                event: "resume_request".to_string(),
                to_state: PackageState::Running as i32,
                condition: Some("depends_on_previous_state".to_string()),
                action: "resume_models_restore_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Running as i32,
                event: "update_request".to_string(),
                to_state: PackageState::Updating as i32,
                condition: None,
                action: "start_update_process".to_string(),
            },
            StateTransition {
                from_state: PackageState::Updating as i32,
                event: "update_successful".to_string(),
                to_state: PackageState::Running as i32,
                condition: None,
                action: "activate_new_version_update_state".to_string(),
            },
            StateTransition {
                from_state: PackageState::Updating as i32,
                event: "update_failed".to_string(),
                to_state: PackageState::Error as i32,
                condition: Some("depends_on_rollback_settings".to_string()),
                action: "rollback_or_error_handling".to_string(),
            },
        ]
    }
}

pub struct ModelTransitions;

impl ModelTransitions {
    pub fn get_transitions() -> Vec<StateTransition> {
        vec![
            StateTransition {
                from_state: ModelState::Unspecified as i32,
                event: "creation_request".to_string(),
                to_state: ModelState::Pending as i32,
                condition: None,
                action: "start_node_selection_and_allocation".to_string(),
            },
            StateTransition {
                from_state: ModelState::Pending as i32,
                event: "node_allocation_complete".to_string(),
                to_state: ModelState::ContainerCreating as i32,
                condition: Some("sufficient_resources".to_string()),
                action: "pull_container_images_mount_volumes".to_string(),
            },
            StateTransition {
                from_state: ModelState::Pending as i32,
                event: "node_allocation_failed".to_string(),
                to_state: ModelState::Failed as i32,
                condition: Some("timeout_or_error".to_string()),
                action: "log_error_retry_or_reschedule".to_string(),
            },
            StateTransition {
                from_state: ModelState::ContainerCreating as i32,
                event: "container_creation_complete".to_string(),
                to_state: ModelState::Running as i32,
                condition: Some("all_containers_started".to_string()),
                action: "update_state_start_readiness_checks".to_string(),
            },
            StateTransition {
                from_state: ModelState::ContainerCreating as i32,
                event: "container_creation_failed".to_string(),
                to_state: ModelState::Failed as i32,
                condition: None,
                action: "log_error_retry_or_reschedule".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "temporary_task_complete".to_string(),
                to_state: ModelState::Succeeded as i32,
                condition: Some("one_time_task".to_string()),
                action: "log_completion_clean_up_resources".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "container_termination".to_string(),
                to_state: ModelState::Failed as i32,
                condition: Some("unexpected_termination".to_string()),
                action: "log_error_evaluate_automatic_restart".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "repeated_crash_detection".to_string(),
                to_state: ModelState::CrashLoopBackOff as i32,
                condition: Some("consecutive_restart_failures".to_string()),
                action: "set_backoff_timer_collect_logs".to_string(),
            },
            StateTransition {
                from_state: ModelState::Running as i32,
                event: "monitoring_failure".to_string(),
                to_state: ModelState::Unknown as i32,
                condition: Some("node_communication_issues".to_string()),
                action: "attempt_diagnostics_restore_communication".to_string(),
            },
            StateTransition {
                from_state: ModelState::CrashLoopBackOff as i32,
                event: "backoff_time_elapsed".to_string(),
                to_state: ModelState::Running as i32,
                condition: Some("restart_successful".to_string()),
                action: "resume_monitoring_reset_counter".to_string(),
            },
            StateTransition {
                from_state: ModelState::CrashLoopBackOff as i32,
                event: "maximum_retries_exceeded".to_string(),
                to_state: ModelState::Failed as i32,
                condition: Some("retry_limit_reached".to_string()),
                action: "log_error_notify_for_manual_intervention".to_string(),
            },
            StateTransition {
                from_state: ModelState::Unknown as i32,
                event: "state_check_recovered".to_string(),
                to_state: ModelState::Running as i32,
                condition: Some("depends_on_actual_state".to_string()),
                action: "synchronize_state_recover_if_needed".to_string(),
            },
            StateTransition {
                from_state: ModelState::Failed as i32,
                event: "manual_automatic_recovery".to_string(),
                to_state: ModelState::Pending as i32,
                condition: Some("according_to_restart_policy".to_string()),
                action: "start_model_recreation".to_string(),
            },
        ]
    }
}

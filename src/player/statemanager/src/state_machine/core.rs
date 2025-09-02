use crate::monitoring::health::HealthManager;
use crate::core::types::{ActionCommand, ResourceState, StateTransition, TransitionResult};
use crate::utils::utility::StateUtilities;
use crate::monitoring::validation::StateValidator;
use crate::storage::etcd_state;
use common::statemanager::{ErrorCode, ResourceType, StateChange};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::{debug, error, trace, warn};

/// Core state machine implementation for PICCOLO resource management
pub struct StateMachine {
    /// State transition tables indexed by resource type
    transition_tables: HashMap<ResourceType, Vec<StateTransition>>,
    
    /// Current state tracking for all managed resources
    resource_states: HashMap<String, ResourceState>,
    
    /// Backoff timers for CrashLoopBackOff and retry management
    backoff_timers: HashMap<String, Instant>,
    
    /// Action command sender for async execution
    action_sender: Option<mpsc::UnboundedSender<ActionCommand>>,
    
    /// Health manager for resource health tracking
    health_manager: HealthManager,
}

impl StateMachine {
    /// Creates a new StateMachine with predefined transition tables
    pub fn new() -> Self {
        println!("Initializing new StateMachine instance");
        
        let mut state_machine = StateMachine {
            transition_tables: HashMap::new(),
            resource_states: HashMap::new(),
            backoff_timers: HashMap::new(),
            action_sender: None,
            health_manager: HealthManager::new(),
        };

        // Initialize transition tables for each resource type
        debug!("Initializing state transition tables");
        state_machine.initialize_all_transitions();

        println!(
            "StateMachine initialized with {} resource types", 
            state_machine.transition_tables.len()
        );

        state_machine
    }

    /// Initialize async action executor
    pub fn initialize_action_executor(&mut self) -> mpsc::UnboundedReceiver<ActionCommand> {
        println!("Initializing async action executor");
        let (sender, receiver) = mpsc::unbounded_channel();
        self.action_sender = Some(sender);
        println!("Action executor initialized successfully");
        receiver
    }

    /// Initialize all transition tables
    fn initialize_all_transitions(&mut self) {
        use super::transitions::*;
        
        self.transition_tables.insert(
            ResourceType::Scenario, 
            ScenarioTransitions::get_transitions()
        );
        self.transition_tables.insert(
            ResourceType::Package, 
            PackageTransitions::get_transitions()
        );
        self.transition_tables.insert(
            ResourceType::Model, 
            ModelTransitions::get_transitions()
        );
    }

    /// Process a state change request with non-blocking action execution
    pub async fn process_state_change(&mut self, state_change: StateChange) -> TransitionResult {
        println!(
            "Processing state change: {} -> {} for resource '{}'",
            state_change.current_state, state_change.target_state, state_change.resource_name
        );

        // Validate input parameters
        if let Err(validation_error) = StateValidator::validate_state_change(&state_change) {
            error!(
                "State change validation failed for resource '{}': {}",
                state_change.resource_name, validation_error
            );
            return TransitionResult::failure(
                StateUtilities::state_str_to_enum(
                    state_change.current_state.as_str(),
                    state_change.resource_type,
                ),
                state_change.transition_id.clone(),
                ErrorCode::InvalidRequest,
                format!("Invalid state change request: {}", validation_error),
                validation_error,
            );
        }

        // Convert i32 to ResourceType enum
        let resource_type = match ResourceType::try_from(state_change.resource_type) {
            Ok(rt) => {
                debug!("Resource type resolved: {:?}", rt);
                rt
            }
            Err(_) => {
                error!(
                    "Invalid resource type {} for resource '{}'", 
                    state_change.resource_type, state_change.resource_name
                );
                return TransitionResult::failure(
                    StateUtilities::state_str_to_enum(
                        state_change.current_state.as_str(),
                        state_change.resource_type,
                    ),
                    state_change.transition_id.clone(),
                    ErrorCode::InvalidStateTransition,
                    format!("Invalid resource type: {}", state_change.resource_type),
                    format!(
                        "Unsupported resource type ID: {}",
                        state_change.resource_type
                    ),
                );
            }
        };

        let resource_key = StateUtilities::generate_resource_key(resource_type, &state_change.resource_name);
        trace!("Generated resource key: {}", resource_key);

        // Get current state from storage
        let current_state: i32 = match super::persistence::StatePersistence::get_current_state_from_storage(
            &resource_key,
            &state_change.current_state,
            state_change.resource_type,
        ).await {
            Ok(state) => state,
            Err(e) => {
                error!("Failed to retrieve state from etcd for {}: {}", resource_key, e);
                return TransitionResult::failure(
                    StateUtilities::state_str_to_enum(
                        state_change.current_state.as_str(),
                        state_change.resource_type,
                    ),
                    state_change.transition_id.clone(),
                    ErrorCode::InternalError,
                    format!("Failed to retrieve state from etcd: {}", e),
                    format!("{}", e),
                );
            }
        };

        // Check backoff period
        if let Err((error_code, message)) = super::backoff::BackoffManager::check_backoff_period(
            &self.backoff_timers,
            &resource_key,
            current_state,
        ) {
            return TransitionResult::failure(
                current_state,
                state_change.transition_id.clone(),
                error_code,
                message,
                "Backoff timer has not elapsed yet".to_string(),
            );
        }

        // Find valid transition
        let target_state_int = StateUtilities::state_str_to_enum(
            state_change.target_state.as_str(),
            state_change.resource_type,
        );
        
        let transition_event = super::events::EventInference::infer_event_from_states(
            current_state,
            target_state_int,
            resource_type,
        );
        
        debug!(
            "Inferred transition event '{}' for {} -> {}",
            transition_event,
            StateUtilities::state_enum_to_str(current_state, resource_type),
            StateUtilities::state_enum_to_str(target_state_int, resource_type)
        );

        if let Some(transition) = self.find_valid_transition(
            resource_type,
            current_state,
            &transition_event,
            target_state_int,
        ) {
            println!(
                "Valid transition found: {} -> {} via event '{}'",
                StateUtilities::state_enum_to_str(current_state, resource_type),
                StateUtilities::state_enum_to_str(transition.to_state, resource_type),
                transition.event
            );

            // Check conditions if any
            if let Some(condition) = &transition.condition {
                debug!("Evaluating transition condition: {}", condition);
                if !StateValidator::evaluate_condition(condition, &state_change) {
                    warn!(
                        "Transition condition '{}' not met for resource '{}'",
                        condition, state_change.resource_name
                    );
                    return TransitionResult::failure(
                        current_state,
                        state_change.transition_id.clone(),
                        ErrorCode::PreconditionFailed,
                        format!("Transition condition not met: {}", condition),
                        "Transition condition failed".to_string(),
                    );
                }
                debug!("Transition condition satisfied");
            }

            // Execute transition - update state
            debug!("Executing state transition to etcd");
            if let Err(e) = super::persistence::StatePersistence::update_resource_state(
                &mut self.resource_states,
                &resource_key,
                &state_change,
                transition.to_state,
                resource_type,
            ).await {
                error!(
                    "Failed to update resource state for {}: {}",
                    resource_key, e
                );
                return TransitionResult::failure(
                    current_state,
                    state_change.transition_id.clone(),
                    ErrorCode::InternalError,
                    format!("Failed to update resource state: {}", e),
                    format!("{}", e),
                );
            }

            // Initialize health tracking if needed
            if !self.health_manager.get_health_status(&resource_key).is_some() {
                self.health_manager.initialize_health_tracking(resource_key.clone());
            }

            // NON-BLOCKING ACTION EXECUTION
            if let Some(ref sender) = self.action_sender {
                let action_command = ActionCommand {
                    action: transition.action.clone(),
                    resource_key: resource_key.clone(),
                    resource_type,
                    transition_id: state_change.transition_id.clone(),
                    context: StateUtilities::build_action_context(&state_change, &transition),
                };

                debug!("Queuing action '{}' for async execution", transition.action);
                if let Err(e) = sender.send(action_command) {
                    error!("Failed to queue action '{}' for execution: {}", transition.action, e);
                } else {
                    trace!("Action '{}' queued successfully", transition.action);
                }
            } else {
                warn!("Action sender not initialized, action '{}' will not be executed", transition.action);
            }
            
            // Handle special state-specific logic
            super::backoff::BackoffManager::set_backoff_timer(
                &mut self.backoff_timers,
                &resource_key,
                transition.to_state,
            );
            
            let transitioned_state_str = StateUtilities::state_enum_to_str(transition.to_state, resource_type);

            // Create the transition result
            let transition_result = TransitionResult::success(
                transition.to_state,
                state_change.transition_id.clone(),
                Some(format!("State transition completed successfully to {}", transitioned_state_str)),
            );

            // NOW update health status AFTER transition_result is created
            self.health_manager.update_health_status(&resource_key, &transition_result);

            println!(
                "State transition completed successfully: {} -> {} for resource '{}'",
                StateUtilities::state_enum_to_str(current_state, resource_type),
                transitioned_state_str,
                state_change.resource_name
            );

            transition_result
        } else {
            let current_state_str = StateUtilities::state_enum_to_str(current_state, resource_type);
            let target_state_str = StateUtilities::state_enum_to_str(target_state_int, resource_type);

            error!(
                "No valid transition found from {} to {} for resource type {:?}",
                current_state_str, target_state_str, resource_type
            );

            let transition_result = TransitionResult::failure(
                current_state,
                state_change.transition_id.clone(),
                ErrorCode::InvalidStateTransition,
                format!(
                    "No valid transition from {} to {} for resource type {:?}",
                    current_state_str, target_state_str, resource_type
                ),
                format!(
                    "Invalid state transition attempted: {} -> {}",
                    current_state_str, target_state_str
                ),
            );

            // Also update health status for failures
            self.health_manager.update_health_status(&resource_key, &transition_result);
            transition_result
        }
    }

    /// Load all existing states from etcd on startup
    pub async fn load_states_from_etcd(&mut self) -> common::Result<()> {
        println!("Starting to load existing states from etcd");
        
        let mut loaded_count = 0;
        let mut error_count = 0;
        
        match super::persistence::StatePersistence::load_all_states().await {
            Ok(states) => {
                println!("Retrieved {} states from etcd", states.len());
                
                for (resource_key, serializable_state) in states {
                    match self.load_single_state(resource_key, serializable_state).await {
                        Ok(()) => {
                            loaded_count += 1;
                        }
                        Err(e) => {
                            error_count += 1;
                            error!("Failed to load state: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to retrieve states from etcd: {}", e);
                return Err(e);
            }
        }
        
        println!(
            "State loading completed: {} successful, {} errors, {} total resources in cache", 
            loaded_count, error_count, self.resource_states.len()
        );
        
        if error_count > 0 {
            warn!("Attempting to clean up invalid states due to {} errors", error_count);
            match crate::storage::etcd_state::cleanup_invalid_states().await {
                Ok(cleaned) => println!("Successfully cleaned up {} invalid states", cleaned),
                Err(e) => error!("Failed to clean up invalid states: {}", e),
            }
        }
        
        Ok(())
    }
    
    /// Load a single state into the in-memory cache
    async fn load_single_state(
        &mut self,
        resource_key: String,
        serializable_state: crate::core::types::SerializableResourceState,
    ) -> common::Result<()> {
        trace!("Loading single state for resource: {}", resource_key);

        if !self.validate_loaded_state(&serializable_state) {
            warn!("Invalid state for resource: {}", resource_key);
            return Err(format!("Invalid state for resource: {}", resource_key).into());
        }

        let runtime_state = ResourceState::from(serializable_state.clone());
        self.resource_states.insert(resource_key.clone(), runtime_state);
        
        super::backoff::BackoffManager::restore_backoff_timer(
            &mut self.backoff_timers,
            &resource_key,
            &serializable_state,
        )?;
        
        let state_name = &serializable_state.current_state;

        debug!(
            "Successfully loaded state for {}: {} (transitions: {})", 
            resource_key, state_name, serializable_state.transition_count
        );
        
        Ok(())
    }

    /// Validate a state loaded from etcd
    fn validate_loaded_state(&self, state: &crate::core::types::SerializableResourceState) -> bool {
        trace!("Validating loaded state for resource: {}", state.resource_name);

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
            Ok(ResourceType::Scenario) => common::statemanager::ScenarioState::from_str_name(&state.current_state).is_some(),
            Ok(ResourceType::Package) => common::statemanager::PackageState::from_str_name(&state.current_state).is_some(),
            Ok(ResourceType::Model) => common::statemanager::ModelState::from_str_name(&state.current_state).is_some(),
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

        trace!("State validation passed");
        true
    }

    /// Cache warming: load frequently accessed states into memory
    pub async fn warm_cache_for_active_resources(&mut self) -> common::Result<()> {
        println!("Starting cache warming for active resources");

        let active_prefixes = vec!["Scenario::", "Package::", "Model::"];
        let mut total_warmed = 0;

        for prefix in active_prefixes {
            let resource_type = ResourceType::try_from(match prefix {
                "Scenario::" => 1,
                "Package::" => 2,
                "Model::" => 3,
                _ => 0,
            }).unwrap_or(ResourceType::Scenario);

            debug!("Warming cache for resource type: {:?}", resource_type);

            if let Ok(states) = crate::storage::etcd_state::get_resource_states_by_type(resource_type).await {
                for (key, serializable_state) in states {
                    if StateUtilities::is_active_state(
                        StateUtilities::enum_str_to_int(&serializable_state.current_state, serializable_state.resource_type),
                        serializable_state.resource_type,
                    ) {
                        let runtime_state = ResourceState::from(serializable_state);
                        self.resource_states.insert(key.clone(), runtime_state);
                        total_warmed += 1;
                        trace!("Warmed cache for: {}", key);
                    }
                }
            }
        }

        println!("Cache warming completed: {} resources loaded", total_warmed);
        Ok(())
    }

    /// Find a valid transition rule for the given parameters
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

    /// Get backoff timers (for external access)
    pub fn get_backoff_timers(&self) -> &HashMap<String, Instant> {
        &self.backoff_timers
    }

    /// Get mutable backoff timers (for external access)
    pub fn get_backoff_timers_mut(&mut self) -> &mut HashMap<String, Instant> {
        &mut self.backoff_timers
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}
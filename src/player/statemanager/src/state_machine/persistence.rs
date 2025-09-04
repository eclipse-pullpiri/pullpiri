use crate::core::types::{ResourceState, SerializableHealthStatus, SerializableResourceState};
use crate::storage::etcd_state::{get_all_resource_states, get_current_state, set_current_state};
use crate::utils::utility::StateUtilities;
use common::statemanager::{ResourceType, StateChange};
use std::collections::HashMap;
use tracing::{debug, error, trace};

pub struct StatePersistence;

impl StatePersistence {
    pub async fn load_all_states() -> common::Result<HashMap<String, SerializableResourceState>> {
        println!("Starting to load existing states from etcd");

        match get_all_resource_states().await {
            Ok(states) => {
                // Convert Vec<(String, SerializableResourceState)> to HashMap
                let states_map: HashMap<String, SerializableResourceState> =
                    states.into_iter().collect();
                Ok(states_map)
            }
            Err(e) => {
                error!("Failed to retrieve states from etcd: {}", e);
                Err(e)
            }
        }
    }

    pub async fn get_current_state_from_storage(
        resource_key: &str,
        fallback_state: &str,
        resource_type: i32,
    ) -> common::Result<i32> {
        match get_current_state(resource_key).await {
            Ok(Some(serializable_state)) => {
                println!(
                    "Found existing state for {}: {}",
                    resource_key, serializable_state.current_state
                );
                Ok(StateUtilities::enum_str_to_int(
                    &serializable_state.current_state,
                    resource_type,
                ))
            }
            Ok(None) => {
                println!(
                    "Resource {} not found in etcd, using provided current state",
                    resource_key
                );
                Ok(StateUtilities::state_str_to_enum(
                    fallback_state,
                    resource_type,
                ))
            }
            Err(e) => {
                error!(
                    "Failed to retrieve state from etcd for {}: {}",
                    resource_key, e
                );
                Err(e)
            }
        }
    }

    pub async fn update_resource_state(
        resource_states: &mut HashMap<String, ResourceState>,
        resource_key: &str,
        state_change: &StateChange,
        new_state: i32,
        resource_type: ResourceType,
    ) -> common::Result<()> {
        debug!("Updating resource state for: {}", resource_key);

        let existing_state = resource_states.get(resource_key);
        let updated_state =
            Self::build_updated_state(existing_state, state_change, new_state, resource_type);

        // Write-through: persist to etcd FIRST (durability)
        debug!("Persisting state to etcd");
        set_current_state(resource_key, &updated_state).await?;

        // Then update in-memory cache (performance)
        let runtime_state = ResourceState::from(updated_state);
        resource_states.insert(resource_key.to_string(), runtime_state);

        let state_name = StateUtilities::state_enum_to_str(new_state, resource_type);
        println!(
            "Successfully updated state for {} to {}",
            resource_key, state_name
        );
        Ok(())
    }

    fn build_updated_state(
        existing_state: Option<&ResourceState>,
        state_change: &StateChange,
        new_state: i32,
        resource_type: ResourceType,
    ) -> SerializableResourceState {
        match existing_state {
            Some(current) => {
                trace!("Updating existing state");
                let mut serializable = SerializableResourceState::from(current.clone());
                serializable.current_state =
                    StateUtilities::state_enum_to_str(new_state, resource_type).to_string();
                serializable.desired_state = Some(
                    StateUtilities::state_enum_to_str(
                        StateUtilities::state_str_to_enum(
                            state_change.target_state.as_str(),
                            state_change.resource_type,
                        ),
                        resource_type,
                    )
                    .to_string(),
                );
                serializable.last_transition_unix_timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                serializable.transition_count += 1;
                serializable
            }
            None => {
                trace!("Creating new state record");
                let desired_state_enum = StateUtilities::state_str_to_enum(
                    state_change.target_state.as_str(),
                    state_change.resource_type,
                );
                SerializableResourceState {
                    resource_type: resource_type as i32,
                    resource_name: state_change.resource_name.clone(),
                    current_state: StateUtilities::state_enum_to_str(new_state, resource_type)
                        .to_string(),
                    desired_state: Some(
                        StateUtilities::state_enum_to_str(desired_state_enum, resource_type)
                            .to_string(),
                    ),
                    last_transition_unix_timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                    transition_count: 1,
                    metadata: HashMap::new(),
                    health_status: SerializableHealthStatus {
                        healthy: true,
                        status_message: "Healthy".to_string(),
                        last_check_unix_timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        consecutive_failures: 0,
                    },
                }
            }
        }
    }
}

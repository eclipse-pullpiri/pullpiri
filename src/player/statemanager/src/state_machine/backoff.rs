use crate::core::config::BACKOFF_DURATION_SECS;
use crate::core::types::SerializableResourceState;
use crate::utils::utility::StateUtilities;
use common::statemanager::{ErrorCode, ModelState};
use std::collections::HashMap;
use tokio::time::{Duration, Instant};
use tracing::{debug, warn};

pub struct BackoffManager;

impl BackoffManager {
    pub fn check_backoff_period(
        backoff_timers: &HashMap<String, Instant>,
        resource_key: &str,
        current_state: i32,
    ) -> Result<(), (ErrorCode, String)> {
        if current_state == ModelState::CrashLoopBackOff as i32 {
            if let Some(backoff_time) = backoff_timers.get(resource_key) {
                let remaining = Duration::from_secs(BACKOFF_DURATION_SECS)
                    .saturating_sub(backoff_time.elapsed());

                if !remaining.is_zero() {
                    warn!(
                        "Resource {} is in backoff period, {} seconds remaining",
                        resource_key,
                        remaining.as_secs()
                    );
                    return Err((
                        ErrorCode::PreconditionFailed,
                        "Resource is in backoff period".to_string(),
                    ));
                }

                debug!(
                    "Backoff period elapsed for {}, proceeding with transition",
                    resource_key
                );
            }
        }
        Ok(())
    }

    pub fn set_backoff_timer(
        backoff_timers: &mut HashMap<String, Instant>,
        resource_key: &str,
        to_state: i32,
    ) {
        if to_state == ModelState::CrashLoopBackOff as i32 {
            backoff_timers.insert(resource_key.to_string(), Instant::now());
            println!("Set backoff timer for resource {}", resource_key);
        }
    }

    pub fn restore_backoff_timer(
        backoff_timers: &mut HashMap<String, Instant>,
        resource_key: &str,
        state: &SerializableResourceState,
    ) -> common::Result<()> {
        let current_state_int =
            StateUtilities::enum_str_to_int(&state.current_state, state.resource_type);

        if current_state_int == ModelState::CrashLoopBackOff as i32 {
            let backoff_start_time = std::time::UNIX_EPOCH
                + std::time::Duration::from_secs(state.last_transition_unix_timestamp);

            let elapsed_since_boot = std::time::SystemTime::now()
                .duration_since(backoff_start_time)
                .unwrap_or_default();

            let backoff_instant = if elapsed_since_boot < Duration::from_secs(BACKOFF_DURATION_SECS)
            {
                Instant::now() - elapsed_since_boot
            } else {
                Instant::now() - Duration::from_secs(BACKOFF_DURATION_SECS + 1)
            };

            backoff_timers.insert(resource_key.to_string(), backoff_instant);
            println!("Restored backoff timer for resource: {}", resource_key);
        }

        Ok(())
    }
}

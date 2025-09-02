/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Health monitoring and status tracking for StateManager resources

use crate::core::config::MAX_CONSECUTIVE_FAILURES;
use crate::core::types::{TransitionResult, HealthStatus};
use common::statemanager::ErrorCode;
use std::collections::HashMap;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, warn};

pub struct HealthManager {
    /// Health status tracking for each resource
    health_statuses: HashMap<String, HealthStatus>,
}

impl HealthManager {
    pub fn new() -> Self {
        Self {
            health_statuses: HashMap::new(),
        }
    }

    /// Updates health status based on transition result
    pub fn update_health_status(&mut self, resource_key: &str, transition_result: &TransitionResult) {
        tracing::trace!("Updating health status for resource: {}", resource_key);

        // Get or create health status for this resource
        let health_status = self.health_statuses
            .entry(resource_key.to_string())
            .or_insert_with(|| HealthStatus {
                healthy: true,
                status_message: "Healthy".to_string(),
                last_check: Instant::now(),
                consecutive_failures: 0,
            });

        // Update last check time
        health_status.last_check = Instant::now();

        if transition_result.is_success() {
            // Successful transition
            if !health_status.healthy {
                println!("Resource {} recovered to healthy state", resource_key);
            }
            health_status.healthy = true;
            health_status.consecutive_failures = 0;
            health_status.status_message = "Healthy".to_string();
        } else {
            // Failed transition
            health_status.consecutive_failures += 1;
            health_status.status_message = transition_result.message.clone();

            if health_status.consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                if health_status.healthy {
                    warn!(
                        "Resource {} marked as unhealthy after {} consecutive failures",
                        resource_key, health_status.consecutive_failures
                    );
                }
                health_status.healthy = false;
            }
        }

        debug!(
            "Health status updated for {}: healthy={}, failures={}, message='{}'",
            resource_key,
            health_status.healthy,
            health_status.consecutive_failures,
            health_status.status_message
        );
    }

    /// Check if a resource is healthy
    pub fn is_resource_healthy(&self, resource_key: &str) -> bool {
        self.health_statuses
            .get(resource_key)
            .map(|status| status.healthy)
            .unwrap_or(true) // Default to healthy if not tracked
    }

    /// Get health status for a resource
    pub fn get_health_status(&self, resource_key: &str) -> Option<&HealthStatus> {
        self.health_statuses.get(resource_key)
    }

    /// Get mutable health status for a resource (for external updates)
    pub fn get_health_status_mut(&mut self, resource_key: &str) -> Option<&mut HealthStatus> {
        self.health_statuses.get_mut(resource_key)
    }

    /// Initialize health tracking for a new resource
    pub fn initialize_health_tracking(&mut self, resource_key: String) {
        self.health_statuses.insert(resource_key.clone(), HealthStatus {
            healthy: true,
            status_message: "Healthy".to_string(),
            last_check: Instant::now(),
            consecutive_failures: 0,
        });
        debug!("Initialized health tracking for resource: {}", resource_key);
    }
}

impl Default for HealthManager {
    fn default() -> Self {
        Self::new()
    }
}
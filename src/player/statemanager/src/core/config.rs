/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Configuration constants and settings for the State Machine

use tokio::time::Duration;

// ========================================
// CONSTANTS AND CONFIGURATION
// ========================================

/// Default backoff duration for CrashLoopBackOff states
pub const BACKOFF_DURATION_SECS: u64 = 30;

/// Maximum consecutive failures before marking resource as unhealthy
pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;

/// Cache warming configuration
pub const ACTIVE_RESOURCE_PREFIXES: &[&str] = &["Scenario::", "Package::", "Model::"];

/// State transition timeouts
pub const TRANSITION_TIMEOUT_SECS: u64 = 300;

/// Health check interval
pub const HEALTH_CHECK_INTERVAL_SECS: u64 = 60;

/// Maximum age for state records before cleanup (in seconds)
pub const MAX_STATE_AGE_SECS: u64 = 86400; // 24 hours

/// Get backoff duration as Duration
pub fn get_backoff_duration() -> Duration {
    Duration::from_secs(BACKOFF_DURATION_SECS)
}

/// Get health check interval as Duration
pub fn get_health_check_interval() -> Duration {
    Duration::from_secs(HEALTH_CHECK_INTERVAL_SECS)
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Policy caching module for PolicyManager
//!
//! This module provides caching functionality to reduce etcd calls when
//! fetching policies. Policies are cached with a configurable TTL.

use common::spec::artifact::Policy;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

const ETCD_POLICY_PREFIX: &str = "Policy";

/// Cache TTL for policies (seconds)
const POLICY_CACHE_TTL_SECS: u64 = 10;

/// Cached policy with timestamp
struct CachedPolicy {
    policy: Policy,
    cached_at: Instant,
}

lazy_static::lazy_static! {
    /// Policy cache to reduce etcd calls
    static ref POLICY_CACHE: RwLock<HashMap<String, CachedPolicy>> = RwLock::new(HashMap::new());
}

/// Get policy from cache or fetch from etcd
///
/// # Arguments
/// * `policy_name` - Name of the policy to fetch
///
/// # Returns
/// * `Some(Policy)` if found and valid
/// * `None` if not found or parse error
pub async fn get_policy_cached(policy_name: &str) -> Option<Policy> {
    // Try to get from cache first
    {
        let cache = POLICY_CACHE.read().unwrap();
        if let Some(cached) = cache.get(policy_name) {
            if cached.cached_at.elapsed() < Duration::from_secs(POLICY_CACHE_TTL_SECS) {
                return Some(cached.policy.clone());
            }
        }
    }

    // Cache miss or expired - fetch from etcd
    let etcd_key = format!("{}/{}", ETCD_POLICY_PREFIX, policy_name);
    let policy_str = common::etcd::get(&etcd_key).await.ok()?;
    let policy: Policy = serde_yaml::from_str(&policy_str).ok()?;

    // Store in cache
    {
        let mut cache = POLICY_CACHE.write().unwrap();
        cache.insert(
            policy_name.to_string(),
            CachedPolicy {
                policy: policy.clone(),
                cached_at: Instant::now(),
            },
        );
    }

    Some(policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_policy_cached_not_found() {
        // Policy that doesn't exist should return None
        let result = get_policy_cached("nonexistent_policy_xyz").await;
        assert!(result.is_none());
    }
}

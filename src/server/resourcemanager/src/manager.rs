/*
* SPDX-FileCopyrightText: Copyright 2026 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/

//! Resource state store and validation / reconciliation logic.
//!
//! ResourceManager owns the *desired* resource state. The *actual* state is
//! reported back by the NodeAgent (through the ActionController) after a
//! runtime update. Drift is detected by comparing desired and actual state.

use std::collections::HashMap;
use std::sync::Mutex;

use common::resourcemanager::ResourceSyncState;

/// Per-workload resource state tracked by the ResourceManager.
#[derive(Debug, Clone, Default)]
pub struct ResourceState {
    /// Desired CPU limit in millicores (1000 == 1 core).
    pub desired_cpu: u32,
    pub desired_memory: u64,
    /// Actual CPU limit in millicores (1000 == 1 core).
    pub actual_cpu: u32,
    pub actual_memory: u64,
    pub sync_state: i32,
}

/// Outcome of a resource availability validation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub allowed: bool,
    pub reason: String,
    pub available_cpu: u32,
    pub available_memory: u64,
}

/// ResourceManager state store.
///
/// Node capacity is discovered once at startup. `states` holds the desired /
/// actual resource state for every workload known to the scaling subsystem.
pub struct ResourceManagerManager {
    total_cpu: u32,
    total_memory_mib: u64,
    states: Mutex<HashMap<String, ResourceState>>,
}

impl ResourceManagerManager {
    /// Create a manager using the host capacity discovered via `sysinfo`.
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let total_cpu = num_cpus_available();
        // sysinfo reports bytes; convert to MiB.
        let total_memory_mib = sys.total_memory() / (1024 * 1024);
        Self::with_capacity(total_cpu, total_memory_mib)
    }

    /// Create a manager with explicit capacity (used by tests).
    pub fn with_capacity(total_cpu: u32, total_memory_mib: u64) -> Self {
        Self {
            total_cpu,
            total_memory_mib,
            states: Mutex::new(HashMap::new()),
        }
    }

    /// Sum of the desired resources across all workloads except `exclude`.
    ///
    /// The workload being updated is excluded so that its *new* request is
    /// validated against the capacity not already committed to *other*
    /// workloads (a resize must not double-count the workload itself).
    fn allocated_excluding(&self, exclude: &str) -> (u32, u64) {
        let states = self.states.lock().unwrap();
        let mut cpu = 0u32;
        let mut mem = 0u64;
        for (id, st) in states.iter() {
            if id == exclude {
                continue;
            }
            cpu = cpu.saturating_add(st.desired_cpu);
            mem = mem.saturating_add(st.desired_memory);
        }
        (cpu, mem)
    }

    /// Current desired resource state for `workload_id` (0 if unknown).
    fn current_desired(&self, workload_id: &str) -> (u32, u64) {
        self.states
            .lock()
            .unwrap()
            .get(workload_id)
            .map(|s| (s.desired_cpu, s.desired_memory))
            .unwrap_or((0, 0))
    }

    /// Validate whether `requested` CPU / memory can be allocated for
    /// `workload_id`.
    ///
    /// Scale direction is derived per-resource from the current desired state,
    /// *not* from a caller-supplied flag. Each resource is capacity-checked
    /// **only when it increases** above its current desired value; a resource
    /// that stays the same or shrinks can never exceed capacity and is never
    /// rejected. This handles mixed requests (e.g. CPU up while memory down)
    /// correctly and prevents bypassing the check by mislabelling the request.
    pub fn validate(
        &self,
        workload_id: &str,
        requested_cpu: u32,
        requested_memory: u64,
    ) -> ValidationResult {
        let (alloc_cpu, alloc_mem) = self.allocated_excluding(workload_id);
        let available_cpu = self.total_cpu.saturating_sub(alloc_cpu);
        let available_memory = self.total_memory_mib.saturating_sub(alloc_mem);

        // Per-resource scale direction from the recorded desired state.
        let (cur_cpu, cur_mem) = self.current_desired(workload_id);
        let cpu_increases = requested_cpu > cur_cpu;
        let mem_increases = requested_memory > cur_mem;

        // Only a resource that grows is checked against remaining capacity.
        if cpu_increases && requested_cpu > available_cpu {
            return ValidationResult {
                allowed: false,
                reason: format!(
                    "insufficient CPU: requested {} millicores, available {} millicores",
                    requested_cpu, available_cpu
                ),
                available_cpu,
                available_memory,
            };
        }
        if mem_increases && requested_memory > available_memory {
            return ValidationResult {
                allowed: false,
                reason: format!(
                    "insufficient memory: requested {} MiB, available {} MiB",
                    requested_memory, available_memory
                ),
                available_cpu,
                available_memory,
            };
        }
        ValidationResult {
            allowed: true,
            reason: "ok".to_string(),
            available_cpu,
            available_memory,
        }
    }

    /// Record the desired resource state for a workload and mark it PENDING.
    pub fn update_desired(&self, workload_id: &str, cpu: u32, memory: u64) -> i32 {
        let mut states = self.states.lock().unwrap();
        let st = states.entry(workload_id.to_string()).or_default();
        st.desired_cpu = cpu;
        st.desired_memory = memory;
        st.sync_state = ResourceSyncState::SyncPending as i32;
        st.sync_state
    }

    /// Report the actual runtime state and reconcile it against the desired
    /// state. Returns `(synchronized, sync_state, drift_detected)`.
    pub fn report_actual(
        &self,
        workload_id: &str,
        cpu: u32,
        memory: u64,
        update_success: bool,
    ) -> (bool, i32, bool) {
        let mut states = self.states.lock().unwrap();
        let st = states.entry(workload_id.to_string()).or_default();
        st.actual_cpu = cpu;
        st.actual_memory = memory;

        if !update_success {
            st.sync_state = ResourceSyncState::SyncFailed as i32;
            return (false, st.sync_state, false);
        }

        let matched = st.actual_cpu == st.desired_cpu && st.actual_memory == st.desired_memory;
        if matched {
            st.sync_state = ResourceSyncState::SyncSynchronized as i32;
            (true, st.sync_state, false)
        } else {
            st.sync_state = ResourceSyncState::SyncDriftDetected as i32;
            (false, st.sync_state, true)
        }
    }

    /// Look up the current state for a workload.
    pub fn get_state(&self, workload_id: &str) -> Option<ResourceState> {
        self.states.lock().unwrap().get(workload_id).cloned()
    }
}

impl Default for ResourceManagerManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Number of logical CPUs available to the host, expressed in millicores
/// (1 core == 1000 millicores) to match the scaling pipeline's CPU unit.
fn num_cpus_available() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32 * 1000)
        .unwrap_or(1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_allows_within_capacity() {
        // 8 cores == 8000 millicores.
        let m = ResourceManagerManager::with_capacity(8000, 8192);
        let r = m.validate("w1", 4000, 2048);
        assert!(r.allowed, "reason: {}", r.reason);
        assert_eq!(r.available_cpu, 8000);
        assert_eq!(r.available_memory, 8192);
    }

    #[test]
    fn validate_rejects_over_cpu() {
        let m = ResourceManagerManager::with_capacity(4000, 8192);
        let r = m.validate("w1", 8000, 1024);
        assert!(!r.allowed);
        assert!(r.reason.contains("CPU"));
    }

    #[test]
    fn validate_rejects_over_memory() {
        let m = ResourceManagerManager::with_capacity(8000, 2048);
        let r = m.validate("w1", 2000, 4096);
        assert!(!r.allowed);
        assert!(r.reason.contains("memory"));
    }

    #[test]
    fn validate_accounts_for_other_workloads() {
        let m = ResourceManagerManager::with_capacity(8000, 8192);
        m.update_desired("w1", 6000, 6144);
        // Only 2 cores / 2048 MiB left for a different workload.
        let r = m.validate("w2", 4000, 1024);
        assert!(!r.allowed);
        let ok = m.validate("w2", 2000, 2048);
        assert!(ok.allowed, "reason: {}", ok.reason);
    }

    #[test]
    fn resize_excludes_self_from_allocation() {
        let m = ResourceManagerManager::with_capacity(8000, 8192);
        m.update_desired("w1", 4000, 4096);
        // Growing the same workload to the full node must be allowed.
        let r = m.validate("w1", 8000, 8192);
        assert!(r.allowed, "reason: {}", r.reason);
    }

    #[test]
    fn validate_allows_sub_core_millicores() {
        // Fractional-core requests (e.g. 500m) must be accepted.
        let m = ResourceManagerManager::with_capacity(2000, 4096);
        let r = m.validate("w1", 500, 256);
        assert!(r.allowed, "reason: {}", r.reason);
    }

    #[test]
    fn validate_allows_scale_down_without_capacity_check() {
        // Node is fully committed by other workloads, but shrinking w1 must
        // still be allowed because it never increases usage.
        let m = ResourceManagerManager::with_capacity(4000, 4096);
        m.update_desired("other", 4000, 4096); // node fully allocated
        m.update_desired("w1", 3000, 3072);
        let r = m.validate("w1", 1000, 1024); // shrink
        assert!(r.allowed, "scale-down must be allowed: {}", r.reason);
    }

    #[test]
    fn validate_rejects_oversized_request_regardless_of_direction() {
        // A caller cannot bypass the capacity check: even coming from a small
        // current desired, an oversized request is validated as an increase.
        let m = ResourceManagerManager::with_capacity(4000, 4096);
        m.update_desired("w1", 1000, 1024);
        let r = m.validate("w1", 100_000, 100_000);
        assert!(!r.allowed, "oversized request must be rejected");
        assert!(r.reason.contains("CPU") || r.reason.contains("memory"));
    }

    #[test]
    fn validate_mixed_shrinking_resource_not_rejected() {
        // CPU grows while memory shrinks. The shrinking resource must never be
        // capacity-checked, even if the reduced value still exceeds the (small)
        // remaining memory left by other workloads.
        let m = ResourceManagerManager::with_capacity(8000, 8192);
        m.update_desired("other", 0, 7168); // available memory == 1024 MiB
        m.update_desired("w1", 1000, 8192); // w1 currently holds large memory
        // CPU 1000 -> 2000 (up), memory 8192 -> 2048 (down, but 2048 > 1024).
        let r = m.validate("w1", 2000, 2048);
        assert!(
            r.allowed,
            "shrinking memory must not be rejected: {}",
            r.reason
        );
    }

    #[test]
    fn validate_mixed_growing_resource_still_checked() {
        // Symmetric case: memory grows beyond capacity while CPU shrinks.
        // The growing resource must still be rejected.
        let m = ResourceManagerManager::with_capacity(8000, 2048);
        m.update_desired("w1", 4000, 1024);
        // CPU 4000 -> 1000 (down), memory 1024 -> 4096 (up, over capacity).
        let r = m.validate("w1", 1000, 4096);
        assert!(!r.allowed, "growing memory over capacity must be rejected");
        assert!(r.reason.contains("memory"));
    }

    #[test]
    fn report_actual_detects_synchronized() {
        let m = ResourceManagerManager::with_capacity(8000, 8192);
        m.update_desired("w1", 4000, 2048);
        let (sync, state, drift) = m.report_actual("w1", 4000, 2048, true);
        assert!(sync);
        assert!(!drift);
        assert_eq!(state, ResourceSyncState::SyncSynchronized as i32);
    }

    #[test]
    fn report_actual_detects_drift() {
        let m = ResourceManagerManager::with_capacity(8000, 8192);
        m.update_desired("w1", 4000, 2048);
        let (sync, state, drift) = m.report_actual("w1", 2000, 2048, true);
        assert!(!sync);
        assert!(drift);
        assert_eq!(state, ResourceSyncState::SyncDriftDetected as i32);
    }

    #[test]
    fn report_actual_marks_failed_on_update_failure() {
        let m = ResourceManagerManager::with_capacity(8000, 8192);
        m.update_desired("w1", 4000, 2048);
        let (sync, state, _drift) = m.report_actual("w1", 0, 0, false);
        assert!(!sync);
        assert_eq!(state, ResourceSyncState::SyncFailed as i32);
    }
}

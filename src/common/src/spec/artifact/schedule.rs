/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
use super::Artifact;
use super::Schedule;

impl Artifact for Schedule {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Schedule {
    pub fn get_spec(&self) -> &Option<Vec<ScheduleSpec>> {
        &self.spec
    }

    /// Get the temporal class of this schedule
    /// Returns PERIODIC as default if not specified
    pub fn get_temporal_class(&self) -> TemporalClass {
        self.temporal_class
            .clone()
            .unwrap_or(TemporalClass::PERIODIC)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ScheduleSpec {
    pub name: String,
    pub priority: i32,
    pub policy: SchedPolicy,
    pub cpu_affinity: u64,
    pub period: i32,
    pub release_time: i32,
    pub runtime: i32,
    pub deadline: i32,
    pub node_id: String,
    pub max_dmiss: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum SchedPolicy {
    // SCHED_NORMAL
    NORMAL = 0,
    // SCHED_FIFO
    FIFO = 1,
    // SCHED_RR
    RR = 2,
}

/// Temporal class for workload classification (DDR-001/DDR-007)
/// Determines the scheduling mechanism:
/// - PERIODIC → L1 → Time-Triggered (TT)
/// - SPORADIC → L2 → Constant Bandwidth Server (CBS)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub enum TemporalClass {
    #[default]
    PERIODIC = 0,
    SPORADIC = 1,
}

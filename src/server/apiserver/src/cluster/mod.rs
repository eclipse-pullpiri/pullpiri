/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node registry and cluster management

pub mod registry;

// Re-export clustering structures from common
pub use common::spec::artifact::node::{
    ClusterConfig, ClusterTopology, NodeInfo, NodeLifecycleStatus, NodeResources, NodeRole,
    TopologyType,
};
pub use registry::NodeRegistry;

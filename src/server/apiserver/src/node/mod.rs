/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node Management Module
//! 
//! This module provides clustering functionality for the PICCOLO API Server.
//! It includes node registration, status monitoring, and cluster topology management.

pub mod manager;
pub mod registry;
pub mod status;

pub use manager::NodeManager;
pub use registry::NodeRegistry;
pub use status::{NodeStatusManager, NodeMetrics, ClusterHealthSummary};
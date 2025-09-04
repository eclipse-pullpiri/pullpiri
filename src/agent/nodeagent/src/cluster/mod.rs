/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! NodeAgent Clustering Module
//!
//! This module provides clustering functionality for the NodeAgent including:
//! - Connection to master node
//! - Node registration and authentication
//! - Heartbeat and status reporting
//! - System readiness checks
//! - Connection recovery and reconnection

pub mod client;

pub use client::{ClusterClient, ClusterConfig};

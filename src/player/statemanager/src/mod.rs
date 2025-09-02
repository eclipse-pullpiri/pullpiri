/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! StateManager Module Exports
//!
//! This module provides the public interface for the StateManager component

// Core state management functionality
pub mod core;

// Data persistence and storage
pub mod storage;

// Health monitoring and validation
pub mod monitoring;

// Utility functions
pub mod utils;

// External communication interfaces
pub mod communication;

// State machine implementation
pub mod state_machine;

// Re-export commonly used items
pub use core::{manager::StateManagerManager, types::*, config::*};
pub use state_machine::StateMachine;
pub use storage::etcd_state;
pub use monitoring::{health::HealthManager, validation::StateValidator};
pub use utils::utility::StateUtilities;
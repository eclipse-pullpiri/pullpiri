/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Core state management functionality

pub mod config;
pub mod manager;
pub mod types;

pub use config::*;
pub use manager::StateManagerManager;
pub use types::*;

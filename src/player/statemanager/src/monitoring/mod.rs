/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Health monitoring and validation functionality

pub mod health;
pub mod validation;

pub use health::HealthManager;
pub use validation::StateValidator;

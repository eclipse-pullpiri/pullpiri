/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC sender modules for FilterGateway
//!
//! This module contains client implementations for communicating with other services:
//! - StateManager: For reporting policy decisions and security enforcement results
//! - ActionController: For sending action requests based on policy decisions

pub mod actioncontroller;
pub mod statemanager;

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! The Apiserver provides internal/external APIs for Piccolo operations
//! and performs registration and preparation tasks for scenarios and other
//! artifacts.
//!
//! * Open a REST API to communicate with Piccolo Cloud or receive artifacts
//!   directly.
//! * Appropriately parse the received string-type artifacts so that they can
//!   be used within Piccolo.
//! * The parsing results are stored in etcd and passed to filtergateway so
//!   that a filter can be created.

mod artifact;
mod bluechi;
mod grpc;
mod manager;
mod route;

/// Main function of Piccolo API Server
#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() {
    manager::initialize().await
}

//UNIT TEST CASES
//main() itself is not directly testable in typical unit test form because it's an entry point with #[tokio::main]

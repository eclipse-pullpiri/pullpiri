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
#[tokio::main]
async fn main() {
    manager::initialize().await
}

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_manager_initialize_runs_briefly() {
        tokio::select! {
            _ = manager::initialize() => {
                // initialize() completed (unlikely)
            }
            _ = sleep(Duration::from_millis(200)) => {
                // We let it run for 200ms and then we consider test successful
            }
        }

        // Test passes if initialize() starts cleanly and doesn't panic immediately
    }
}

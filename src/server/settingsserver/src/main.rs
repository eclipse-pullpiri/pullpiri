/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! The SettingsServer provides configuration management APIs for Pullpiri monitoring functionality
//!
//! * Provides REST APIs for monitoring settings configuration
//! * Integrates with the existing monitoring infrastructure
//! * Supports Web GUI Tool backend requirements

mod manager;
mod route;

/// Main function of Pullpiri Settings Server
#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() {
    println!("Starting SettingsServer...");
    manager::initialize().await
}

//UNIT TEST CASES
//main() itself is not directly testable in typical unit test form because it's an entry point with #[tokio::main]
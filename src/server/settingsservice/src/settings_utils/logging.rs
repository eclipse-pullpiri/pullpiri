// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Logging utilities

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize logging with the specified level
pub fn init_logging(level: &str) -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("settingsservice={},warn", level)));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    Ok(())
}

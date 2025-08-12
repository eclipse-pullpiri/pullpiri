// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Error handling utilities

use thiserror::Error;

/// Main error types for the Settings Service
#[derive(Error, Debug)]
pub enum SettingsError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("History error: {0}")]
    History(String),

    #[error("Metrics error: {0}")]
    Metrics(String),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("API error: {0}")]
    Api(String),

    #[error("CLI error: {0}")]
    Cli(String),

    #[error("System error: {0}")]
    System(String),
}

/// Storage-specific errors
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("ETCD connection failed: {0}")]
    ConnectionFailed(String),

    #[error("ETCD operation failed: {0}")]
    OperationFailed(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// API-specific errors
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

/// Validation-specific errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Schema validation failed: {0}")]
    SchemaError(String),

    #[error("Required field missing: {0}")]
    MissingField(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),
}

pub type Result<T> = std::result::Result<T, SettingsError>;
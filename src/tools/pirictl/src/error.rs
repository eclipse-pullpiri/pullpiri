/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! Error handling for pirictl

use std::fmt;

/// Custom error type for CLI operations
#[derive(Debug)]
pub enum CliError {
    /// HTTP client errors
    Http(reqwest::Error),
    /// JSON parsing errors
    Json(serde_json::Error),
    /// IO errors
    Io(std::io::Error),
    /// Custom error messages
    Custom(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Http(e) => write!(f, "HTTP error: {}", e),
            CliError::Json(e) => write!(f, "JSON error: {}", e),
            CliError::Io(e) => write!(f, "IO error: {}", e),
            CliError::Custom(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for CliError {}

impl From<reqwest::Error> for CliError {
    fn from(err: reqwest::Error) -> Self {
        CliError::Http(err)
    }
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::Json(err)
    }
}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError::Io(err)
    }
}

/// Result type for CLI operations
pub type Result<T> = std::result::Result<T, CliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_error_display_custom() {
        let err = CliError::Custom("test error message".to_string());
        assert_eq!(format!("{}", err), "Error: test error message");
    }

    #[test]
    fn test_cli_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = CliError::Io(io_err);
        let display = format!("{}", err);
        assert!(display.starts_with("IO error:"));
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_cli_error_display_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let err = CliError::Json(json_err);
        let display = format!("{}", err);
        assert!(display.starts_with("JSON error:"));
    }

    #[test]
    fn test_cli_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let cli_err: CliError = io_err.into();
        match cli_err {
            CliError::Io(_) => (),
            _ => panic!("Expected CliError::Io"),
        }
    }

    #[test]
    fn test_cli_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid}").unwrap_err();
        let cli_err: CliError = json_err.into();
        match cli_err {
            CliError::Json(_) => (),
            _ => panic!("Expected CliError::Json"),
        }
    }

    #[test]
    fn test_cli_error_debug() {
        let err = CliError::Custom("debug test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Custom"));
        assert!(debug_str.contains("debug test"));
    }

    #[test]
    fn test_cli_error_is_error_trait() {
        let err: Box<dyn std::error::Error> = Box::new(CliError::Custom("trait test".to_string()));
        assert!(err.to_string().contains("trait test"));
    }
}

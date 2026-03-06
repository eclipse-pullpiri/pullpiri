/*
* SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
* SPDX-License-Identifier: Apache-2.0
*/
//! REST API client for SettingsService

use crate::error::{CliError, Result};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// HTTP client for communicating with SettingsService
pub struct SettingsClient {
    client: Client,
    base_url: String,
}

impl SettingsClient {
    /// Create a new SettingsClient
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the SettingsService (e.g., "http://localhost:47098")
    /// * `timeout` - Request timeout in seconds
    pub fn new(base_url: &str, timeout: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .map_err(CliError::Http)?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Make a GET request to the specified endpoint
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint (e.g., "/api/v1/metrics")
    pub async fn get(&self, endpoint: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(CliError::Custom(format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let json: Value = response.json().await?;
        Ok(json)
    }

    /// Make a POST request to the specified endpoint
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint
    /// * `body` - Request body as JSON
    pub async fn post(&self, endpoint: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self.client.post(&url).json(body).send().await?;

        if !response.status().is_success() {
            return Err(CliError::Custom(format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let json: Value = response.json().await?;
        Ok(json)
    }

    /// Make a PUT request to the specified endpoint
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint
    /// * `body` - Request body as JSON
    pub async fn put(&self, endpoint: &str, body: &Value) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self.client.put(&url).json(body).send().await?;

        if !response.status().is_success() {
            return Err(CliError::Custom(format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let json: Value = response.json().await?;
        Ok(json)
    }

    /// Make a DELETE request to the specified endpoint
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint
    pub async fn delete(&self, endpoint: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self.client.delete(&url).send().await?;

        if !response.status().is_success() {
            return Err(CliError::Custom(format!(
                "Request failed with status: {}",
                response.status()
            )));
        }

        let json: Value = response.json().await?;
        Ok(json)
    }

    /// Check if the SettingsService is reachable
    pub async fn health_check(&self) -> Result<bool> {
        match self.get("/api/v1/system/health").await {
            Ok(_) => Ok(true),
            Err(_) => {
                // Try alternative health check endpoint
                match self.get("/api/v1/health").await {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
        }
    }

    /// Check if the API Server is reachable
    pub async fn api_health_check(&self) -> Result<bool> {
        match self.get("/api/health").await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Apply YAML artifact (POST with text/plain content)
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint (e.g., "/api/v1/yaml")
    /// * `yaml_content` - YAML content as string
    pub async fn post_yaml(&self, endpoint: &str, yaml_content: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "text/plain")
            .body(yaml_content.to_owned())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Custom(format!(
                "Request failed with status: {} - {}",
                status, error_text
            )));
        }

        let json: Value = response.json().await?;
        Ok(json)
    }

    /// Withdraw YAML artifact (DELETE with text/plain content)
    ///
    /// # Arguments
    /// * `endpoint` - API endpoint (e.g., "/api/v1/yaml")
    /// * `yaml_content` - YAML content as string
    pub async fn delete_yaml(&self, endpoint: &str, yaml_content: &str) -> Result<Value> {
        let url = format!("{}{}", self.base_url, endpoint);
        let response = self
            .client
            .delete(&url)
            .header("Content-Type", "text/plain")
            .body(yaml_content.to_owned())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(CliError::Custom(format!(
                "Request failed with status: {} - {}",
                status, error_text
            )));
        }

        let json: Value = response.json().await?;
        Ok(json)
    }
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Handler functions for Settings Server REST API

use axum::{
    extract::Path,
    response::Response,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

/// Monitoring settings configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSettings {
    /// Unique identifier for the settings
    pub id: String,
    /// Monitoring interval in seconds
    pub monitoring_interval: u64,
    /// Container monitoring enabled flag
    pub container_monitoring_enabled: bool,
    /// Alert threshold for resource usage (0-100)
    pub resource_alert_threshold: u8,
    /// Data retention period in days
    pub data_retention_days: u32,
    /// Enable detailed logging
    pub detailed_logging: bool,
}

/// Default monitoring settings
impl Default for MonitoringSettings {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            monitoring_interval: 30,
            container_monitoring_enabled: true,
            resource_alert_threshold: 80,
            data_retention_days: 30,
            detailed_logging: false,
        }
    }
}

/// Create router for monitoring settings API endpoints
pub fn router() -> Router {
    Router::new()
        .route("/api/monitoring/settings", get(get_all_monitoring_settings))
        .route("/api/monitoring/settings", post(create_monitoring_settings))
        .route("/api/monitoring/settings/:id", get(get_monitoring_settings))
        .route("/api/monitoring/settings/:id", put(update_monitoring_settings))
        .route("/api/monitoring/settings/:id", delete(delete_monitoring_settings))
        .route("/api/monitoring/status", get(get_monitoring_status))
}

/// Get all monitoring settings
async fn get_all_monitoring_settings() -> Response {
    let result = crate::manager::get_all_monitoring_settings().await;
    super::json_response(result)
}

/// Get specific monitoring settings by ID
async fn get_monitoring_settings(Path(id): Path<String>) -> Response {
    let result = crate::manager::get_monitoring_settings(&id).await;
    super::json_response(result)
}

/// Create new monitoring settings
async fn create_monitoring_settings(Json(settings): Json<MonitoringSettings>) -> Response {
    let result = crate::manager::create_monitoring_settings(settings).await;
    super::status(result)
}

/// Update existing monitoring settings
async fn update_monitoring_settings(
    Path(id): Path<String>,
    Json(settings): Json<MonitoringSettings>,
) -> Response {
    let result = crate::manager::update_monitoring_settings(&id, settings).await;
    super::status(result)
}

/// Delete monitoring settings
async fn delete_monitoring_settings(Path(id): Path<String>) -> Response {
    let result = crate::manager::delete_monitoring_settings(&id).await;
    super::status(result)
}

/// Get monitoring system status
async fn get_monitoring_status() -> Response {
    let result = crate::manager::get_monitoring_status().await;
    super::json_response(result)
}

//UNIT TEST CASES
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    /// Test MonitoringSettings default values
    #[tokio::test]
    async fn test_monitoring_settings_default() {
        let settings = MonitoringSettings::default();
        assert_eq!(settings.id, "default");
        assert_eq!(settings.monitoring_interval, 30);
        assert!(settings.container_monitoring_enabled);
        assert_eq!(settings.resource_alert_threshold, 80);
        assert_eq!(settings.data_retention_days, 30);
        assert!(!settings.detailed_logging);
    }

    /// Test MonitoringSettings serialization
    #[tokio::test]
    async fn test_monitoring_settings_serialization() {
        let settings = MonitoringSettings::default();
        let json = serde_json::to_string(&settings);
        assert!(json.is_ok());
        
        let json_str = json.unwrap();
        let deserialized: Result<MonitoringSettings, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok());
        
        let deserialized_settings = deserialized.unwrap();
        assert_eq!(settings.id, deserialized_settings.id);
    }

    /// Test router configuration
    #[tokio::test]
    async fn test_router_endpoints() {
        let app = router();
        
        // Test GET /api/monitoring/settings (should be 500 due to mock implementation)
        let req = Request::builder()
            .method("GET")
            .uri("/api/monitoring/settings")
            .body(Body::empty())
            .unwrap();
        
        let response = app.clone().oneshot(req).await.unwrap();
        // Should return 200 since the implementation works correctly
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test valid POST request format
    #[tokio::test]
    async fn test_post_monitoring_settings() {
        let app = router();
        let mut settings = MonitoringSettings::default();
        settings.id = "test_post".to_string(); // Use unique ID
        let json_body = serde_json::to_string(&settings).unwrap();
        
        let req = Request::builder()
            .method("POST")
            .uri("/api/monitoring/settings")
            .header("Content-Type", "application/json")
            .body(Body::from(json_body))
            .unwrap();
        
        let response = app.oneshot(req).await.unwrap();
        // Should return 200 since the implementation works correctly
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test invalid method on endpoints
    #[tokio::test]
    async fn test_invalid_method() {
        let app = router();
        
        let req = Request::builder()
            .method("PATCH")  // Not supported
            .uri("/api/monitoring/settings")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    /// Test GET specific settings endpoint
    #[tokio::test]
    async fn test_get_specific_settings() {
        let app = router();
        
        let req = Request::builder()
            .method("GET")
            .uri("/api/monitoring/settings/default")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(req).await.unwrap();
        // Should return 200 since the implementation works correctly
        assert_eq!(response.status(), StatusCode::OK);
    }

    /// Test monitoring status endpoint
    #[tokio::test]
    async fn test_monitoring_status() {
        let app = router();
        
        let req = Request::builder()
            .method("GET")
            .uri("/api/monitoring/status")
            .body(Body::empty())
            .unwrap();
        
        let response = app.oneshot(req).await.unwrap();
        // Should return 200 since the implementation works correctly
        assert_eq!(response.status(), StatusCode::OK);
    }
}
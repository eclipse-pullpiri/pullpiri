// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! REST API server module

use crate::settings_config::{Config, ConfigManager, ConfigSummary, ValidationResult};
use crate::settings_history::{HistoryEntry, HistoryManager, ChangeAction};
use crate::settings_monitoring::{MonitoringManager, Metric, MetricsFilter, FilterSummary};
use crate::settings_utils::error::{SettingsError, ApiError};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, error, debug};

/// API server state
#[derive(Clone)]
pub struct ApiState {
    pub config_manager: Arc<RwLock<ConfigManager>>,
    pub history_manager: Arc<RwLock<HistoryManager>>,
    pub monitoring_manager: Arc<RwLock<MonitoringManager>>,
}

/// Query parameters for metrics API
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub component: Option<String>,
    pub metric_type: Option<String>,
    pub filter_id: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// Query parameters for history API
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
}

/// Query parameters for config listing
#[derive(Debug, Deserialize)]
pub struct ConfigQuery {
    pub prefix: Option<String>,
}

/// Request body for config creation/updates
#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    pub content: Value,
    pub schema_type: String,
    pub author: String,
    pub comment: Option<String>,
}

/// Response for successful operations
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub details: Option<Value>,
}

/// API server
pub struct ApiServer {
    bind_address: String,
    bind_port: u16,
    state: ApiState,
}

impl ApiServer {
    /// Create a new API server
    pub async fn new(
        bind_address: String,
        bind_port: u16,
        config_manager: Arc<RwLock<ConfigManager>>,
        history_manager: Arc<RwLock<HistoryManager>>,
        monitoring_manager: Arc<RwLock<MonitoringManager>>,
    ) -> Result<Self, SettingsError> {
        let state = ApiState {
            config_manager,
            history_manager,
            monitoring_manager,
        };

        Ok(Self {
            bind_address,
            bind_port,
            state,
        })
    }

    /// Start the API server
    pub async fn start(self) -> Result<(), SettingsError> {
        let app = self.create_router();

        let addr = format!("{}:{}", self.bind_address, self.bind_port);
        info!("Starting API server on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| SettingsError::Api(format!("Failed to bind to {}: {}", addr, e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| SettingsError::Api(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// Create the router with all endpoints
    fn create_router(&self) -> Router {
        Router::new()
            // Metrics endpoints
            .route("/api/v1/metrics", get(get_metrics))
            .route("/api/v1/metrics/:id", get(get_metric_by_id))
            .route("/api/v1/metrics/component/:component", get(get_metrics_by_component))
            .route("/api/v1/metrics/type/:metric_type", get(get_metrics_by_type))
            .route("/api/v1/metrics/filters", get(get_filters))
            .route("/api/v1/metrics/filters", post(create_filter))
            .route("/api/v1/metrics/filters/:id", get(get_filter))
            .route("/api/v1/metrics/filters/:id", put(update_filter))
            .route("/api/v1/metrics/filters/:id", delete(delete_filter))
            
            // Configuration endpoints
            .route("/api/v1/settings", get(list_configs))
            .route("/api/v1/settings/:path", get(get_config))
            .route("/api/v1/settings/:path", post(create_config))
            .route("/api/v1/settings/:path", put(update_config))
            .route("/api/v1/settings/:path", delete(delete_config))
            .route("/api/v1/settings/validate", post(validate_config))
            .route("/api/v1/settings/schemas/:schema_type", get(get_schema))
            .route("/api/v1/settings/schemas/:schema_type", put(save_schema))
            
            // History endpoints
            .route("/api/v1/history/:path", get(get_history))
            .route("/api/v1/history/:path/version/:version", get(get_version))
            .route("/api/v1/history/:path/rollback/:version", post(rollback_to_version))
            .route("/api/v1/history/:path/diff", get(diff_versions))
            
            // System endpoints
            .route("/api/v1/system/status", get(get_system_status))
            .route("/api/v1/system/health", get(health_check))
            
            .layer(CorsLayer::permissive())
            .with_state(self.state.clone())
    }
}

// Metrics API handlers

async fn get_metrics(
    Query(query): Query<MetricsQuery>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<Metric>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics with query: {:?}", query);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    let filter = if let Some(filter_id) = query.filter_id {
        match monitoring_manager.get_filter(&filter_id).await {
            Ok(filter) => Some(filter),
            Err(_) => return Err(not_found_error("Filter not found")),
        }
    } else if query.component.is_some() || query.metric_type.is_some() {
        Some(MetricsFilter {
            id: "temp".to_string(),
            name: "Temporary filter".to_string(),
            enabled: true,
            components: query.component.map(|c| vec![c]),
            metric_types: query.metric_type.map(|t| vec![t]),
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: query.page_size.map(|ps| ps as usize),
        })
    } else {
        None
    };

    match monitoring_manager.get_metrics(filter.as_ref()).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => Err(internal_error(&format!("Failed to get metrics: {}", e))),
    }
}

async fn get_metric_by_id(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Metric>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/{}", id);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_metric_by_id(&id).await {
        Ok(Some(metric)) => Ok(Json(metric)),
        Ok(None) => Err(not_found_error("Metric not found")),
        Err(e) => Err(internal_error(&format!("Failed to get metric: {}", e))),
    }
}

async fn get_metrics_by_component(
    Path(component): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<Metric>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/component/{}", component);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_metrics_by_component(&component).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => Err(internal_error(&format!("Failed to get metrics: {}", e))),
    }
}

async fn get_metrics_by_type(
    Path(metric_type): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<Metric>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/type/{}", metric_type);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_metrics_by_type(&metric_type).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => Err(internal_error(&format!("Failed to get metrics: {}", e))),
    }
}

async fn get_filters(
    State(state): State<ApiState>,
) -> Result<Json<Vec<FilterSummary>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/filters");

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.list_filters().await {
        Ok(filters) => Ok(Json(filters)),
        Err(e) => Err(internal_error(&format!("Failed to list filters: {}", e))),
    }
}

async fn create_filter(
    State(state): State<ApiState>,
    Json(filter): Json<MetricsFilter>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/metrics/filters");

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.create_filter(&filter).await {
        Ok(filter_id) => Ok(Json(serde_json::json!({
            "id": filter_id,
            "message": "Filter created successfully"
        }))),
        Err(e) => Err(internal_error(&format!("Failed to create filter: {}", e))),
    }
}

async fn get_filter(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<MetricsFilter>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/filters/{}", id);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_filter(&id).await {
        Ok(filter) => Ok(Json(filter)),
        Err(_) => Err(not_found_error("Filter not found")),
    }
}

async fn update_filter(
    Path(id): Path<String>,
    State(state): State<ApiState>,
    Json(filter): Json<MetricsFilter>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("PUT /api/v1/metrics/filters/{}", id);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.update_filter(&id, &filter).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err(internal_error(&format!("Failed to update filter: {}", e))),
    }
}

async fn delete_filter(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("DELETE /api/v1/metrics/filters/{}", id);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.delete_filter(&id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(internal_error(&format!("Failed to delete filter: {}", e))),
    }
}

// Configuration API handlers

async fn list_configs(
    Query(query): Query<ConfigQuery>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<ConfigSummary>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/settings");

    let mut config_manager = state.config_manager.write().await;

    match config_manager.list_configs(query.prefix.as_deref()).await {
        Ok(configs) => Ok(Json(configs)),
        Err(e) => Err(internal_error(&format!("Failed to list configs: {}", e))),
    }
}

async fn get_config(
    Path(path): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Config>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/settings/{}", path);

    let mut config_manager = state.config_manager.write().await;

    match config_manager.load_config(&path).await {
        Ok(config) => Ok(Json(config)),
        Err(_) => Err(not_found_error("Configuration not found")),
    }
}

async fn create_config(
    Path(path): Path<String>,
    State(state): State<ApiState>,
    Json(request): Json<ConfigRequest>,
) -> Result<Json<Config>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/settings/{}", path);

    let mut config_manager = state.config_manager.write().await;

    match config_manager.create_config(
        &path,
        request.content,
        &request.schema_type,
        &request.author,
        request.comment,
    ).await {
        Ok(config) => Ok(Json(config)),
        Err(e) => Err(bad_request_error(&format!("Failed to create config: {}", e))),
    }
}

async fn update_config(
    Path(path): Path<String>,
    State(state): State<ApiState>,
    Json(request): Json<ConfigRequest>,
) -> Result<Json<Config>, (StatusCode, Json<ErrorResponse>)> {
    debug!("PUT /api/v1/settings/{}", path);

    let mut config_manager = state.config_manager.write().await;

    match config_manager.update_config(
        &path,
        request.content,
        &request.author,
        request.comment,
    ).await {
        Ok(config) => Ok(Json(config)),
        Err(e) => Err(bad_request_error(&format!("Failed to update config: {}", e))),
    }
}

async fn delete_config(
    Path(path): Path<String>,
    State(state): State<ApiState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("DELETE /api/v1/settings/{}", path);

    let mut config_manager = state.config_manager.write().await;

    match config_manager.delete_config(&path).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(internal_error(&format!("Failed to delete config: {}", e))),
    }
}

async fn validate_config(
    State(state): State<ApiState>,
    Json(config): Json<Config>,
) -> Result<Json<ValidationResult>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/settings/validate");

    let mut config_manager = state.config_manager.write().await;

    match config_manager.validate_config(&config).await {
        Ok(result) => Ok(Json(result)),
        Err(e) => Err(bad_request_error(&format!("Validation failed: {}", e))),
    }
}

async fn get_schema(
    Path(schema_type): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Value>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/settings/schemas/{}", schema_type);

    let mut config_manager = state.config_manager.write().await;

    match config_manager.get_schema(&schema_type).await {
        Ok(schema) => Ok(Json(schema)),
        Err(_) => Err(not_found_error("Schema not found")),
    }
}

async fn save_schema(
    Path(schema_type): Path<String>,
    State(state): State<ApiState>,
    Json(schema): Json<Value>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("PUT /api/v1/settings/schemas/{}", schema_type);

    let mut config_manager = state.config_manager.write().await;

    match config_manager.save_schema(&schema_type, &schema).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err(bad_request_error(&format!("Failed to save schema: {}", e))),
    }
}

// History API handlers

async fn get_history(
    Path(path): Path<String>,
    Query(query): Query<HistoryQuery>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<HistoryEntry>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/history/{}", path);

    let mut history_manager = state.history_manager.write().await;

    match history_manager.list_history(&path, query.limit).await {
        Ok(history) => Ok(Json(history)),
        Err(e) => Err(internal_error(&format!("Failed to get history: {}", e))),
    }
}

async fn get_version(
    Path((path, version)): Path<(String, u64)>,
    State(state): State<ApiState>,
) -> Result<Json<Config>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/history/{}/version/{}", path, version);

    let mut history_manager = state.history_manager.write().await;

    match history_manager.get_version(&path, version).await {
        Ok(config) => Ok(Json(config)),
        Err(_) => Err(not_found_error("Version not found")),
    }
}

async fn rollback_to_version(
    Path((path, version)): Path<(String, u64)>,
    State(state): State<ApiState>,
    Json(request): Json<ConfigRequest>,
) -> Result<Json<Config>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/history/{}/rollback/{}", path, version);

    let mut history_manager = state.history_manager.write().await;
    let mut config_manager = state.config_manager.write().await;

    match history_manager.rollback_to_version(
        &path,
        version,
        &mut *config_manager,
        &request.author,
        request.comment,
    ).await {
        Ok(config) => Ok(Json(config)),
        Err(e) => Err(bad_request_error(&format!("Rollback failed: {}", e))),
    }
}

async fn diff_versions(
    Path(path): Path<String>,
    State(_state): State<ApiState>,
) -> Result<Json<Vec<crate::settings_history::DiffEntry>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/history/{}/diff", path);
    
    // For simplicity, return empty diff for now
    // This would need version1 and version2 parameters in a real implementation
    Ok(Json(vec![]))
}

// System API handlers

async fn get_system_status() -> Json<serde_json::Value> {
    debug!("GET /api/v1/system/status");

    // Create a dummy status for now
    let status = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "uptime_seconds": 60
    });

    Json(status)
}

async fn health_check() -> StatusCode {
    debug!("GET /api/v1/system/health");
    StatusCode::OK
}

// Helper functions for error responses

fn not_found_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_string(),
            details: None,
        }),
    )
}

fn bad_request_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.to_string(),
            details: None,
        }),
    )
}

fn internal_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    error!("Internal server error: {}", message);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "Internal server error".to_string(),
            details: Some(serde_json::json!({ "details": message })),
        }),
    )
}
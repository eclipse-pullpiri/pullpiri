// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! REST API server module
use crate::monitoring_etcd;
use crate::monitoring_types::{BoardInfo, NodeInfo, SocInfo};
use crate::settings_config::{Config, ConfigManager, ConfigSummary, ValidationResult};
use crate::settings_history::{HistoryEntry, HistoryManager};
use crate::settings_monitoring::{
    BoardListResponse, FilterSummary, Metric, MetricsFilter, MonitoringManager, NodeListResponse,
    SocListResponse,
};
use crate::settings_storage::filter_key;
use crate::settings_utils::error::SettingsError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use chrono::Utc;
use common::monitoringserver::ContainerInfo;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info};

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

/// Query parameters for diff API
#[derive(Debug, Deserialize)]
pub struct DiffQuery {
    pub version1: u64,
    pub version2: u64,
}

/// Query parameters for config listing
#[derive(Debug, Deserialize)]
pub struct ConfigQuery {
    pub prefix: Option<String>,
}

/// Query parameters for resource listing (Node/SoC/Board)
#[derive(Debug, Deserialize)]
pub struct ResourceQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub filter: Option<String>,
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

/// Request body for container creation
#[derive(Debug, Deserialize)]
pub struct CreateContainerRequest {
    pub name: String,
    pub image: String,
    pub node_name: String,
    pub description: Option<String>,
    pub labels: HashMap<String, String>,
}

/// Enhanced pod metrics response with node information
#[derive(Debug, Serialize)]
pub struct PodMetricsResponse {
    pub node_name: String,
    pub hostname: Option<String>,
    pub pod_count: usize,
    pub pods: Vec<PodInfo>,
}

/// Pod information structure following nodeagent resource pattern
#[derive(Debug, Serialize)]
pub struct PodInfo {
    pub container_id: String,
    pub container_name: Option<String>,
    pub image: String,
    pub status: Option<String>,
    pub node_name: String,
    pub hostname: Option<String>,
    pub labels: HashMap<String, String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
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
            .route(
                "/api/v1/metrics/component/:component",
                get(get_metrics_by_component),
            )
            .route(
                "/api/v1/metrics/type/:metric_type",
                get(get_metrics_by_type),
            )
            .route("/api/v1/metrics/filters", get(get_filters))
            .route("/api/v1/metrics/filters", post(create_filter))
            .route("/api/v1/metrics/filters/:id", get(get_filter))
            .route("/api/v1/metrics/filters/:id", delete(delete_filter))
            // Configuration endpoints
            .route("/api/v1/settings", get(list_configs))
            .route("/api/v1/settings/:path", get(get_config))
            .route("/api/v1/settings/:path", post(create_config))
            .route("/api/v1/settings/:path", delete(delete_config))
            .route("/api/v1/settings/validate", post(validate_config))
            .route("/api/v1/settings/schemas/:schema_type", get(get_schema))
            // History endpoints
            .route("/api/v1/history/:path", get(get_history))
            .route("/api/v1/history/:path/version/:version", get(get_version))
            .route(
                "/api/v1/history/:path/rollback/:version",
                post(rollback_to_version),
            )
            .route("/api/v1/history/:path/diff", get(diff_versions))
            // System endpoints
            .route("/api/v1/system/status", get(get_system_status))
            .route("/api/v1/system/health", get(health_check))
            // Node Management APIs - READ ONLY
            .route("/api/v1/nodes", get(list_nodes))
            .route("/api/v1/nodes/:name", get(get_node))
            .route("/api/v1/nodes/:name/pods/metrics", get(get_pod_metrics))
            // Container Management APIs - CHANGED
            .route("/api/v1/containers", get(list_containers))
            .route("/api/v1/containers/:id", get(get_container))
            .route(
                "/api/v1/nodes/:name/containers",
                get(get_containers_by_node),
            )
            // YAML Management APIs - NEW (replacing container create/delete)
            .route("/api/v1/yaml", post(apply_yaml_artifact))
            .route("/api/v1/yaml", delete(withdraw_yaml_artifact))
            // SoC Management APIs - READ ONLY
            .route("/api/v1/socs", get(list_socs))
            .route("/api/v1/socs/:name", get(get_soc))
            // Board Management APIs - READ ONLY
            .route("/api/v1/boards", get(list_boards))
            .route("/api/v1/boards/:name", get(get_board))
            // Integration with monitoring server
            .route("/api/v1/monitoring/sync", post(sync_with_monitoring_server))
            // Additional metrics routes
            .route("/api/v1/metrics/nodes", get(get_all_node_metrics))
            .route("/api/v1/metrics/containers", get(get_all_container_metrics))
            .route("/api/v1/metrics/socs", get(get_all_soc_metrics))
            .route("/api/v1/metrics/boards", get(get_all_board_metrics))
            .route("/api/v1/metrics/nodes/:name", get(get_node_metric_by_name))
            .route(
                "/api/v1/metrics/containers/:id",
                get(get_container_metric_by_id),
            )
            .with_state(self.state.clone())
            .layer(CorsLayer::permissive())
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
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
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

    match monitoring_manager
        .get_metrics_by_component(&component)
        .await
    {
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
    let mut history_manager = state.history_manager.write().await;

    match config_manager
        .create_config(
            &path,
            request.content,
            &request.schema_type,
            &request.author,
            request.comment,
            Some(&mut *history_manager),
        )
        .await
    {
        Ok(config) => Ok(Json(config)),
        Err(e) => Err(bad_request_error(&format!(
            "Failed to create config: {}",
            e
        ))),
    }
}

async fn delete_config(
    Path(path): Path<String>,
    State(state): State<ApiState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("DELETE /api/v1/settings/{}", path);

    let mut config_manager = state.config_manager.write().await;
    let mut history_manager = state.history_manager.write().await;

    match config_manager
        .delete_config(&path, Some(&mut *history_manager))
        .await
    {
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

    match history_manager
        .rollback_to_version(
            &path,
            version,
            &mut *config_manager,
            &request.author,
            request.comment,
        )
        .await
    {
        Ok(config) => Ok(Json(config)),
        Err(e) => Err(bad_request_error(&format!("Rollback failed: {}", e))),
    }
}

async fn diff_versions(
    Path(path): Path<String>,
    Query(query): Query<DiffQuery>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<crate::settings_history::DiffEntry>>, (StatusCode, Json<ErrorResponse>)> {
    debug!(
        "GET /api/v1/history/{}/diff?version1={}&version2={}",
        path, query.version1, query.version2
    );

    let mut history_manager = state.history_manager.write().await;

    // Get both versions
    let version1_result = history_manager.get_version(&path, query.version1).await;
    let version2_result = history_manager.get_version(&path, query.version2).await;

    match (version1_result, version2_result) {
        (Ok(config1), Ok(config2)) => {
            let diff = crate::settings_history::HistoryManager::calculate_diff(
                &config1.content,
                &config2.content,
            );
            Ok(Json(diff))
        }
        (Err(_), _) => Err(not_found_error(&format!(
            "Version {} not found",
            query.version1
        ))),
        (_, Err(_)) => Err(not_found_error(&format!(
            "Version {} not found",
            query.version2
        ))),
    }
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

// Node API handlers
async fn list_nodes(
    Query(query): Query<ResourceQuery>,
    State(_state): State<ApiState>,
) -> Result<Json<NodeListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/nodes with query: {:?}", query);

    // Fetch from monitoring server data via etcd or direct integration
    match fetch_all_nodes_from_monitoring_server().await {
        Ok(nodes) => {
            let filtered_nodes = if let Some(filter) = query.filter {
                nodes
                    .into_iter()
                    .filter(|node| node.node_name.contains(&filter))
                    .collect()
            } else {
                nodes
            };

            Ok(Json(NodeListResponse {
                total: filtered_nodes.len(),
                nodes: filtered_nodes,
            }))
        }
        Err(e) => Err(internal_error(&format!("Failed to fetch nodes: {}", e))),
    }
}

async fn get_node(
    Path(name): Path<String>,
    State(_state): State<ApiState>,
) -> Result<Json<NodeInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/nodes/{}", name);

    match fetch_node_from_monitoring_server(&name).await {
        Ok(Some(node)) => {
            // Fetch logs for this node
            if let Ok(logs) = fetch_node_logs(&name).await {
                // Add logs to the response - we might need to extend NodeInfo struct
                // For now, we'll add it as a custom field in the response
                let mut node_with_logs = serde_json::to_value(&node).unwrap();
                node_with_logs["logs"] = serde_json::to_value(&logs).unwrap();

                return Ok(Json(serde_json::from_value(node_with_logs).unwrap_or(node)));
            }
            Ok(Json(node))
        }
        Ok(None) => Err(not_found_error("Node not found")),
        Err(e) => Err(internal_error(&format!("Failed to get node: {}", e))),
    }
}

// SoC API handlers
async fn list_socs(
    Query(query): Query<ResourceQuery>,
    State(_state): State<ApiState>,
) -> Result<Json<SocListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/socs with query: {:?}", query);

    match fetch_all_socs_from_monitoring_server().await {
        Ok(socs) => {
            let filtered_socs = if let Some(filter) = query.filter {
                socs.into_iter()
                    .filter(|soc| soc.soc_id.contains(&filter))
                    .collect()
            } else {
                socs
            };

            Ok(Json(SocListResponse {
                total: filtered_socs.len(),
                socs: filtered_socs,
            }))
        }
        Err(e) => Err(internal_error(&format!("Failed to fetch SoCs: {}", e))),
    }
}

async fn get_soc(
    Path(name): Path<String>,
    State(_state): State<ApiState>,
) -> Result<Json<SocInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/socs/{}", name);

    match fetch_soc_from_monitoring_server(&name).await {
        Ok(Some(soc)) => {
            // Fetch logs for this SoC
            if let Ok(logs) = fetch_soc_logs(&name).await {
                let mut soc_with_logs = serde_json::to_value(&soc).unwrap();
                soc_with_logs["logs"] = serde_json::to_value(&logs).unwrap();

                return Ok(Json(serde_json::from_value(soc_with_logs).unwrap_or(soc)));
            }
            Ok(Json(soc))
        }
        Ok(None) => Err(not_found_error("SoC not found")),
        Err(e) => Err(internal_error(&format!("Failed to get SoC: {}", e))),
    }
}

// Board API handlers
async fn list_boards(
    Query(query): Query<ResourceQuery>,
    State(_state): State<ApiState>,
) -> Result<Json<BoardListResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/boards with query: {:?}", query);

    match fetch_all_boards_from_monitoring_server().await {
        Ok(boards) => {
            let filtered_boards = if let Some(filter) = query.filter {
                boards
                    .into_iter()
                    .filter(|board| board.board_id.contains(&filter))
                    .collect()
            } else {
                boards
            };

            Ok(Json(BoardListResponse {
                total: filtered_boards.len(),
                boards: filtered_boards,
            }))
        }
        Err(e) => Err(internal_error(&format!("Failed to fetch boards: {}", e))),
    }
}

async fn get_board(
    Path(name): Path<String>,
    State(_state): State<ApiState>,
) -> Result<Json<BoardInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/boards/{}", name);

    match fetch_board_from_monitoring_server(&name).await {
        Ok(Some(board)) => {
            // Fetch logs for this board
            if let Ok(logs) = fetch_board_logs(&name).await {
                let mut board_with_logs = serde_json::to_value(&board).unwrap();
                board_with_logs["logs"] = serde_json::to_value(&logs).unwrap();

                return Ok(Json(
                    serde_json::from_value(board_with_logs).unwrap_or(board),
                ));
            }
            Ok(Json(board))
        }
        Ok(None) => Err(not_found_error("Board not found")),
        Err(e) => Err(internal_error(&format!("Failed to get board: {}", e))),
    }
}

// Integration with monitoring server
async fn sync_with_monitoring_server(
    State(_state): State<ApiState>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/monitoring/sync");

    match sync_monitoring_data().await {
        Ok(_) => Ok(Json(SuccessResponse {
            message: "Successfully synchronized with monitoring server".to_string(),
        })),
        Err(e) => Err(internal_error(&format!("Failed to sync: {}", e))),
    }
}

// Add new endpoints for pod metrics
async fn get_pod_metrics(
    Path(node_name): Path<String>,
    Query(query): Query<ResourceQuery>,
    State(state): State<ApiState>,
) -> Result<Json<PodMetricsResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/nodes/{}/pods/metrics", node_name);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_containers_by_node(&node_name).await {
        Ok(containers) => {
            let mut pods = Vec::new();
            let mut hostname = None;

            for container in containers {
                if hostname.is_none() {
                    hostname = container
                        .config
                        .get("Hostname")
                        .or_else(|| container.annotation.get("hostname"))
                        .or_else(|| container.state.get("hostname"))
                        .cloned();
                }

                let labels: HashMap<String, String> = container
                    .annotation
                    .iter()
                    .filter(|(key, _)| {
                        !key.starts_with("_") && *key != "node_name" && *key != "hostname"
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                let pod_info = PodInfo {
                    container_id: container.id.clone(),
                    container_name: container.names.first().cloned(),
                    image: container.image.clone(),
                    status: container.state.get("Status").cloned(),
                    node_name: node_name.clone(),
                    hostname: container
                        .config
                        .get("Hostname")
                        .or_else(|| container.annotation.get("hostname"))
                        .or_else(|| container.state.get("hostname"))
                        .cloned(),
                    labels,
                    created_at: chrono::Utc::now(),
                };

                pods.push(pod_info);
            }

            let total_pods = pods.len();
            if let Some(page_size) = query.page_size {
                let start_idx = query.page.unwrap_or(0) * page_size;

                if start_idx < total_pods as u32 {
                    pods = pods
                        .into_iter()
                        .skip(start_idx as usize)
                        .take(page_size as usize)
                        .collect();
                } else {
                    pods.clear();
                }
            }

            let response = PodMetricsResponse {
                node_name: node_name.clone(),
                hostname,
                pod_count: total_pods,
                pods,
            };

            Ok(Json(response))
        }
        Err(e) => {
            error!("Failed to get pod metrics for node {}: {}", node_name, e);
            Err(internal_error(&format!("Failed to get pod metrics: {}", e)))
        }
    }
}

// Log fetching functions
async fn fetch_node_logs(node_name: &str) -> Result<Vec<String>, String> {
    // Integration with monitoring server to fetch node logs
    // This could query etcd or directly communicate with monitoring server
    monitoring_etcd::get_node_logs(node_name)
        .await
        .map_err(|e| format!("Failed to fetch node logs: {}", e))
}

async fn fetch_soc_logs(soc_id: &str) -> Result<Vec<String>, String> {
    monitoring_etcd::get_soc_logs(soc_id)
        .await
        .map_err(|e| format!("Failed to fetch SoC logs: {}", e))
}

async fn fetch_board_logs(board_id: &str) -> Result<Vec<String>, String> {
    monitoring_etcd::get_board_logs(board_id)
        .await
        .map_err(|e| format!("Failed to fetch board logs: {}", e))
}

// Integration functions with monitoring server
async fn fetch_all_nodes_from_monitoring_server() -> Result<Vec<NodeInfo>, String> {
    crate::monitoring_etcd::get_all_nodes()
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_node_from_monitoring_server(name: &str) -> Result<Option<NodeInfo>, String> {
    match crate::monitoring_etcd::get_node_info(name).await {
        Ok(node) => Ok(Some(node)),
        Err(crate::monitoring_etcd::MonitoringEtcdError::NotFound) => Ok(None),
        Err(e) => Err(format!("ETCD error: {}", e)),
    }
}

async fn fetch_all_socs_from_monitoring_server() -> Result<Vec<SocInfo>, String> {
    crate::monitoring_etcd::get_all_socs()
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_all_boards_from_monitoring_server() -> Result<Vec<BoardInfo>, String> {
    crate::monitoring_etcd::get_all_boards()
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

// Container integration functions
async fn fetch_all_containers_from_monitoring_server() -> Result<Vec<ContainerInfo>, String> {
    crate::monitoring_etcd::get_all_containers()
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_container_from_monitoring_server(id: &str) -> Result<Option<ContainerInfo>, String> {
    match crate::monitoring_etcd::get_container_info(id).await {
        Ok(container) => Ok(Some(container)),
        Err(crate::monitoring_etcd::MonitoringEtcdError::NotFound) => Ok(None),
        Err(e) => Err(format!("ETCD error: {}", e)),
    }
}

fn not_found_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    debug!("Not found error: {}", message);
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_string(),
            details: None,
        }),
    )
}

fn bad_request_error(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    debug!("Bad request error: {}", message);
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

async fn get_all_node_metrics(
    State(state): State<ApiState>,
) -> Result<Json<Vec<NodeInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/nodes");

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_node_metrics().await {
        Ok(nodes) => {
            info!("Retrieved {} node metrics", nodes.len());
            Ok(Json(nodes))
        }
        Err(e) => {
            error!("Failed to get node metrics: {}", e);
            Err(internal_error(&format!(
                "Failed to get node metrics: {}",
                e
            )))
        }
    }
}

// Container API handlers
async fn list_containers(
    Query(query): Query<ResourceQuery>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<ContainerInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/containers with query: {:?}", query);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_container_metrics().await {
        Ok(containers) => {
            let filtered_containers = if let Some(filter) = query.filter {
                containers
                    .into_iter()
                    .filter(|container| container.id.contains(&filter))
                    .collect()
            } else {
                containers
            };

            Ok(Json(filtered_containers))
        }
        Err(e) => Err(internal_error(&format!(
            "Failed to fetch containers: {}",
            e
        ))),
    }
}

async fn get_container(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ContainerInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/containers/{}", id);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_container_metric_by_id(&id).await {
        Ok(Some(container)) => {
            // Fetch logs for this container
            if let Ok(logs) = monitoring_manager.get_container_logs(&id).await {
                // Add logs to the response - we might need to extend ContainerInfo struct
                // For now, we'll add it as a custom field in the response
                let mut container_with_logs = serde_json::to_value(&container).unwrap();
                container_with_logs["logs"] = serde_json::to_value(&logs).unwrap();

                return Ok(Json(
                    serde_json::from_value(container_with_logs).unwrap_or(container),
                ));
            }
            Ok(Json(container))
        }
        Ok(None) => Err(not_found_error("Container not found")),
        Err(e) => Err(internal_error(&format!("Failed to get container: {}", e))),
    }
}

async fn get_containers_by_node(
    Path(node_name): Path<String>,
    Query(query): Query<ResourceQuery>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<ContainerInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/nodes/{}/containers", node_name);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_containers_by_node(&node_name).await {
        Ok(mut containers) => {
            if let Some(page_size) = query.page_size {
                let start_idx = query.page.unwrap_or(0) * page_size;
                if start_idx < containers.len() as u32 {
                    containers = containers
                        .into_iter()
                        .skip(start_idx as usize)
                        .take(page_size as usize)
                        .collect();
                } else {
                    containers.clear();
                }
            }

            debug!(
                "Found {} containers for node {}",
                containers.len(),
                node_name
            );
            Ok(Json(containers))
        }
        Err(e) => Err(internal_error(&format!(
            "Failed to get containers by node: {}",
            e
        ))),
    }
}

async fn resolve_hostname_for_node(node_name: &str) -> Option<String> {
    match crate::monitoring_etcd::get_node_info(node_name).await {
        Ok(node_info) => Some(node_info.node_name.clone()),
        Err(_) => None,
    }
}

// YAML artifact operations that forward to API Server
async fn apply_yaml_artifact(
    State(_state): State<ApiState>,
    body: String,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/yaml - Applying YAML artifact");

    // Forward to API Server's /api/artifact endpoint
    match send_artifact_to_api_server(&body, "POST").await {
        Ok(response) => {
            info!("Successfully applied YAML artifact to API Server");
            Ok(Json(SuccessResponse {
                message: format!("YAML artifact applied successfully: {}", response),
            }))
        }
        Err(e) => {
            error!("Failed to apply YAML artifact: {}", e);
            Err(internal_error(&format!(
                "Failed to apply YAML artifact: {}",
                e
            )))
        }
    }
}

async fn withdraw_yaml_artifact(
    State(_state): State<ApiState>,
    body: String,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    debug!("DELETE /api/v1/yaml - Withdrawing YAML artifact");

    // Forward to API Server's /api/artifact endpoint
    match send_artifact_to_api_server(&body, "DELETE").await {
        Ok(response) => {
            info!("Successfully withdrew YAML artifact from API Server");
            Ok(Json(SuccessResponse {
                message: format!("YAML artifact withdrawn successfully: {}", response),
            }))
        }
        Err(e) => {
            error!("Failed to withdraw YAML artifact: {}", e);
            Err(internal_error(&format!(
                "Failed to withdraw YAML artifact: {}",
                e
            )))
        }
    }
}

// Helper function to send artifact to API Server
async fn send_artifact_to_api_server(yaml_content: &str, method: &str) -> Result<String, String> {
    use reqwest::Client;

    let client = Client::new();
    let api_server_url = "http://localhost:47099/api/artifact";

    let request = match method {
        "POST" => client.post(api_server_url),
        "DELETE" => client.delete(api_server_url),
        _ => return Err("Unsupported HTTP method".to_string()),
    };

    let response = request
        .header("Content-Type", "text/plain")
        .body(yaml_content.to_string())
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if response.status().is_success() {
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        Ok(response_text)
    } else {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(format!("API Server returned {}: {}", status, error_text))
    }
}

async fn get_all_container_metrics(
    State(state): State<ApiState>,
) -> Result<Json<Vec<ContainerInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/containers");

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_container_metrics().await {
        Ok(containers) => {
            info!("Retrieved {} container metrics", containers.len());
            Ok(Json(containers))
        }
        Err(e) => {
            error!("Failed to get container metrics: {}", e);
            Err(internal_error(&format!(
                "Failed to get container metrics: {}",
                e
            )))
        }
    }
}

async fn get_all_soc_metrics(
    State(state): State<ApiState>,
) -> Result<Json<Vec<SocInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/socs");

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_soc_metrics().await {
        Ok(socs) => {
            info!("Retrieved {} SoC metrics", socs.len());
            Ok(Json(socs))
        }
        Err(e) => {
            error!("Failed to get SoC metrics: {}", e);
            Err(internal_error(&format!("Failed to get SoC metrics: {}", e)))
        }
    }
}

async fn get_all_board_metrics(
    State(state): State<ApiState>,
) -> Result<Json<Vec<BoardInfo>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/boards");

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_board_metrics().await {
        Ok(boards) => {
            info!("Retrieved {} board metrics", boards.len());
            Ok(Json(boards))
        }
        Err(e) => {
            error!("Failed to get board metrics: {}", e);
            Err(internal_error(&format!(
                "Failed to get board metrics: {}",
                e
            )))
        }
    }
}

async fn get_node_metric_by_name(
    Path(name): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<NodeInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/nodes/{}", name);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_node_metric_by_name(&name).await {
        Ok(Some(node)) => Ok(Json(node)),
        Ok(None) => Err(not_found_error("Node metric not found")),
        Err(e) => Err(internal_error(&format!("Failed to get node metric: {}", e))),
    }
}

async fn get_container_metric_by_id(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<ContainerInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/metrics/containers/{}", id);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    match monitoring_manager.get_container_metric_by_id(&id).await {
        Ok(Some(container)) => Ok(Json(container)),
        Ok(None) => Err(not_found_error("Container metric not found")),
        Err(e) => Err(internal_error(&format!(
            "Failed to get container metric: {}",
            e
        ))),
    }
}

// SoC integration functions
async fn fetch_soc_from_monitoring_server(name: &str) -> Result<Option<SocInfo>, String> {
    match crate::monitoring_etcd::get_soc_info(name).await {
        Ok(soc) => Ok(Some(soc)),
        Err(crate::monitoring_etcd::MonitoringEtcdError::NotFound) => Ok(None),
        Err(e) => Err(format!("ETCD error: {}", e)),
    }
}

// Board integration functions
async fn fetch_board_from_monitoring_server(name: &str) -> Result<Option<BoardInfo>, String> {
    match crate::monitoring_etcd::get_board_info(name).await {
        Ok(board) => Ok(Some(board)),
        Err(crate::monitoring_etcd::MonitoringEtcdError::NotFound) => Ok(None),
        Err(e) => Err(format!("ETCD error: {}", e)),
    }
}

// sync function
async fn sync_monitoring_data() -> Result<(), String> {
    debug!("Syncing monitoring data");
    // For now, this is a stub - implement based on our synchronization needs
    Ok(())
}

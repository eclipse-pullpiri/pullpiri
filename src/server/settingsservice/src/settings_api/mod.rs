// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! REST API server module
use crate::monitoring_etcd;
use crate::monitoring_types::{BoardInfo, NodeInfo, SocInfo};
use crate::settings_config::{Config, ConfigManager, ConfigSummary, ValidationResult};
use crate::settings_history::{HistoryEntry, HistoryManager};
use crate::settings_monitoring::{
    BoardListResponse, CreateBoardRequest, CreateNodeRequest, CreateSocRequest, FilterSummary,
    Metric, MetricsFilter, MonitoringManager, NodeListResponse, SocListResponse,
};
use crate::settings_utils::error::SettingsError;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
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
            .route(
                "/api/v1/history/:path/rollback/:version",
                post(rollback_to_version),
            )
            .route("/api/v1/history/:path/diff", get(diff_versions))
            // System endpoints
            .route("/api/v1/system/status", get(get_system_status))
            .route("/api/v1/system/health", get(health_check))
            // Node Management APIs
            .route("/api/v1/nodes", get(list_nodes).post(create_node))
            .route("/api/v1/nodes/:name", get(get_node).delete(delete_node))
            .route("/api/v1/nodes/:name/pods/metrics", get(get_pod_metrics))
            // SoC Management APIs
            .route("/api/v1/socs", get(list_socs).post(create_soc))
            .route("/api/v1/socs/:name", get(get_soc).delete(delete_soc))
            // Board Management APIs
            .route("/api/v1/boards", get(list_boards).post(create_board))
            .route("/api/v1/boards/:name", get(get_board).delete(delete_board))
            // Integration with monitoring server
            .route("/api/v1/monitoring/sync", post(sync_with_monitoring_server))
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

async fn update_config(
    Path(path): Path<String>,
    State(state): State<ApiState>,
    Json(request): Json<ConfigRequest>,
) -> Result<Json<Config>, (StatusCode, Json<ErrorResponse>)> {
    debug!("PUT /api/v1/settings/{}", path);

    let mut config_manager = state.config_manager.write().await;
    let mut history_manager = state.history_manager.write().await;

    match config_manager
        .update_config(
            &path,
            request.content,
            &request.author,
            request.comment,
            Some(&mut *history_manager),
        )
        .await
    {
        Ok(config) => Ok(Json(config)),
        Err(e) => Err(bad_request_error(&format!(
            "Failed to update config: {}",
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

async fn create_node(
    State(_state): State<ApiState>,
    Json(request): Json<CreateNodeRequest>,
) -> Result<Json<NodeInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/nodes with request: {:?}", request);

    // Validate required fields
    if request.name.is_empty() {
        return Err(bad_request_error("Node name is required"));
    }
    if request.ip.is_empty() {
        return Err(bad_request_error("Node IP is required"));
    }
    if request.image.is_empty() {
        return Err(bad_request_error("Node image is required"));
    }

    // Create NodeInfo from request with proper validation
    let node_info = NodeInfo {
        node_name: request.name.clone(),
        ip: request.ip.clone(),
        cpu_usage: 0.0,
        cpu_count: 0,
        gpu_count: 0,
        used_memory: 0,
        total_memory: 0,
        mem_usage: 0.0,
        rx_bytes: 0,
        tx_bytes: 0,
        read_bytes: 0,
        write_bytes: 0,
        os: "Unknown".to_string(),
        arch: "Unknown".to_string(),
    };

    // Store labels and image metadata separately if needed
    if !request.labels.is_empty() {
        if let Err(e) = store_node_metadata(&request.name, &request.image, &request.labels).await {
            return Err(internal_error(&format!(
                "Failed to store node metadata: {}",
                e
            )));
        }
    }

    match create_node_in_monitoring_server(node_info.clone()).await {
        Ok(_) => {
            info!(
                "Successfully created node: {} with image: {}",
                request.name, request.image
            );
            Ok(Json(node_info))
        }
        Err(e) => Err(internal_error(&format!("Failed to create node: {}", e))),
    }
}

async fn delete_node(
    Path(name): Path<String>,
    State(_state): State<ApiState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("DELETE /api/v1/nodes/{}", name);

    match delete_node_from_monitoring_server(&name).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(internal_error(&format!("Failed to delete node: {}", e))),
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

async fn create_soc(
    State(_state): State<ApiState>,
    Json(request): Json<CreateSocRequest>,
) -> Result<Json<SocInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/socs with request: {:?}", request);

    // Validate required fields
    if request.name.is_empty() {
        return Err(bad_request_error("SoC name is required"));
    }

    let soc_info = SocInfo {
        soc_id: request.name.clone(),
        nodes: Vec::new(),
        total_cpu_usage: 0.0,
        total_cpu_count: 0,
        total_gpu_count: 0,
        total_used_memory: 0,
        total_memory: 0,
        total_mem_usage: 0.0,
        total_rx_bytes: 0,
        total_tx_bytes: 0,
        total_read_bytes: 0,
        total_write_bytes: 0,
        last_updated: std::time::SystemTime::now(),
    };

    // Store labels and metadata
    if !request.labels.is_empty() {
        if let Err(e) =
            store_soc_metadata(&request.name, &request.description, &request.labels).await
        {
            return Err(internal_error(&format!(
                "Failed to store SoC metadata: {}",
                e
            )));
        }
    }

    match create_soc_in_monitoring_server(soc_info.clone()).await {
        Ok(_) => {
            info!("Successfully created SoC: {}", request.name);
            Ok(Json(soc_info))
        }
        Err(e) => Err(internal_error(&format!("Failed to create SoC: {}", e))),
    }
}

async fn delete_soc(
    Path(name): Path<String>,
    State(_state): State<ApiState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("DELETE /api/v1/socs/{}", name);

    match delete_soc_from_monitoring_server(&name).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(internal_error(&format!("Failed to delete SoC: {}", e))),
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

async fn create_board(
    State(_state): State<ApiState>,
    Json(request): Json<CreateBoardRequest>,
) -> Result<Json<BoardInfo>, (StatusCode, Json<ErrorResponse>)> {
    debug!("POST /api/v1/boards with request: {:?}", request);

    // Validate required fields
    if request.name.is_empty() {
        return Err(bad_request_error("Board name is required"));
    }

    let board_info = BoardInfo {
        board_id: request.name.clone(),
        nodes: Vec::new(),
        socs: Vec::new(),
        total_cpu_usage: 0.0,
        total_cpu_count: 0,
        total_gpu_count: 0,
        total_used_memory: 0,
        total_memory: 0,
        total_mem_usage: 0.0,
        total_rx_bytes: 0,
        total_tx_bytes: 0,
        total_read_bytes: 0,
        total_write_bytes: 0,
        last_updated: std::time::SystemTime::now(),
    };

    // Store labels and metadata
    if !request.labels.is_empty() {
        if let Err(e) =
            store_board_metadata(&request.name, &request.description, &request.labels).await
        {
            return Err(internal_error(&format!(
                "Failed to store board metadata: {}",
                e
            )));
        }
    }

    match create_board_in_monitoring_server(board_info.clone()).await {
        Ok(_) => {
            info!("Successfully created board: {}", request.name);
            Ok(Json(board_info))
        }
        Err(e) => Err(internal_error(&format!("Failed to create board: {}", e))),
    }
}

async fn delete_board(
    Path(name): Path<String>,
    State(_state): State<ApiState>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    debug!("DELETE /api/v1/boards/{}", name);

    match delete_board_from_monitoring_server(&name).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err(internal_error(&format!("Failed to delete board: {}", e))),
    }
}

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
) -> Result<Json<Vec<Metric>>, (StatusCode, Json<ErrorResponse>)> {
    debug!("GET /api/v1/nodes/{}/pods/metrics", node_name);

    let mut monitoring_manager = state.monitoring_manager.write().await;

    // Create filter for pod metrics on specific node - Fix the HashMap type
    let mut label_selectors = std::collections::HashMap::new();
    label_selectors.insert("node".to_string(), node_name.clone());

    let pod_filter = MetricsFilter {
        id: format!("pod_metrics_{}", node_name),
        name: format!("Pod metrics for node {}", node_name),
        enabled: true,
        components: Some(vec!["pod".to_string()]),
        metric_types: Some(vec![
            "cpu".to_string(),
            "memory".to_string(),
            "network".to_string(),
        ]),
        label_selectors: Some(label_selectors),
        time_range: None,
        refresh_interval: None,
        max_items: query.page_size.map(|ps| ps as usize),
        version: 1,
        created_at: Utc::now(),
        modified_at: Utc::now(),
    };

    match monitoring_manager.get_metrics(Some(&pod_filter)).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => Err(internal_error(&format!("Failed to get pod metrics: {}", e))),
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

// Metadata storage functions
async fn store_node_metadata(
    node_name: &str,
    image: &str,
    labels: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let metadata = serde_json::json!({
        "image": image,
        "labels": labels
    });

    monitoring_etcd::store_node_metadata(node_name, &metadata)
        .await
        .map_err(|e| format!("Failed to store node metadata: {}", e))
}

async fn store_soc_metadata(
    soc_id: &str,
    description: &Option<String>,
    labels: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let metadata = serde_json::json!({
        "description": description,
        "labels": labels
    });

    monitoring_etcd::store_soc_metadata(soc_id, &metadata)
        .await
        .map_err(|e| format!("Failed to store SoC metadata: {}", e))
}

async fn store_board_metadata(
    board_id: &str,
    description: &Option<String>,
    labels: &std::collections::HashMap<String, String>,
) -> Result<(), String> {
    let metadata = serde_json::json!({
        "description": description,
        "labels": labels
    });

    monitoring_etcd::store_board_metadata(board_id, &metadata)
        .await
        .map_err(|e| format!("Failed to store board metadata: {}", e))
}

// Integration functions with monitoring server
async fn fetch_all_nodes_from_monitoring_server() -> Result<Vec<NodeInfo>, String> {
    monitoring_etcd::get_all_nodes()
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_node_from_monitoring_server(name: &str) -> Result<Option<NodeInfo>, String> {
    match monitoring_etcd::get_node_info(name).await {
        Ok(node) => Ok(Some(node)),
        Err(_) => Ok(None),
    }
}

async fn create_node_in_monitoring_server(node: NodeInfo) -> Result<(), String> {
    monitoring_etcd::store_node_info(&node)
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn delete_node_from_monitoring_server(name: &str) -> Result<(), String> {
    monitoring_etcd::delete_node_info(name)
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_all_socs_from_monitoring_server() -> Result<Vec<SocInfo>, String> {
    monitoring_etcd::get_all_socs()
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_soc_from_monitoring_server(id: &str) -> Result<Option<SocInfo>, String> {
    match monitoring_etcd::get_soc_info(id).await {
        Ok(soc) => Ok(Some(soc)),
        Err(_) => Ok(None),
    }
}

async fn create_soc_in_monitoring_server(soc: SocInfo) -> Result<(), String> {
    monitoring_etcd::store_soc_info(&soc)
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn delete_soc_from_monitoring_server(id: &str) -> Result<(), String> {
    monitoring_etcd::delete_soc_info(id)
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_all_boards_from_monitoring_server() -> Result<Vec<BoardInfo>, String> {
    monitoring_etcd::get_all_boards()
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn fetch_board_from_monitoring_server(id: &str) -> Result<Option<BoardInfo>, String> {
    match monitoring_etcd::get_board_info(id).await {
        Ok(board) => Ok(Some(board)),
        Err(_) => Ok(None),
    }
}

async fn create_board_in_monitoring_server(board: BoardInfo) -> Result<(), String> {
    monitoring_etcd::store_board_info(&board)
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn delete_board_from_monitoring_server(id: &str) -> Result<(), String> {
    monitoring_etcd::delete_board_info(id)
        .await
        .map_err(|e| format!("ETCD error: {}", e))
}

async fn sync_monitoring_data() -> Result<(), String> {
    // Implement synchronization logic between settings service and monitoring server
    // This could involve fetching latest data and updating caches
    Ok(())
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

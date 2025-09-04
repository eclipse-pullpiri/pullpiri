/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Cluster management REST API endpoints

use crate::cluster::{NodeInfo, NodeRegistry, NodeResources, NodeRole, NodeStatus};
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::OnceCell;

// Global node registry instance
static NODE_REGISTRY: OnceCell<NodeRegistry> = OnceCell::const_new();

/// Initialize the cluster management system
pub async fn initialize_cluster_management() -> Result<(), Box<dyn std::error::Error>> {
    let registry = NodeRegistry::new();
    registry
        .initialize()
        .await
        .map_err(|e| format!("Failed to initialize node registry: {}", e))?;

    NODE_REGISTRY
        .set(registry)
        .map_err(|_| "Failed to set global node registry")?;

    // Start background task for checking stale nodes
    tokio::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Some(registry) = NODE_REGISTRY.get() {
                if let Err(e) = registry.check_stale_nodes().await {
                    eprintln!("Error checking stale nodes: {}", e);
                }
            }
        }
    });

    Ok(())
}

/// Create cluster router with all endpoints
pub fn cluster_router() -> Router {
    Router::new()
        .route("/api/v1/nodes", get(get_nodes))
        .route("/api/v1/nodes", post(register_node))
        .route("/api/v1/nodes/:node_id", get(get_node))
        .route("/api/v1/nodes/:node_id", delete(remove_node))
        .route("/api/v1/nodes/:node_id/status", post(update_node_status))
        .route("/api/v1/topology", get(get_cluster_topology))
        .route("/api/v1/cluster/health", get(cluster_health))
}

/// Request/Response structures
#[derive(Debug, Serialize, Deserialize)]
pub struct NodeRegistrationRequest {
    pub node_name: String,
    pub ip_address: String,
    pub role: String, // "master" or "sub"
    pub resources: NodeResourcesRequest,
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeResourcesRequest {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_gb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeRegistrationResponse {
    pub success: bool,
    pub message: String,
    pub cluster_id: Option<String>,
    pub node_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeStatusUpdateRequest {
    pub status: String, // "online", "offline", "maintenance", etc.
    pub metrics: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodesQuery {
    pub status: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterHealthResponse {
    pub status: String,
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub master_nodes: usize,
    pub sub_nodes: usize,
}

/// Get all nodes with optional filtering
async fn get_nodes(Query(params): Query<NodesQuery>) -> Response {
    let registry = match NODE_REGISTRY.get() {
        Some(r) => r,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Node registry not initialized",
            )
                .into_response()
        }
    };

    let nodes = match registry.get_all_nodes().await {
        Ok(nodes) => nodes,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get nodes: {}", e),
            )
                .into_response()
        }
    };

    // Apply filters
    let filtered_nodes: Vec<NodeInfo> = nodes
        .into_iter()
        .filter(|node| {
            if let Some(status_filter) = &params.status {
                let node_status = match node.status {
                    NodeStatus::Online => "online",
                    NodeStatus::Offline => "offline",
                    NodeStatus::Initializing => "initializing",
                    NodeStatus::Error => "error",
                    NodeStatus::Maintenance => "maintenance",
                };
                if node_status != status_filter.as_str() {
                    return false;
                }
            }

            if let Some(role_filter) = &params.role {
                let node_role = match node.role {
                    NodeRole::Master => "master",
                    NodeRole::Sub => "sub",
                };
                if node_role != role_filter.as_str() {
                    return false;
                }
            }

            true
        })
        .collect();

    Json(filtered_nodes).into_response()
}

/// Get a specific node by ID
async fn get_node(Path(node_id): Path<String>) -> Response {
    let registry = match NODE_REGISTRY.get() {
        Some(r) => r,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Node registry not initialized",
            )
                .into_response()
        }
    };

    match registry.get_node(&node_id).await {
        Ok(node) => (StatusCode::OK, Json(node)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Node not found").into_response(),
    }
}

/// Register a new node
async fn register_node(Json(payload): Json<NodeRegistrationRequest>) -> Response {
    let registry = match NODE_REGISTRY.get() {
        Some(r) => r,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Node registry not initialized",
            )
                .into_response()
        }
    };

    // Generate unique node ID
    let node_id = format!(
        "{}-{}",
        payload.node_name,
        chrono::Utc::now().timestamp_millis()
    );

    // Parse role
    let role = match payload.role.to_lowercase().as_str() {
        "master" => NodeRole::Master,
        "sub" => NodeRole::Sub,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                "Invalid role. Must be 'master' or 'sub'",
            )
                .into_response()
        }
    };

    // Create node info
    let mut node_info = NodeInfo::new(node_id.clone(), payload.node_name, payload.ip_address);
    node_info.role = role;
    node_info.resources = NodeResources {
        cpu_cores: payload.resources.cpu_cores,
        memory_mb: payload.resources.memory_mb,
        disk_gb: payload.resources.disk_gb,
        cpu_usage: 0.0,
        memory_usage: 0.0,
    };

    if let Some(labels) = payload.labels {
        node_info.labels = labels;
    }

    match registry.register_node(node_info).await {
        Ok(cluster_id) => {
            let response = NodeRegistrationResponse {
                success: true,
                message: "Node registered successfully".to_string(),
                cluster_id: Some(cluster_id),
                node_id: Some(node_id),
            };
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let response = NodeRegistrationResponse {
                success: false,
                message: format!("Failed to register node: {}", e),
                cluster_id: None,
                node_id: None,
            };
            (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response()
        }
    }
}

/// Update node status
async fn update_node_status(
    Path(node_id): Path<String>,
    Json(payload): Json<NodeStatusUpdateRequest>,
) -> Response {
    let registry = match NODE_REGISTRY.get() {
        Some(r) => r,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Node registry not initialized",
            )
                .into_response()
        }
    };

    // Parse status
    let status = match payload.status.to_lowercase().as_str() {
        "online" => NodeStatus::Online,
        "offline" => NodeStatus::Offline,
        "initializing" => NodeStatus::Initializing,
        "error" => NodeStatus::Error,
        "maintenance" => NodeStatus::Maintenance,
        _ => return (StatusCode::BAD_REQUEST, "Invalid status").into_response(),
    };

    match registry
        .update_node_status(&node_id, status, payload.metrics)
        .await
    {
        Ok(_) => (StatusCode::OK, "Node status updated successfully").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update node status: {}", e),
        )
            .into_response(),
    }
}

/// Remove a node from the cluster
async fn remove_node(Path(node_id): Path<String>) -> Response {
    let registry = match NODE_REGISTRY.get() {
        Some(r) => r,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Node registry not initialized",
            )
                .into_response()
        }
    };

    match registry.remove_node(&node_id).await {
        Ok(_) => (StatusCode::OK, "Node removed successfully").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to remove node: {}", e),
        )
            .into_response(),
    }
}

/// Get cluster topology
async fn get_cluster_topology() -> Response {
    let registry = match NODE_REGISTRY.get() {
        Some(r) => r,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Node registry not initialized",
            )
                .into_response()
        }
    };

    match registry.get_cluster_topology("default").await {
        Ok(topology) => (StatusCode::OK, Json(topology)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get cluster topology: {}", e),
        )
            .into_response(),
    }
}

/// Get cluster health status
async fn cluster_health() -> Response {
    let registry = match NODE_REGISTRY.get() {
        Some(r) => r,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Node registry not initialized",
            )
                .into_response()
        }
    };

    match registry.get_all_nodes().await {
        Ok(nodes) => {
            let total_nodes = nodes.len();
            let online_nodes = nodes.iter().filter(|n| n.is_online()).count();
            let master_nodes = nodes
                .iter()
                .filter(|n| matches!(n.role, NodeRole::Master))
                .count();
            let sub_nodes = nodes
                .iter()
                .filter(|n| matches!(n.role, NodeRole::Sub))
                .count();

            let status = if online_nodes == 0 {
                "unhealthy"
            } else if online_nodes < total_nodes {
                "degraded"
            } else {
                "healthy"
            };

            let health = ClusterHealthResponse {
                status: status.to_string(),
                total_nodes,
                online_nodes,
                master_nodes,
                sub_nodes,
            };

            Json(health).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get cluster health: {}", e),
        )
            .into_response(),
    }
}

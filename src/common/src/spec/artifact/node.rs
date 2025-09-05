/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node clustering specifications and data structures

use super::Artifact;
use super::Node;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

impl Artifact for Node {
    fn get_name(&self) -> String {
        self.metadata.name.clone()
    }
}

impl Node {
    pub fn get_spec(&self) -> &Option<NodeSpec> {
        &self.spec
    }
}

/// Node specification for clustering
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct NodeSpec {
    pub node_info: Option<NodeInfo>,
    pub cluster_config: Option<ClusterConfig>,
}

impl NodeSpec {
    pub fn get_node_info(&self) -> &Option<NodeInfo> {
        &self.node_info
    }

    pub fn get_cluster_config(&self) -> &Option<ClusterConfig> {
        &self.cluster_config
    }
}

/// Node role in the cluster
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeRole {
    Master,
    Sub,
}

/// Node status in the cluster
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Offline,
    Online,
    Initializing,
    Error,
    Maintenance,
}

/// Node resource information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeResources {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub disk_gb: u64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

/// Node information structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeInfo {
    pub node_id: String,
    pub node_name: String,
    pub ip_address: String,
    pub role: NodeRole,
    pub status: NodeStatus,
    pub resources: NodeResources,
    pub labels: HashMap<String, String>,
    pub created_at: i64,
    pub last_heartbeat: i64,
}

/// Cluster configuration for nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterConfig {
    pub cluster_id: String,
    pub master_endpoint: String,
    pub heartbeat_interval: u64,
    pub config: HashMap<String, String>,
}

/// Cluster topology types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TopologyType {
    Simple,
    Hierarchical,
    Mesh,
    Hybrid,
}

/// Cluster topology structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterTopology {
    pub cluster_id: String,
    pub cluster_name: String,
    pub topology_type: TopologyType,
    pub master_nodes: Vec<NodeInfo>,
    pub sub_nodes: Vec<NodeInfo>,
    pub config: HashMap<String, String>,
}

impl Default for NodeResources {
    fn default() -> Self {
        Self {
            cpu_cores: 1,
            memory_mb: 512,
            disk_gb: 10,
            cpu_usage: 0.0,
            memory_usage: 0.0,
        }
    }
}

impl Default for NodeStatus {
    fn default() -> Self {
        NodeStatus::Offline
    }
}

impl Default for NodeRole {
    fn default() -> Self {
        NodeRole::Sub
    }
}

impl NodeInfo {
    pub fn new(node_id: String, node_name: String, ip_address: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            node_id,
            node_name,
            ip_address,
            role: NodeRole::default(),
            status: NodeStatus::default(),
            resources: NodeResources::default(),
            labels: HashMap::new(),
            created_at: now,
            last_heartbeat: now,
        }
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = chrono::Utc::now().timestamp();
    }

    pub fn is_online(&self) -> bool {
        matches!(self.status, NodeStatus::Online)
    }

    pub fn heartbeat_age(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.last_heartbeat
    }
}

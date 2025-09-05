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

    pub fn get_status(&self) -> &Option<NodeStatus> {
        &self.status
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
    pub status: NodeState,
    pub resources: NodeResources,
    pub labels: HashMap<String, String>,
    pub created_at: i64,
    pub last_heartbeat: i64,
}

/// Simplified node registration information for gRPC communication
/// Contains only essential information needed for node registration and heartbeat
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeRegistrationInfo {
    pub node_id: String,
    pub node_name: String,
    pub ip_address: String,
    pub role: NodeRole,
    pub resources: Option<NodeResources>,
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

/// Node status structure following Kubernetes pattern
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeStatus {
    pub state: NodeState,
    pub conditions: Vec<NodeCondition>,
    pub addresses: Vec<NodeAddress>,
    pub capacity: Option<NodeResources>,
    pub allocatable: Option<NodeResources>,
    pub phase: NodeState,
    pub last_heartbeat_time: Option<i64>,
    pub node_info: Option<NodeSystemInfo>,
}

/// Node state enumeration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeState {
    Ready,
    NotReady,
    Unknown,
}

/// Node condition structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeCondition {
    pub condition_type: NodeConditionType,
    pub status: ConditionStatus,
    pub last_heartbeat_time: Option<i64>,
    pub last_transition_time: Option<i64>,
    pub reason: Option<String>,
    pub message: Option<String>,
}

/// Node condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeConditionType {
    Ready,
    MemoryPressure,
    DiskPressure,
    PIDPressure,
    NetworkUnavailable,
}

/// Condition status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionStatus {
    True,
    False,
    Unknown,
}

/// Node address structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeAddress {
    pub address_type: NodeAddressType,
    pub address: String,
}

/// Node address types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeAddressType {
    Hostname,
    ExternalIP,
    InternalIP,
    ExternalDNS,
    InternalDNS,
}

/// Node system information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeSystemInfo {
    pub machine_id: String,
    pub system_uuid: String,
    pub boot_id: String,
    pub kernel_version: String,
    pub os_image: String,
    pub container_runtime_version: String,
    pub kubelet_version: String,
    pub kube_proxy_version: String,
    pub operating_system: String,
    pub architecture: String,
}

impl Default for NodeState {
    fn default() -> Self {
        NodeState::Unknown
    }
}

impl Default for ConditionStatus {
    fn default() -> Self {
        ConditionStatus::Unknown
    }
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

impl Default for NodeRole {
    fn default() -> Self {
        NodeRole::Sub
    }
}

impl NodeStatus {
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            state: NodeState::Unknown,
            conditions: vec![],
            addresses: vec![],
            capacity: None,
            allocatable: None,
            phase: NodeState::Unknown,
            last_heartbeat_time: Some(now),
            node_info: None,
        }
    }

    pub fn ready() -> Self {
        let now = chrono::Utc::now().timestamp();
        let mut status = Self::new();
        status.state = NodeState::Ready;
        status.phase = NodeState::Ready;
        status.conditions.push(NodeCondition {
            condition_type: NodeConditionType::Ready,
            status: ConditionStatus::True,
            last_heartbeat_time: Some(now),
            last_transition_time: Some(now),
            reason: Some("KubeletReady".to_string()),
            message: Some("kubelet is posting ready status".to_string()),
        });
        status
    }

    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat_time = Some(chrono::Utc::now().timestamp());
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, NodeState::Ready)
    }
}

impl NodeCondition {
    pub fn new(condition_type: NodeConditionType, status: ConditionStatus) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            condition_type,
            status,
            last_heartbeat_time: Some(now),
            last_transition_time: Some(now),
            reason: None,
            message: None,
        }
    }
}

impl NodeAddress {
    pub fn new(address_type: NodeAddressType, address: String) -> Self {
        Self {
            address_type,
            address,
        }
    }
}

impl NodeRegistrationInfo {
    pub fn new(node_id: String, node_name: String, ip_address: String) -> Self {
        Self {
            node_id,
            node_name,
            ip_address,
            role: NodeRole::default(),
            resources: Some(NodeResources::default()),
        }
    }

    pub fn with_role(mut self, role: NodeRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_resources(mut self, resources: NodeResources) -> Self {
        self.resources = Some(resources);
        self
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
            status: NodeState::default(),
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
        matches!(self.status, NodeState::Ready)
    }

    pub fn heartbeat_age(&self) -> i64 {
        chrono::Utc::now().timestamp() - self.last_heartbeat
    }
}

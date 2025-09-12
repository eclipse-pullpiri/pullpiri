// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Monitoring data types for integration with monitoring server

use serde::{Deserialize, Serialize};

// Re-export the monitoring server NodeInfo from protobuf
// Since we need to use the protobuf types, let's define compatible types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NodeInfo {
    pub node_name: String,
    pub cpu_usage: f64,
    pub cpu_count: u64,
    pub gpu_count: u64,
    pub used_memory: u64,
    pub total_memory: u64,
    pub mem_usage: f64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub os: String,
    pub arch: String,
    pub ip: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SocInfo {
    pub soc_id: String,
    pub nodes: Vec<NodeInfo>,
    pub total_cpu_usage: f64,
    pub total_cpu_count: u64,
    pub total_gpu_count: u64,
    pub total_used_memory: u64,
    pub total_memory: u64,
    pub total_mem_usage: f64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
    pub last_updated: std::time::SystemTime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoardInfo {
    pub board_id: String,
    pub nodes: Vec<NodeInfo>,
    pub socs: Vec<SocInfo>,
    pub total_cpu_usage: f64,
    pub total_cpu_count: u64,
    pub total_gpu_count: u64,
    pub total_used_memory: u64,
    pub total_memory: u64,
    pub total_mem_usage: f64,
    pub total_rx_bytes: u64,
    pub total_tx_bytes: u64,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
    pub last_updated: std::time::SystemTime,
}

// Conversion functions from protobuf types if needed
impl From<common::monitoringserver::NodeInfo> for NodeInfo {
    fn from(proto_node: common::monitoringserver::NodeInfo) -> Self {
        Self {
            node_name: proto_node.node_name,
            cpu_usage: proto_node.cpu_usage,
            cpu_count: proto_node.cpu_count,
            gpu_count: proto_node.gpu_count,
            used_memory: proto_node.used_memory,
            total_memory: proto_node.total_memory,
            mem_usage: proto_node.mem_usage,
            rx_bytes: proto_node.rx_bytes,
            tx_bytes: proto_node.tx_bytes,
            read_bytes: proto_node.read_bytes,
            write_bytes: proto_node.write_bytes,
            os: proto_node.os,
            arch: proto_node.arch,
            ip: proto_node.ip,
        }
    }
}

impl Into<common::monitoringserver::NodeInfo> for NodeInfo {
    fn into(self) -> common::monitoringserver::NodeInfo {
        common::monitoringserver::NodeInfo {
            node_name: self.node_name,
            cpu_usage: self.cpu_usage,
            cpu_count: self.cpu_count,
            gpu_count: self.gpu_count,
            used_memory: self.used_memory,
            total_memory: self.total_memory,
            mem_usage: self.mem_usage,
            rx_bytes: self.rx_bytes,
            tx_bytes: self.tx_bytes,
            read_bytes: self.read_bytes,
            write_bytes: self.write_bytes,
            os: self.os,
            arch: self.arch,
            ip: self.ip,
        }
    }
}

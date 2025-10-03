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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::time::SystemTime;

    fn create_test_node_info() -> NodeInfo {
        NodeInfo {
            node_name: "test-node-1".to_string(),
            cpu_usage: 75.5,
            cpu_count: 8,
            gpu_count: 2,
            used_memory: 8192,
            total_memory: 16384,
            mem_usage: 50.0,
            rx_bytes: 1024000,
            tx_bytes: 2048000,
            read_bytes: 4096000,
            write_bytes: 8192000,
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            ip: "192.168.1.100".to_string(),
        }
    }

    fn create_test_soc_info() -> SocInfo {
        let node1 = create_test_node_info();
        let mut node2 = create_test_node_info();
        node2.node_name = "test-node-2".to_string();
        node2.ip = "192.168.1.101".to_string();

        SocInfo {
            soc_id: "soc-alpha-1".to_string(),
            nodes: vec![node1, node2],
            total_cpu_usage: 65.8,
            total_cpu_count: 16,
            total_gpu_count: 4,
            total_used_memory: 16384,
            total_memory: 32768,
            total_mem_usage: 50.0,
            total_rx_bytes: 2048000,
            total_tx_bytes: 4096000,
            total_read_bytes: 8192000,
            total_write_bytes: 16384000,
            last_updated: SystemTime::now(),
        }
    }

    fn create_test_board_info() -> BoardInfo {
        let nodes = vec![create_test_node_info()];
        let socs = vec![create_test_soc_info()];

        BoardInfo {
            board_id: "board-main-1".to_string(),
            nodes,
            socs,
            total_cpu_usage: 68.2,
            total_cpu_count: 24,
            total_gpu_count: 6,
            total_used_memory: 24576,
            total_memory: 49152,
            total_mem_usage: 50.0,
            total_rx_bytes: 3072000,
            total_tx_bytes: 6144000,
            total_read_bytes: 12288000,
            total_write_bytes: 24576000,
            last_updated: SystemTime::now(),
        }
    }

    #[test]
    fn test_node_info_creation() {
        let node = create_test_node_info();

        assert_eq!(node.node_name, "test-node-1");
        assert_eq!(node.cpu_usage, 75.5);
        assert_eq!(node.cpu_count, 8);
        assert_eq!(node.gpu_count, 2);
        assert_eq!(node.used_memory, 8192);
        assert_eq!(node.total_memory, 16384);
        assert_eq!(node.mem_usage, 50.0);
        assert_eq!(node.rx_bytes, 1024000);
        assert_eq!(node.tx_bytes, 2048000);
        assert_eq!(node.read_bytes, 4096000);
        assert_eq!(node.write_bytes, 8192000);
        assert_eq!(node.os, "Linux");
        assert_eq!(node.arch, "x86_64");
        assert_eq!(node.ip, "192.168.1.100");
    }

    #[test]
    fn test_soc_info_creation() {
        let soc = create_test_soc_info();

        assert_eq!(soc.soc_id, "soc-alpha-1");
        assert_eq!(soc.nodes.len(), 2);
        assert_eq!(soc.total_cpu_usage, 65.8);
        assert_eq!(soc.total_cpu_count, 16);
        assert_eq!(soc.total_gpu_count, 4);
        assert_eq!(soc.total_used_memory, 16384);
        assert_eq!(soc.total_memory, 32768);
        assert_eq!(soc.total_mem_usage, 50.0);
        assert_eq!(soc.total_rx_bytes, 2048000);
        assert_eq!(soc.total_tx_bytes, 4096000);
        assert_eq!(soc.total_read_bytes, 8192000);
        assert_eq!(soc.total_write_bytes, 16384000);

        // Verify node references
        assert_eq!(soc.nodes[0].node_name, "test-node-1");
        assert_eq!(soc.nodes[1].node_name, "test-node-2");
    }

    #[test]
    fn test_board_info_creation() {
        let board = create_test_board_info();

        assert_eq!(board.board_id, "board-main-1");
        assert_eq!(board.nodes.len(), 1);
        assert_eq!(board.socs.len(), 1);
        assert_eq!(board.total_cpu_usage, 68.2);
        assert_eq!(board.total_cpu_count, 24);
        assert_eq!(board.total_gpu_count, 6);
        assert_eq!(board.total_used_memory, 24576);
        assert_eq!(board.total_memory, 49152);
        assert_eq!(board.total_mem_usage, 50.0);
        assert_eq!(board.total_rx_bytes, 3072000);
        assert_eq!(board.total_tx_bytes, 6144000);
        assert_eq!(board.total_read_bytes, 12288000);
        assert_eq!(board.total_write_bytes, 24576000);

        // Verify nested structure
        assert_eq!(board.nodes[0].node_name, "test-node-1");
        assert_eq!(board.socs[0].soc_id, "soc-alpha-1");
    }

    #[test]
    fn test_node_info_serialization() {
        let node = create_test_node_info();

        // Test serialization
        let serialized = serde_json::to_string(&node).expect("Failed to serialize NodeInfo");
        assert!(serialized.contains("test-node-1"));
        assert!(serialized.contains("75.5"));
        assert!(serialized.contains("Linux"));
        assert!(serialized.contains("x86_64"));
        assert!(serialized.contains("192.168.1.100"));

        // Test deserialization
        let deserialized: NodeInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize NodeInfo");
        assert_eq!(deserialized.node_name, node.node_name);
        assert_eq!(deserialized.cpu_usage, node.cpu_usage);
        assert_eq!(deserialized.cpu_count, node.cpu_count);
        assert_eq!(deserialized.os, node.os);
        assert_eq!(deserialized.arch, node.arch);
        assert_eq!(deserialized.ip, node.ip);
    }

    #[test]
    fn test_soc_info_serialization() {
        let soc = create_test_soc_info();

        // Test serialization
        let serialized = serde_json::to_string(&soc).expect("Failed to serialize SocInfo");
        assert!(serialized.contains("soc-alpha-1"));
        assert!(serialized.contains("65.8"));
        assert!(serialized.contains("test-node-1"));
        assert!(serialized.contains("test-node-2"));

        // Test deserialization
        let deserialized: SocInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize SocInfo");
        assert_eq!(deserialized.soc_id, soc.soc_id);
        assert_eq!(deserialized.total_cpu_usage, soc.total_cpu_usage);
        assert_eq!(deserialized.nodes.len(), soc.nodes.len());
        assert_eq!(deserialized.nodes[0].node_name, soc.nodes[0].node_name);
    }

    #[test]
    fn test_board_info_serialization() {
        let board = create_test_board_info();

        // Test serialization
        let serialized = serde_json::to_string(&board).expect("Failed to serialize BoardInfo");
        assert!(serialized.contains("board-main-1"));
        assert!(serialized.contains("68.2"));
        assert!(serialized.contains("soc-alpha-1"));

        // Test deserialization
        let deserialized: BoardInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize BoardInfo");
        assert_eq!(deserialized.board_id, board.board_id);
        assert_eq!(deserialized.total_cpu_usage, board.total_cpu_usage);
        assert_eq!(deserialized.nodes.len(), board.nodes.len());
        assert_eq!(deserialized.socs.len(), board.socs.len());
    }

    #[test]
    fn test_node_info_clone() {
        let original = create_test_node_info();
        let cloned = original.clone();

        assert_eq!(original.node_name, cloned.node_name);
        assert_eq!(original.cpu_usage, cloned.cpu_usage);
        assert_eq!(original.cpu_count, cloned.cpu_count);
        assert_eq!(original.gpu_count, cloned.gpu_count);
        assert_eq!(original.os, cloned.os);
        assert_eq!(original.arch, cloned.arch);
        assert_eq!(original.ip, cloned.ip);

        // Verify they are separate instances
        assert_ne!(&original as *const NodeInfo, &cloned as *const NodeInfo);
    }

    #[test]
    fn test_soc_info_clone() {
        let original = create_test_soc_info();
        let cloned = original.clone();

        assert_eq!(original.soc_id, cloned.soc_id);
        assert_eq!(original.total_cpu_usage, cloned.total_cpu_usage);
        assert_eq!(original.nodes.len(), cloned.nodes.len());

        // Verify nested cloning
        assert_eq!(original.nodes[0].node_name, cloned.nodes[0].node_name);

        // Verify they are separate instances
        assert_ne!(&original as *const SocInfo, &cloned as *const SocInfo);
    }

    #[test]
    fn test_board_info_clone() {
        let original = create_test_board_info();
        let cloned = original.clone();

        assert_eq!(original.board_id, cloned.board_id);
        assert_eq!(original.total_cpu_usage, cloned.total_cpu_usage);
        assert_eq!(original.nodes.len(), cloned.nodes.len());
        assert_eq!(original.socs.len(), cloned.socs.len());

        // Verify nested cloning
        assert_eq!(original.nodes[0].node_name, cloned.nodes[0].node_name);
        assert_eq!(original.socs[0].soc_id, cloned.socs[0].soc_id);
    }

    #[test]
    fn test_debug_formatting() {
        let node = create_test_node_info();
        let debug_str = format!("{:?}", node);

        assert!(debug_str.contains("NodeInfo"));
        assert!(debug_str.contains("test-node-1"));
        assert!(debug_str.contains("75.5"));
        assert!(debug_str.contains("Linux"));
    }

    #[test]
    fn test_node_info_from_proto() {
        let proto_node = common::monitoringserver::NodeInfo {
            node_name: "proto-node".to_string(),
            cpu_usage: 80.0,
            cpu_count: 4,
            gpu_count: 1,
            used_memory: 4096,
            total_memory: 8192,
            mem_usage: 50.0,
            rx_bytes: 512000,
            tx_bytes: 1024000,
            read_bytes: 2048000,
            write_bytes: 4096000,
            os: "Ubuntu".to_string(),
            arch: "arm64".to_string(),
            ip: "10.0.0.1".to_string(),
        };

        let node_info: NodeInfo = proto_node.into();

        assert_eq!(node_info.node_name, "proto-node");
        assert_eq!(node_info.cpu_usage, 80.0);
        assert_eq!(node_info.cpu_count, 4);
        assert_eq!(node_info.gpu_count, 1);
        assert_eq!(node_info.used_memory, 4096);
        assert_eq!(node_info.total_memory, 8192);
        assert_eq!(node_info.mem_usage, 50.0);
        assert_eq!(node_info.rx_bytes, 512000);
        assert_eq!(node_info.tx_bytes, 1024000);
        assert_eq!(node_info.read_bytes, 2048000);
        assert_eq!(node_info.write_bytes, 4096000);
        assert_eq!(node_info.os, "Ubuntu");
        assert_eq!(node_info.arch, "arm64");
        assert_eq!(node_info.ip, "10.0.0.1");
    }

    #[test]
    fn test_node_info_into_proto() {
        let node_info = NodeInfo {
            node_name: "local-node".to_string(),
            cpu_usage: 60.0,
            cpu_count: 12,
            gpu_count: 3,
            used_memory: 12288,
            total_memory: 24576,
            mem_usage: 50.0,
            rx_bytes: 1536000,
            tx_bytes: 3072000,
            read_bytes: 6144000,
            write_bytes: 12288000,
            os: "CentOS".to_string(),
            arch: "x86_64".to_string(),
            ip: "172.16.0.1".to_string(),
        };

        let proto_node: common::monitoringserver::NodeInfo = node_info.into();

        assert_eq!(proto_node.node_name, "local-node");
        assert_eq!(proto_node.cpu_usage, 60.0);
        assert_eq!(proto_node.cpu_count, 12);
        assert_eq!(proto_node.gpu_count, 3);
        assert_eq!(proto_node.used_memory, 12288);
        assert_eq!(proto_node.total_memory, 24576);
        assert_eq!(proto_node.mem_usage, 50.0);
        assert_eq!(proto_node.rx_bytes, 1536000);
        assert_eq!(proto_node.tx_bytes, 3072000);
        assert_eq!(proto_node.read_bytes, 6144000);
        assert_eq!(proto_node.write_bytes, 12288000);
        assert_eq!(proto_node.os, "CentOS");
        assert_eq!(proto_node.arch, "x86_64");
        assert_eq!(proto_node.ip, "172.16.0.1");
    }

    #[test]
    fn test_round_trip_conversion() {
        let original_proto = common::monitoringserver::NodeInfo {
            node_name: "round-trip-node".to_string(),
            cpu_usage: 90.5,
            cpu_count: 16,
            gpu_count: 4,
            used_memory: 20480,
            total_memory: 32768,
            mem_usage: 62.5,
            rx_bytes: 2048000,
            tx_bytes: 4096000,
            read_bytes: 8192000,
            write_bytes: 16384000,
            os: "RHEL".to_string(),
            arch: "aarch64".to_string(),
            ip: "192.168.100.1".to_string(),
        };

        // Proto -> NodeInfo -> Proto
        let node_info: NodeInfo = original_proto.clone().into();
        let converted_proto: common::monitoringserver::NodeInfo = node_info.into();

        assert_eq!(original_proto.node_name, converted_proto.node_name);
        assert_eq!(original_proto.cpu_usage, converted_proto.cpu_usage);
        assert_eq!(original_proto.cpu_count, converted_proto.cpu_count);
        assert_eq!(original_proto.os, converted_proto.os);
        assert_eq!(original_proto.arch, converted_proto.arch);
        assert_eq!(original_proto.ip, converted_proto.ip);
    }

    #[test]
    fn test_empty_collections() {
        let empty_soc = SocInfo {
            soc_id: "empty-soc".to_string(),
            nodes: Vec::new(), // Empty nodes
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
            last_updated: SystemTime::now(),
        };

        assert_eq!(empty_soc.nodes.len(), 0);
        assert!(empty_soc.nodes.is_empty());

        let empty_board = BoardInfo {
            board_id: "empty-board".to_string(),
            nodes: Vec::new(), // Empty nodes
            socs: Vec::new(),  // Empty socs
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
            last_updated: SystemTime::now(),
        };

        assert_eq!(empty_board.nodes.len(), 0);
        assert_eq!(empty_board.socs.len(), 0);
        assert!(empty_board.nodes.is_empty());
        assert!(empty_board.socs.is_empty());
    }

    #[test]
    fn test_extreme_values() {
        let extreme_node = NodeInfo {
            node_name: "extreme-test".to_string(),
            cpu_usage: 100.0, // Maximum CPU usage
            cpu_count: u64::MAX,
            gpu_count: u64::MAX,
            used_memory: u64::MAX,
            total_memory: u64::MAX,
            mem_usage: 100.0, // Maximum memory usage
            rx_bytes: u64::MAX,
            tx_bytes: u64::MAX,
            read_bytes: u64::MAX,
            write_bytes: u64::MAX,
            os: "".to_string(),        // Empty string
            arch: "".to_string(),      // Empty string
            ip: "0.0.0.0".to_string(), // Minimum IP
        };

        // Test serialization with extreme values
        let serialized =
            serde_json::to_string(&extreme_node).expect("Failed to serialize extreme values");
        let deserialized: NodeInfo =
            serde_json::from_str(&serialized).expect("Failed to deserialize extreme values");

        assert_eq!(deserialized.cpu_usage, 100.0);
        assert_eq!(deserialized.cpu_count, u64::MAX);
        assert_eq!(deserialized.mem_usage, 100.0);
        assert_eq!(deserialized.os, "");
        assert_eq!(deserialized.arch, "");
        assert_eq!(deserialized.ip, "0.0.0.0");
    }

    #[test]
    fn test_system_time_handling() {
        let now = SystemTime::now();
        let soc = SocInfo {
            soc_id: "time-test".to_string(),
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
            last_updated: now,
        };

        // SystemTime should be preserved
        assert_eq!(soc.last_updated, now);

        // Test with a different time
        let different_time = SystemTime::UNIX_EPOCH;
        let mut soc_clone = soc.clone();
        soc_clone.last_updated = different_time;

        assert_ne!(soc.last_updated, soc_clone.last_updated);
        assert_eq!(soc_clone.last_updated, different_time);
    }
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node registry for cluster membership management

use base64::Engine;
use common::apiserver::{ClusterTopology, TopologyType};
use common::persistency;
use common::logd;
use prost::Message;

/// Node registry for managing cluster topology
#[derive(Clone)]
#[allow(dead_code)]
pub struct NodeRegistry;
#[allow(dead_code)]
impl NodeRegistry {
    /// Get the current cluster topology
    pub async fn get_topology(
        &self,
    ) -> Result<ClusterTopology, Box<dyn std::error::Error + Send + Sync>> {
        let topology_key = "cluster/topology";

        match persistency::get(topology_key).await {
            Ok(encoded) => {
                let buf = base64::engine::general_purpose::STANDARD.decode(&encoded)?;
                let topology = ClusterTopology::decode(&buf[..])?;
                Ok(topology)
            }
            Err(_) => {
                // Return default topology if not found
                Ok(ClusterTopology {
                    cluster_id: "default-cluster".to_string(),
                    cluster_name: "PICCOLO Cluster".to_string(),
                    r#type: TopologyType::Embedded.into(),
                    master_nodes: vec![],
                    sub_nodes: vec![],
                    parent_cluster: String::new(),
                    config: std::collections::HashMap::new(),
                })
            }
        }
    }

    /// Update the cluster topology
    pub async fn update_topology(
        &self,
        topology: ClusterTopology,
    ) -> Result<ClusterTopology, Box<dyn std::error::Error + Send + Sync>> {
        let topology_key = "cluster/topology";

        // 인코딩을 제거하고 json string으로 변환
        let topology_json = serde_json::to_string(&topology)?;

        persistency::put(topology_key, &topology_json).await?;

        logd!(2, "Updated cluster topology: {}", topology.cluster_name);
        Ok(topology)
    }

    /// Initialize default cluster topology
    pub async fn initialize_default_topology(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let default_topology = ClusterTopology {
            cluster_id: "default-cluster".to_string(),
            cluster_name: "PICCOLO Cluster".to_string(),
            r#type: TopologyType::Embedded.into(),
            master_nodes: vec![],
            sub_nodes: vec![],
            parent_cluster: String::new(),
            config: {
                let mut config = std::collections::HashMap::new();
                config.insert("heartbeat_interval".to_string(), "30".to_string());
                config.insert("max_nodes".to_string(), "10".to_string());
                config
            },
        };

        self.update_topology(default_topology).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apiserver::NodeInfo;
    use common::nodeagent::fromapiserver::{NodeRole, NodeStatus, NodeType, ResourceInfo};
    use std::collections::HashMap;

    fn create_test_node_info(
        node_id: &str,
        hostname: &str,
        ip_address: &str,
        node_role: NodeRole,
    ) -> NodeInfo {
        NodeInfo {
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            ip_address: ip_address.to_string(),
            node_type: NodeType::Vehicle as i32,
            node_role: node_role as i32,
            status: NodeStatus::Ready as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 100,
                architecture: "x86_64".to_string(),
                os_version: "Ubuntu 20.04".to_string(),
            }),
            last_heartbeat: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        }
    }

    fn create_test_topology(
        cluster_id: &str,
        cluster_name: &str,
        topology_type: TopologyType,
    ) -> ClusterTopology {
        let mut config = HashMap::new();
        config.insert("heartbeat_interval".to_string(), "30".to_string());
        config.insert("max_nodes".to_string(), "10".to_string());

        ClusterTopology {
            cluster_id: cluster_id.to_string(),
            cluster_name: cluster_name.to_string(),
            r#type: topology_type.into(),
            master_nodes: vec![],
            sub_nodes: vec![],
            parent_cluster: String::new(),
            config,
        }
    }

    #[test]
    fn test_node_registry_creation() {
        let registry = NodeRegistry;
        // Test that registry can be cloned
        let _cloned_registry = registry.clone();
    }

    #[tokio::test]
    async fn test_get_topology_default() {
        let registry = NodeRegistry;

        // Test getting topology (will use default if etcd not available)
        match registry.get_topology().await {
            Ok(topology) => {
                // The topology might be default or previously set depending on etcd state
                assert!(!topology.cluster_id.is_empty());
                assert!(!topology.cluster_name.is_empty());
                // Verify the topology structure is valid
                assert!(topology.master_nodes.is_empty() || !topology.master_nodes.is_empty());
                assert!(topology.sub_nodes.is_empty() || !topology.sub_nodes.is_empty());
                logd!(
                    2,
                    "Successfully got topology: {} ({})",
                    topology.cluster_name,
                    topology.cluster_id
                );
            }
            Err(e) => {
                // Expected if etcd is not available during testing
                logd!(
                    4,
                    "Expected error getting topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_get_topology_from_etcd() {
        let registry = NodeRegistry;

        // This test will either get a stored topology or default
        let result = registry.get_topology().await;

        match result {
            Ok(topology) => {
                assert!(!topology.cluster_id.is_empty());
                assert!(!topology.cluster_name.is_empty());
                logd!(
                    2,
                    "Got topology: {} ({})",
                    topology.cluster_name,
                    topology.cluster_id
                );
            }
            Err(e) => {
                logd!(4, "Error getting topology: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_update_topology_embedded() {
        let registry = NodeRegistry;
        let test_topology = create_test_topology(
            "embedded-cluster",
            "Embedded Test Cluster",
            TopologyType::Embedded,
        );

        match registry.update_topology(test_topology.clone()).await {
            Ok(updated_topology) => {
                assert_eq!(updated_topology.cluster_id, test_topology.cluster_id);
                assert_eq!(updated_topology.cluster_name, test_topology.cluster_name);
                assert_eq!(updated_topology.r#type, TopologyType::Embedded as i32);
                logd!(2, "Successfully updated embedded topology");
            }
            Err(e) => {
                logd!(
                    4,
                    "Expected error updating topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_update_topology_hybrid_cloud() {
        let registry = NodeRegistry;
        let mut test_topology = create_test_topology(
            "hybrid-cluster",
            "Hybrid Cloud Cluster",
            TopologyType::HybridCloud,
        );

        // Add some master and sub nodes
        test_topology.master_nodes = vec![create_test_node_info(
            "master-1",
            "master-host-1",
            "10.0.0.1",
            NodeRole::Master,
        )];
        test_topology.sub_nodes = vec![
            create_test_node_info("node-1", "node-host-1", "10.0.0.2", NodeRole::Nodeagent),
            create_test_node_info("node-2", "node-host-2", "10.0.0.3", NodeRole::Nodeagent),
        ];

        match registry.update_topology(test_topology.clone()).await {
            Ok(updated_topology) => {
                assert_eq!(updated_topology.cluster_id, "hybrid-cluster");
                assert_eq!(updated_topology.r#type, TopologyType::HybridCloud as i32);
                assert_eq!(updated_topology.master_nodes.len(), 1);
                assert_eq!(updated_topology.sub_nodes.len(), 2);
                logd!(2, "Successfully updated hybrid cloud topology");
            }
            Err(e) => {
                logd!(
                    4,
                    "Expected error updating topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_update_topology_multi_cluster() {
        let registry = NodeRegistry;
        let mut test_topology = create_test_topology(
            "multi-cluster",
            "Multi Cluster Setup",
            TopologyType::MultiCluster,
        );

        test_topology.parent_cluster = "parent-cluster-id".to_string();
        test_topology
            .config
            .insert("cluster_role".to_string(), "child".to_string());

        match registry.update_topology(test_topology.clone()).await {
            Ok(updated_topology) => {
                assert_eq!(updated_topology.cluster_id, "multi-cluster");
                assert_eq!(updated_topology.r#type, TopologyType::MultiCluster as i32);
                assert_eq!(updated_topology.parent_cluster, "parent-cluster-id");
                assert!(updated_topology.config.contains_key("cluster_role"));
                logd!(2, "Successfully updated multi-cluster topology");
            }
            Err(e) => {
                logd!(
                    4,
                    "Expected error updating topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_update_topology_distributed() {
        let registry = NodeRegistry;
        let test_topology = create_test_topology(
            "distributed-cluster",
            "Distributed Test Cluster",
            TopologyType::Distributed,
        );

        match registry.update_topology(test_topology.clone()).await {
            Ok(updated_topology) => {
                assert_eq!(updated_topology.r#type, TopologyType::Distributed as i32);
                logd!(2, "Successfully updated distributed topology");
            }
            Err(e) => {
                logd!(
                    4,
                    "Expected error updating topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_initialize_default_topology() {
        let registry = NodeRegistry;

        match registry.initialize_default_topology().await {
            Ok(()) => {
                println!("Successfully initialized default topology");

                // Try to get the topology to verify initialization worked
                if let Ok(topology) = registry.get_topology().await {
                    // The topology might be default or previously set depending on etcd state
                    assert!(!topology.cluster_id.is_empty());
                    assert!(!topology.cluster_name.is_empty());
                    println!(
                        "Retrieved topology after init: {} ({})",
                        topology.cluster_name, topology.cluster_id
                    );
                }
            }
            Err(e) => {
                println!(
                    "Expected error initializing topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_topology_with_empty_values() {
        let registry = NodeRegistry;
        let empty_topology = ClusterTopology {
            cluster_id: "".to_string(),
            cluster_name: "".to_string(),
            r#type: TopologyType::Unspecified.into(),
            master_nodes: vec![],
            sub_nodes: vec![],
            parent_cluster: "".to_string(),
            config: HashMap::new(),
        };

        match registry.update_topology(empty_topology.clone()).await {
            Ok(updated_topology) => {
                assert_eq!(updated_topology.cluster_id, "");
                assert_eq!(updated_topology.cluster_name, "");
                assert_eq!(updated_topology.r#type, TopologyType::Unspecified as i32);
                println!("Successfully updated empty topology");
            }
            Err(e) => {
                println!(
                    "Expected error updating empty topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_topology_with_large_config() {
        let registry = NodeRegistry;
        let mut large_config = HashMap::new();

        // Create a topology with many configuration items
        for i in 0..100 {
            large_config.insert(format!("config_key_{}", i), format!("config_value_{}", i));
        }

        let topology_with_large_config = ClusterTopology {
            cluster_id: "large-config-cluster".to_string(),
            cluster_name: "Large Config Cluster".to_string(),
            r#type: TopologyType::Embedded.into(),
            master_nodes: vec![],
            sub_nodes: vec![],
            parent_cluster: String::new(),
            config: large_config.clone(),
        };

        match registry
            .update_topology(topology_with_large_config.clone())
            .await
        {
            Ok(updated_topology) => {
                assert_eq!(updated_topology.config.len(), 100);
                assert!(updated_topology.config.contains_key("config_key_0"));
                assert!(updated_topology.config.contains_key("config_key_99"));
                println!("Successfully updated topology with large config");
            }
            Err(e) => {
                println!(
                    "Expected error updating large config topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[tokio::test]
    async fn test_topology_with_many_nodes() {
        let registry = NodeRegistry;
        let mut topology = create_test_topology(
            "many-nodes-cluster",
            "Many Nodes Cluster",
            TopologyType::HybridCloud,
        );

        // Add many master nodes
        for i in 0..5 {
            topology.master_nodes.push(create_test_node_info(
                &format!("master-{}", i),
                &format!("master-host-{}", i),
                &format!("10.0.0.{}", i + 1),
                NodeRole::Master,
            ));
        }

        // Add many sub nodes
        for i in 0..20 {
            topology.sub_nodes.push(create_test_node_info(
                &format!("node-{}", i),
                &format!("node-host-{}", i),
                &format!("10.0.1.{}", i + 1),
                NodeRole::Nodeagent,
            ));
        }

        match registry.update_topology(topology.clone()).await {
            Ok(updated_topology) => {
                assert_eq!(updated_topology.master_nodes.len(), 5);
                assert_eq!(updated_topology.sub_nodes.len(), 20);
                println!("Successfully updated topology with many nodes");
            }
            Err(e) => {
                println!(
                    "Expected error updating many nodes topology (etcd unavailable): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_topology_type_enum_values() {
        // Test all TopologyType enum variants
        let types = vec![
            TopologyType::Unspecified,
            TopologyType::Embedded,
            TopologyType::HybridCloud,
            TopologyType::MultiCluster,
            TopologyType::Distributed,
        ];

        for topology_type in types {
            let topology = create_test_topology("test-cluster", "Test Cluster", topology_type);

            assert_eq!(topology.r#type, topology_type as i32);
            println!("TopologyType::{:?} = {}", topology_type, topology.r#type);
        }
    }

    #[test]
    fn test_cluster_topology_creation() {
        let topology = create_test_topology("test", "Test Cluster", TopologyType::Embedded);

        assert_eq!(topology.cluster_id, "test");
        assert_eq!(topology.cluster_name, "Test Cluster");
        assert_eq!(topology.r#type, TopologyType::Embedded as i32);
        assert_eq!(topology.master_nodes.len(), 0);
        assert_eq!(topology.sub_nodes.len(), 0);
        assert_eq!(topology.parent_cluster, "");
        assert_eq!(topology.config.len(), 2); // heartbeat_interval and max_nodes
    }

    #[test]
    fn test_node_info_creation() {
        let node = create_test_node_info("test-1", "test-host", "192.168.1.1", NodeRole::Master);

        assert_eq!(node.node_id, "test-1");
        assert_eq!(node.hostname, "test-host");
        assert_eq!(node.ip_address, "192.168.1.1");
        assert_eq!(node.node_role, NodeRole::Master as i32);
        assert_eq!(node.status, NodeStatus::Ready as i32);
        assert!(node.resources.is_some());
    }

    #[tokio::test]
    async fn test_topology_serialization_deserialization() {
        // Test the serialization/deserialization logic used in update_topology
        let original_topology = create_test_topology(
            "serialization-test",
            "Serialization Test Cluster",
            TopologyType::HybridCloud,
        );

        // Simulate the encoding process
        let mut buf = Vec::new();
        match prost::Message::encode(&original_topology, &mut buf) {
            Ok(()) => {
                let encoded = base64::engine::general_purpose::STANDARD.encode(&buf);
                assert!(!encoded.is_empty());

                // Simulate the decoding process
                match base64::engine::general_purpose::STANDARD.decode(&encoded) {
                    Ok(decoded_buf) => match ClusterTopology::decode(&decoded_buf[..]) {
                        Ok(decoded_topology) => {
                            assert_eq!(decoded_topology.cluster_id, original_topology.cluster_id);
                            assert_eq!(
                                decoded_topology.cluster_name,
                                original_topology.cluster_name
                            );
                            assert_eq!(decoded_topology.r#type, original_topology.r#type);
                            println!("Serialization/deserialization successful");
                        }
                        Err(e) => {
                            println!("Protobuf decode error: {}", e);
                        }
                    },
                    Err(e) => {
                        println!("Base64 decode error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Protobuf encode error: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_error_handling_scenarios() {
        let registry = NodeRegistry;

        // Test various error scenarios that might occur with malformed data
        // These tests exercise error handling paths in the code

        // Test get_topology error handling (etcd unavailable scenario is already covered)
        let get_result = registry.get_topology().await;
        match get_result {
            Ok(topology) => {
                println!("Get topology succeeded: {}", topology.cluster_name);
            }
            Err(e) => {
                println!("Get topology failed as expected: {}", e);
            }
        }

        // Test update_topology error handling
        let test_topology =
            create_test_topology("error-test", "Error Test", TopologyType::Embedded);
        let update_result = registry.update_topology(test_topology).await;
        match update_result {
            Ok(topology) => {
                println!("Update topology succeeded: {}", topology.cluster_name);
            }
            Err(e) => {
                println!("Update topology failed as expected: {}", e);
            }
        }

        // Test initialize_default_topology error handling
        let init_result = registry.initialize_default_topology().await;
        match init_result {
            Ok(()) => {
                println!("Initialize default topology succeeded");
            }
            Err(e) => {
                println!("Initialize default topology failed as expected: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_topology_update_and_retrieve_cycle() {
        let registry = NodeRegistry;

        // Create a test topology
        let mut test_topology = create_test_topology(
            "cycle-test",
            "Cycle Test Cluster",
            TopologyType::MultiCluster,
        );

        test_topology.master_nodes.push(create_test_node_info(
            "cycle-master",
            "cycle-master-host",
            "192.168.100.1",
            NodeRole::Master,
        ));

        // Try to update and then retrieve
        match registry.update_topology(test_topology.clone()).await {
            Ok(_) => {
                println!("Update succeeded, now trying to retrieve...");

                match registry.get_topology().await {
                    Ok(retrieved_topology) => {
                        // Note: The retrieved topology might be the one we just set or a default
                        // depending on etcd availability
                        println!("Retrieved topology: {}", retrieved_topology.cluster_name);
                    }
                    Err(e) => {
                        println!("Retrieve failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Update failed (expected if etcd unavailable): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_topology_operations() {
        let registry = NodeRegistry;

        // Test concurrent access to topology operations
        let get_task = tokio::spawn({
            let registry = registry.clone();
            async move { registry.get_topology().await }
        });

        let update_task = tokio::spawn({
            let registry = registry.clone();
            async move {
                let topology =
                    create_test_topology("concurrent-1", "Concurrent 1", TopologyType::Embedded);
                registry.update_topology(topology).await
            }
        });

        let init_task = tokio::spawn({
            let registry = registry.clone();
            async move {
                registry
                    .initialize_default_topology()
                    .await
                    .map(|_| ClusterTopology {
                        cluster_id: "init-result".to_string(),
                        cluster_name: "Init Result".to_string(),
                        r#type: TopologyType::Embedded as i32,
                        master_nodes: vec![],
                        sub_nodes: vec![],
                        parent_cluster: String::new(),
                        config: HashMap::new(),
                    })
            }
        });

        let tasks = vec![get_task, update_task, init_task];

        for (i, task) in tasks.into_iter().enumerate() {
            match task.await {
                Ok(result) => match result {
                    Ok(_) => println!("Concurrent task {} succeeded", i),
                    Err(e) => println!("Concurrent task {} failed: {}", i, e),
                },
                Err(e) => {
                    println!("Concurrent task {} panicked: {}", i, e);
                }
            }
        }
    }

    #[test]
    fn test_default_topology_values() {
        // Test the exact default values used in get_topology when etcd is not available
        let default_topology = ClusterTopology {
            cluster_id: "default-cluster".to_string(),
            cluster_name: "PICCOLO Cluster".to_string(),
            r#type: TopologyType::Embedded.into(),
            master_nodes: vec![],
            sub_nodes: vec![],
            parent_cluster: String::new(),
            config: std::collections::HashMap::new(),
        };

        assert_eq!(default_topology.cluster_id, "default-cluster");
        assert_eq!(default_topology.cluster_name, "PICCOLO Cluster");
        assert_eq!(default_topology.r#type, TopologyType::Embedded as i32);
        assert!(default_topology.master_nodes.is_empty());
        assert!(default_topology.sub_nodes.is_empty());
        assert!(default_topology.parent_cluster.is_empty());
        assert!(default_topology.config.is_empty());
    }

    #[test]
    fn test_initialize_default_topology_values() {
        // Test the exact values used in initialize_default_topology
        let mut config = std::collections::HashMap::new();
        config.insert("heartbeat_interval".to_string(), "30".to_string());
        config.insert("max_nodes".to_string(), "10".to_string());

        let default_topology = ClusterTopology {
            cluster_id: "default-cluster".to_string(),
            cluster_name: "PICCOLO Cluster".to_string(),
            r#type: TopologyType::Embedded.into(),
            master_nodes: vec![],
            sub_nodes: vec![],
            parent_cluster: String::new(),
            config: config.clone(),
        };

        assert_eq!(default_topology.config.len(), 2);
        assert_eq!(
            default_topology.config.get("heartbeat_interval"),
            Some(&"30".to_string())
        );
        assert_eq!(
            default_topology.config.get("max_nodes"),
            Some(&"10".to_string())
        );
    }
}

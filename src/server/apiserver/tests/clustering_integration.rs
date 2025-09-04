/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Integration tests for clustering functionality
//! 
//! Tests the basic clustering features including node registration,
//! heartbeat, and status management between API server and NodeAgent.

#[cfg(test)]
mod tests {
    use apiserver::node::NodeManager;
    use common::nodeagent::{
        NodeRegistrationRequest, NodeRole, NodeStatus,
        ResourceInfo, Credentials, HeartbeatRequest
    };
    use common::apiserver::{GetNodesRequest, HealthCheckRequest};
    
    #[tokio::test]
    async fn test_basic_node_registration() {
        // Create a node manager (API server side)
        let node_manager = NodeManager::new();
        node_manager.initialize().await.unwrap();

        // Create a node registration request
        let request = NodeRegistrationRequest {
            node_id: "test-node-1".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            role: NodeRole::Sub.into(),
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 2048,
                disk_gb: 100,
                architecture: "x86_64".to_string(),
                platform: "linux".to_string(),
            }),
            credentials: Some(Credentials {
                token: "test-token".to_string(),
                certificate: "".to_string(),
                expires_at: chrono::Utc::now().timestamp() + 3600,
            }),
        };

        // Register the node
        let response = node_manager.register_node(request).await.unwrap();
        assert_eq!(response.node_id, "test-node-1");
        assert!(response.cluster_info.is_some());

        // Verify the node appears in the node list
        let nodes_response = node_manager.get_nodes(GetNodesRequest {
            filter: None,
            status_filter: None,
            role_filter: None,
        }).await.unwrap();
        
        assert!(nodes_response.success);
        assert_eq!(nodes_response.nodes.len(), 1);
        assert_eq!(nodes_response.nodes[0].node_id, "test-node-1");
        assert_eq!(nodes_response.nodes[0].hostname, "test-host");
    }

    #[tokio::test]
    async fn test_heartbeat_and_status_update() {
        let node_manager = NodeManager::new();
        node_manager.initialize().await.unwrap();

        // First register a node
        let request = NodeRegistrationRequest {
            node_id: "heartbeat-test".to_string(),
            hostname: "heartbeat-host".to_string(),
            ip_address: "192.168.1.101".to_string(),
            role: NodeRole::Sub.into(),
            resources: None,
            credentials: None,
        };

        node_manager.register_node(request).await.unwrap();

        // Send a heartbeat
        let heartbeat = HeartbeatRequest {
            node_id: "heartbeat-test".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            status: NodeStatus::Ready.into(),
        };

        let heartbeat_response = node_manager.process_heartbeat(heartbeat).await.unwrap();
        assert!(heartbeat_response.success);

        // Verify the node status was updated
        let node_response = node_manager.get_node(common::apiserver::GetNodeRequest {
            node_id: "heartbeat-test".to_string(),
        }).await.unwrap();

        assert!(node_response.success);
        let node = node_response.node.unwrap();
        assert_eq!(node.status, NodeStatus::Ready as i32);
    }

    #[tokio::test]
    async fn test_cluster_health_check() {
        let node_manager = NodeManager::new();
        node_manager.initialize().await.unwrap();

        // Add a few test nodes
        for i in 1..=3 {
            let request = NodeRegistrationRequest {
                node_id: format!("health-test-{}", i),
                hostname: format!("host-{}", i),
                ip_address: format!("192.168.1.{}", 100 + i),
                role: NodeRole::Sub.into(),
                resources: None,
                credentials: None,
            };
            node_manager.register_node(request).await.unwrap();
        }

        // Check cluster health
        let health_response = node_manager.health_check(HealthCheckRequest {}).await.unwrap();
        
        // Should show the nodes in the cluster
        assert!(health_response.details.contains_key("total_nodes"));
        assert!(health_response.details.contains_key("cluster_id"));
        
        let total_nodes = health_response.details.get("total_nodes").unwrap();
        assert_eq!(total_nodes, "3");
    }

    #[tokio::test]
    async fn test_multiple_node_registration() {
        let node_manager = NodeManager::new();
        node_manager.initialize().await.unwrap();

        // Register multiple nodes
        let nodes = vec![
            ("edge-node-1", "edge-host-1", "192.168.1.10"),
            ("edge-node-2", "edge-host-2", "192.168.1.11"),
            ("edge-node-3", "edge-host-3", "192.168.1.12"),
        ];

        for (node_id, hostname, ip) in nodes {
            let request = NodeRegistrationRequest {
                node_id: node_id.to_string(),
                hostname: hostname.to_string(),
                ip_address: ip.to_string(),
                role: NodeRole::Sub.into(),
                resources: Some(ResourceInfo {
                    cpu_cores: 2,
                    memory_mb: 1024,
                    disk_gb: 50,
                    architecture: "arm64".to_string(),
                    platform: "linux".to_string(),
                }),
                credentials: None,
            };

            let response = node_manager.register_node(request).await.unwrap();
            assert_eq!(response.node_id, node_id);
        }

        // Verify all nodes are registered
        let nodes_response = node_manager.get_nodes(GetNodesRequest {
            filter: None,
            status_filter: None,
            role_filter: None,
        }).await.unwrap();

        assert!(nodes_response.success);
        assert_eq!(nodes_response.nodes.len(), 3);

        // Check that we can filter by role
        let filtered_response = node_manager.get_nodes(GetNodesRequest {
            filter: None,
            status_filter: None,
            role_filter: Some(NodeRole::Sub.into()),
        }).await.unwrap();

        assert!(filtered_response.success);
        assert_eq!(filtered_response.nodes.len(), 3);
    }

    #[tokio::test]
    async fn test_cluster_topology() {
        let node_manager = NodeManager::new();
        node_manager.initialize().await.unwrap();

        // Register a master node and sub nodes
        let master_request = NodeRegistrationRequest {
            node_id: "master-1".to_string(),
            hostname: "master-host".to_string(),
            ip_address: "192.168.1.1".to_string(),
            role: NodeRole::Master.into(),
            resources: None,
            credentials: None,
        };
        node_manager.register_node(master_request).await.unwrap();

        let sub_request = NodeRegistrationRequest {
            node_id: "sub-1".to_string(),
            hostname: "sub-host".to_string(),
            ip_address: "192.168.1.2".to_string(),
            role: NodeRole::Sub.into(),
            resources: None,
            credentials: None,
        };
        node_manager.register_node(sub_request).await.unwrap();

        // Get topology
        let topology_response = node_manager.get_topology(
            common::apiserver::GetTopologyRequest {}
        ).await.unwrap();

        assert!(topology_response.success);
        let topology = topology_response.topology.unwrap();
        assert_eq!(topology.cluster_name, "PICCOLO Embedded Cluster");
        assert_eq!(topology.master_nodes.len(), 1);
        assert_eq!(topology.sub_nodes.len(), 1);
        assert_eq!(topology.master_nodes[0].node_id, "master-1");
        assert_eq!(topology.sub_nodes[0].node_id, "sub-1");
    }
}
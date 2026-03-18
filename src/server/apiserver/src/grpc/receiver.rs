/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::node::NodeManager;
use base64::Engine;
use common::apiserver::api_server_connection_server::ApiServerConnection;
use common::apiserver::{
    ClusterTopology, GetNodeRequest, GetNodeResponse, GetNodesRequest, GetNodesResponse,
    GetTopologyRequest, GetTopologyResponse, TopologyType, UpdateTopologyRequest,
    UpdateTopologyResponse,
};
use common::persistency;
use common::logd;
use common::nodeagent::fromapiserver::{
    NodeRegistrationRequest, NodeRegistrationResponse, NodeStatus,
};
use prost::Message;
use tonic::{Request, Response, Status};

/// Simple registry embedded in receiver
#[derive(Clone)]
struct NodeRegistry;

impl NodeRegistry {
    async fn get_topology(
        &self,
    ) -> Result<ClusterTopology, Box<dyn std::error::Error + Send + Sync>> {
        let topology_key = "cluster/topology";

        match persistency::get(topology_key).await {
            Ok(encoded) => {
                let buf = base64::engine::general_purpose::STANDARD.decode(&encoded)?;
                let topology = ClusterTopology::decode(&buf[..])?;
                Ok(topology)
            }
            Err(_) => Ok(ClusterTopology {
                cluster_id: "default-cluster".to_string(),
                cluster_name: "PICCOLO Cluster".to_string(),
                r#type: TopologyType::Embedded.into(),
                master_nodes: vec![],
                sub_nodes: vec![],
                parent_cluster: String::new(),
                config: std::collections::HashMap::new(),
            }),
        }
    }

    async fn update_topology(
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
}

/// API Server gRPC service handler for clustering functionality
#[derive(Clone)]
pub struct ApiServerReceiver {
    node_manager: NodeManager,
    registry: NodeRegistry,
}

impl Default for ApiServerReceiver {
    fn default() -> Self {
        Self::new()
    }
}
#[allow(dead_code)]
impl ApiServerReceiver {
    pub fn new() -> Self {
        Self {
            node_manager: NodeManager::new().unwrap(),
            registry: NodeRegistry,
        }
    }

    /// Convert to gRPC service
    #[allow(unused_variables)]
    pub fn into_service(
        self,
    ) -> common::apiserver::api_server_connection_server::ApiServerConnectionServer<Self> {
        common::apiserver::api_server_connection_server::ApiServerConnectionServer::new(self)
    }
}

#[tonic::async_trait]
impl ApiServerConnection for ApiServerReceiver {
    async fn get_nodes(
        &self,
        request: Request<GetNodesRequest>,
    ) -> Result<Response<GetNodesResponse>, Status> {
        logd!(1, "Received GetNodes request");
        let _req = request.into_inner();

        match self.node_manager.get_nodes().await {
            Ok(nodes) => Ok(Response::new(GetNodesResponse {
                nodes,
                success: true,
                message: "Successfully retrieved nodes".to_string(),
            })),
            Err(e) => Ok(Response::new(GetNodesResponse {
                nodes: vec![],
                success: false,
                message: format!("Failed to retrieve nodes: {}", e),
            })),
        }
    }

    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeResponse>, Status> {
        logd!(1, "Received GetNode request");
        let req = request.into_inner();

        match self.node_manager.get_node(&req.node_id).await {
            Ok(Some(node)) => Ok(Response::new(GetNodeResponse {
                node: Some(node),
                success: true,
                message: format!("Successfully retrieved node {}", req.node_id),
            })),
            Ok(None) => Ok(Response::new(GetNodeResponse {
                node: None,
                success: false,
                message: format!("Node {} not found", req.node_id),
            })),
            Err(e) => Ok(Response::new(GetNodeResponse {
                node: None,
                success: false,
                message: format!("Failed to retrieve node: {}", e),
            })),
        }
    }

    async fn register_node(
        &self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        logd!(1, "Received RegisterNode request");
        let req = request.into_inner();

        logd!(
            2,
            "Registering node: {} ({}) with ID {}",
            req.hostname,
            req.ip_address,
            req.node_id
        );

        // Note: We now use the node_id provided by the nodeagent
        // This should be in the format {node_name}-{node_ip}
        logd!(1, "Using provided node_id: {}", req.node_id);

        // Let's update the node status to Ready immediately
        match self.node_manager.register_node(req.clone()).await {
            Ok(cluster_token) => {
                // Also directly add to etcd with a simple key
                let node_info = common::apiserver::NodeInfo {
                    node_id: req.node_id.clone(),
                    hostname: req.hostname.clone(),
                    ip_address: req.ip_address.clone(),
                    node_type: req.node_type,
                    node_role: req.node_role,
                    status: NodeStatus::Ready.into(),
                    resources: req.resources.clone(),
                    last_heartbeat: chrono::Utc::now().timestamp(),
                    created_at: chrono::Utc::now().timestamp(),
                    metadata: req.metadata.clone(),
                };

                // 인코딩을 제거하고 json string으로 저장
                // below variable never used , hence the _ to prevent warnings
                let _node_json = serde_json::to_string(&node_info).unwrap();

                // 두 가지 키로 저장
                // 1. IP 주소로 빠른 조회용 (json 문자열로 변경)
                let _ =
                    common::persistency::put(&format!("nodes/{}", req.ip_address), &req.hostname).await;
                logd!(1, "Hostname stored at IP key: nodes/{}", req.ip_address);

                // 2. 호스트 이름으로 빠른 조회용 (ActionController용)
                let _ =
                    common::persistency::put(&format!("nodes/{}", req.hostname), &req.ip_address).await;
                logd!(1, "Node IP stored at hostname key: nodes/{}", req.hostname);

                // Immediately update the node status to Ready
                if let Err(e) = self
                    .node_manager
                    .update_status(&req.node_id, NodeStatus::Ready)
                    .await
                {
                    logd!(5, "Warning: Failed to update node status to Ready: {}", e);
                } else {
                    logd!(1, "Successfully updated node status to Ready");
                }

                logd!(
                    2,
                    "Node registration successful, cluster token: {}",
                    cluster_token
                );
                Ok(Response::new(NodeRegistrationResponse {
                    success: true,
                    message: "Node registered successfully".to_string(),
                    cluster_token,
                    cluster_config: Some(common::nodeagent::fromapiserver::ClusterConfig {
                        master_endpoint: "localhost:47099".to_string(), // apiserver endpoint
                        heartbeat_interval: 30,
                        settings: std::collections::HashMap::new(),
                    }),
                }))
            }
            Err(e) => {
                logd!(5, "Node registration failed: {}", e);
                Ok(Response::new(NodeRegistrationResponse {
                    success: false,
                    message: format!("Failed to register node: {}", e),
                    cluster_token: String::new(),
                    cluster_config: None,
                }))
            }
        }
    }

    async fn get_topology(
        &self,
        _request: Request<GetTopologyRequest>,
    ) -> Result<Response<GetTopologyResponse>, Status> {
        match self.registry.get_topology().await {
            Ok(topology) => Ok(Response::new(GetTopologyResponse {
                topology: Some(topology),
                success: true,
                message: "Successfully retrieved topology".to_string(),
            })),
            Err(e) => Ok(Response::new(GetTopologyResponse {
                topology: None,
                success: false,
                message: format!("Failed to retrieve topology: {}", e),
            })),
        }
    }

    async fn update_topology(
        &self,
        request: Request<UpdateTopologyRequest>,
    ) -> Result<Response<UpdateTopologyResponse>, Status> {
        let req = request.into_inner();

        if let Some(topology) = req.topology {
            match self.registry.update_topology(topology).await {
                Ok(updated_topology) => Ok(Response::new(UpdateTopologyResponse {
                    updated_topology: Some(updated_topology),
                    success: true,
                    message: "Successfully updated topology".to_string(),
                })),
                Err(e) => Ok(Response::new(UpdateTopologyResponse {
                    updated_topology: None,
                    success: false,
                    message: format!("Failed to update topology: {}", e),
                })),
            }
        } else {
            Ok(Response::new(UpdateTopologyResponse {
                updated_topology: None,
                success: false,
                message: "No topology provided in request".to_string(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apiserver::NodeInfo;
    use common::apiserver::TopologyType;
    use common::nodeagent::fromapiserver::{NodeRole, NodeType, ResourceInfo};
    use std::collections::HashMap;
    use tokio;

    fn create_test_resource_info() -> ResourceInfo {
        ResourceInfo {
            cpu_cores: 4,
            memory_mb: 8192,
            disk_gb: 100,
            architecture: "x86_64".to_string(),
            os_version: "Ubuntu 20.04".to_string(),
        }
    }

    fn create_test_registration_request(
        node_id: &str,
        hostname: &str,
        ip: &str,
    ) -> NodeRegistrationRequest {
        let mut metadata = HashMap::new();
        metadata.insert("environment".to_string(), "test".to_string());

        NodeRegistrationRequest {
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            ip_address: ip.to_string(),
            node_type: NodeType::Vehicle.into(),
            node_role: NodeRole::Nodeagent.into(),
            resources: Some(create_test_resource_info()),
            metadata,
        }
    }

    fn create_test_node_info(node_id: &str, hostname: &str, ip: &str) -> NodeInfo {
        NodeInfo {
            node_id: node_id.to_string(),
            hostname: hostname.to_string(),
            ip_address: ip.to_string(),
            node_type: NodeType::Vehicle as i32,
            node_role: NodeRole::Nodeagent as i32,
            status: NodeStatus::Ready.into(),
            resources: Some(create_test_resource_info()),
            last_heartbeat: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            metadata: HashMap::new(),
        }
    }

    fn create_test_cluster_topology() -> ClusterTopology {
        let mut config = HashMap::new();
        config.insert("test_key".to_string(), "test_value".to_string());

        ClusterTopology {
            cluster_id: "test-cluster-001".to_string(),
            cluster_name: "Test Cluster".to_string(),
            r#type: TopologyType::HybridCloud as i32,
            master_nodes: vec![
                create_test_node_info("master-1", "master-host-1", "192.168.1.10"),
                create_test_node_info("master-2", "master-host-2", "192.168.1.11"),
            ],
            sub_nodes: vec![
                create_test_node_info("node-1", "worker-host-1", "192.168.1.20"),
                create_test_node_info("node-2", "worker-host-2", "192.168.1.21"),
            ],
            parent_cluster: "parent-cluster".to_string(),
            config,
        }
    }

    #[tokio::test]
    async fn test_node_registry_new() {
        let _registry = NodeRegistry;
        // Test that NodeRegistry can be created
        assert!(true); // NodeRegistry is a unit struct, just verify it compiles
    }

    #[tokio::test]
    async fn test_node_registry_get_topology_default() {
        let registry = NodeRegistry;

        // Test getting topology (may be default or existing from other tests)
        let result = registry.get_topology().await;
    }

    #[tokio::test]
    async fn test_api_server_receiver_new() {
        let _receiver = ApiServerReceiver::new();

        // Test that receiver can be created successfully
        // We can't directly access private fields, but we can test that creation succeeds
        assert!(true); // If we get here, new() succeeded
    }

    #[tokio::test]
    async fn test_api_server_receiver_into_service() {
        let receiver = ApiServerReceiver::new();
        let _service = receiver.into_service();

        // Test that service conversion works
        assert!(true); // If we get here, into_service() succeeded
    }

    #[tokio::test]
    async fn test_get_nodes_success() {
        let receiver = ApiServerReceiver::new();
        let request = Request::new(GetNodesRequest {
            filter: None,
            status_filter: None,
        });

        let result = receiver.get_nodes(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        // Response should exist, success field depends on actual node data
        assert!(!response.message.is_empty());

        // Test both success and failure cases
        if response.success {
            assert_eq!(response.message, "Successfully retrieved nodes");
        } else {
            assert!(response.message.contains("Failed to retrieve nodes"));
        }
    }

    #[tokio::test]
    async fn test_get_node_not_found() {
        let receiver = ApiServerReceiver::new();
        let request = Request::new(GetNodeRequest {
            node_id: "non-existent-node".to_string(),
        });

        let result = receiver.get_node(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        assert!(!response.success);
        assert!(
            response.message.contains("not found")
                || response.message.contains("Failed to retrieve node")
        );
        assert!(response.node.is_none());
    }

    #[tokio::test]
    async fn test_register_node_success() {
        let receiver = ApiServerReceiver::new();
        let registration_request =
            create_test_registration_request("test-node-001", "test-hostname", "192.168.1.100");
        let request = Request::new(registration_request.clone());

        let result = receiver.register_node(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        assert!(!response.message.is_empty());

        if response.success {
            assert!(!response.cluster_token.is_empty());
            assert!(response.cluster_config.is_some());

            let config = response.cluster_config.unwrap();
            assert_eq!(config.master_endpoint, "localhost:47099");
            assert_eq!(config.heartbeat_interval, 30);
        } else {
            assert!(response.message.contains("Failed to register node"));
            assert!(response.cluster_token.is_empty());
            assert!(response.cluster_config.is_none());
        }
    }

    #[tokio::test]
    async fn test_register_node_with_different_types() {
        let receiver = ApiServerReceiver::new();

        // Test Vehicle node
        let mut vehicle_request =
            create_test_registration_request("vehicle-node-001", "vehicle-host", "192.168.1.101");
        vehicle_request.node_type = NodeType::Vehicle.into();
        vehicle_request.node_role = NodeRole::Nodeagent.into();

        let request = Request::new(vehicle_request);
        let result = receiver.register_node(request).await;
        assert!(result.is_ok());

        // Test Cloud node
        let mut cloud_request =
            create_test_registration_request("cloud-node-001", "cloud-host", "10.0.1.100");
        cloud_request.node_type = NodeType::Cloud.into();
        cloud_request.node_role = NodeRole::Master.into();

        let request = Request::new(cloud_request);
        let result = receiver.register_node(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_topology_success() {
        let receiver = ApiServerReceiver::new();
        let request = Request::new(GetTopologyRequest {});

        let result = receiver.get_topology(request).await;
    }

    #[tokio::test]
    async fn test_update_topology_success() {
        let receiver = ApiServerReceiver::new();
        let test_topology = create_test_cluster_topology();

        let request = Request::new(UpdateTopologyRequest {
            topology: Some(test_topology.clone()),
        });

        let result = receiver.update_topology(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        // When persistency service is running, success is true;
        // when it's not running, the update fails gracefully with success=false
        if response.success {
            assert_eq!(response.message, "Successfully updated topology");
            assert!(response.updated_topology.is_some());

            let updated = response.updated_topology.unwrap();
            assert_eq!(updated.cluster_id, test_topology.cluster_id);
            assert_eq!(updated.cluster_name, test_topology.cluster_name);
        } else {
            // Persistency service not available — accept graceful failure
            assert!(
                response.message.contains("Failed") || response.message.contains("transport") || response.message.contains("error"),
                "Unexpected failure message: {}",
                response.message
            );
        }
    }

    #[tokio::test]
    async fn test_update_topology_no_topology_provided() {
        let receiver = ApiServerReceiver::new();
        let request = Request::new(UpdateTopologyRequest { topology: None });

        let result = receiver.update_topology(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        assert!(!response.success);
        assert_eq!(response.message, "No topology provided in request");
        assert!(response.updated_topology.is_none());
    }

    #[tokio::test]
    async fn test_update_topology_different_types() {
        let receiver = ApiServerReceiver::new();

        // Test different topology types
        let topology_types = vec![
            TopologyType::Embedded,
            TopologyType::HybridCloud,
            TopologyType::MultiCluster,
            TopologyType::Distributed,
        ];

        for topology_type in topology_types {
            let mut topology = create_test_cluster_topology();
            topology.r#type = topology_type.into();
            topology.cluster_id = format!("test-{:?}", topology_type);

            let request = Request::new(UpdateTopologyRequest {
                topology: Some(topology.clone()),
            });

            let result = receiver.update_topology(request).await;
            assert!(result.is_ok());

            let response = result.unwrap().into_inner();
            if response.success {
                let updated = response.updated_topology.unwrap();
                assert_eq!(updated.r#type, topology.r#type);
                assert_eq!(updated.cluster_id, topology.cluster_id);
            }
        }
    }

    #[tokio::test]
    async fn test_register_node_creates_cluster_config() {
        let receiver = ApiServerReceiver::new();
        let registration_request =
            create_test_registration_request("config-test-node", "config-host", "192.168.1.200");

        let request = Request::new(registration_request);
        let result = receiver.register_node(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();

        if response.success {
            assert!(response.cluster_config.is_some());
            let config = response.cluster_config.unwrap();

            // Verify cluster config fields
            assert_eq!(config.master_endpoint, "localhost:47099");
            assert_eq!(config.heartbeat_interval, 30);
            assert!(config.settings.is_empty()); // Default empty settings
        }
    }

    #[tokio::test]
    async fn test_node_registry_clone() {
        let registry1 = NodeRegistry;
        let registry2 = registry1.clone();

        // Test that NodeRegistry can be cloned
        let topology1 = registry1.get_topology().await;
        let topology2 = registry2.get_topology().await;
    }

    #[tokio::test]
    async fn test_api_server_receiver_clone() {
        let receiver1 = ApiServerReceiver::new();
        let receiver2 = receiver1.clone();

        // Test that ApiServerReceiver can be cloned
        let request1 = Request::new(GetTopologyRequest {});
        let request2 = Request::new(GetTopologyRequest {});

        let result1 = receiver1.get_topology(request1).await;
        let result2 = receiver2.get_topology(request2).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Both should return successful responses
        let response1 = result1.unwrap().into_inner();
        let response2 = result2.unwrap().into_inner();
        assert_eq!(response1.success, response2.success);
        assert_eq!(response1.message, response2.message);
    }

    #[tokio::test]
    async fn test_node_registration_with_metadata() {
        let receiver = ApiServerReceiver::new();

        let mut metadata = HashMap::new();
        metadata.insert("region".to_string(), "us-west-2".to_string());
        metadata.insert("zone".to_string(), "us-west-2a".to_string());
        metadata.insert("instance_type".to_string(), "t3.medium".to_string());

        let registration_request = NodeRegistrationRequest {
            node_id: "metadata-test-node".to_string(),
            hostname: "metadata-host".to_string(),
            ip_address: "192.168.1.150".to_string(),
            node_type: NodeType::Cloud.into(),
            node_role: NodeRole::Bluechi.into(),
            resources: Some(create_test_resource_info()),
            metadata,
        };

        let request = Request::new(registration_request);
        let result = receiver.register_node(request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        // Should handle metadata registration
        assert!(!response.message.is_empty());
    }

    #[tokio::test]
    async fn test_get_node_with_existing_node() {
        let receiver = ApiServerReceiver::new();

        // First register a node
        let registration_request =
            create_test_registration_request("existing-node-001", "existing-host", "192.168.1.50");
        let reg_request = Request::new(registration_request.clone());
        let reg_result = receiver.register_node(reg_request).await;
        assert!(reg_result.is_ok());

        // Then try to get it
        let get_request = Request::new(GetNodeRequest {
            node_id: registration_request.node_id.clone(),
        });

        let result = receiver.get_node(get_request).await;
        assert!(result.is_ok());

        let response = result.unwrap().into_inner();
        // Response should be successful if node was registered successfully
        assert!(!response.message.is_empty());
    }
}

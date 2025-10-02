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
use common::etcd;
use common::nodeagent::{NodeRegistrationRequest, NodeRegistrationResponse, NodeStatus};
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

        match etcd::get(topology_key).await {
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
        
        etcd::put(topology_key, &topology_json).await?;

        println!("Updated cluster topology: {}", topology.cluster_name);
        Ok(topology)
    }
}

/// API Server gRPC service handler for clustering functionality
#[derive(Clone)]
pub struct ApiServerReceiver {
    node_manager: NodeManager,
    registry: NodeRegistry,
}

impl ApiServerReceiver {
    pub fn new() -> Self {
        Self {
            node_manager: NodeManager::new().unwrap(),
            registry: NodeRegistry,
        }
    }

    /// Convert to gRPC service
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
        println!("Received GetNodes request");
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
        println!("Received GetNode request");
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
        println!("Received RegisterNode request");
        let req = request.into_inner();

        println!(
            "Registering node: {} ({}) with ID {}",
            req.hostname, req.ip_address, req.node_id
        );

        // Note: We now use the node_id provided by the nodeagent
        // This should be in the format {node_name}-{node_ip}
        println!("Using provided node_id: {}", req.node_id);

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
                let node_json = serde_json::to_string(&node_info).unwrap();

                // 두 가지 키로 저장
                // 1. IP 주소로 빠른 조회용 (json 문자열로 변경)
                let _ = common::etcd::put(&format!("nodes/{}", req.ip_address), &req.hostname).await;
                println!("Hostname stored at IP key: nodes/{}", req.ip_address);

                // 2. 호스트 이름으로 빠른 조회용 (ActionController용)
                let _ =
                    common::etcd::put(&format!("nodes/{}", req.hostname), &req.ip_address).await;
                println!("Node IP stored at hostname key: nodes/{}", req.hostname);

                // Immediately update the node status to Ready
                if let Err(e) = self
                    .node_manager
                    .update_status(&req.node_id, NodeStatus::Ready)
                    .await
                {
                    println!("Warning: Failed to update node status to Ready: {}", e);
                } else {
                    println!("Successfully updated node status to Ready");
                }

                println!(
                    "Node registration successful, cluster token: {}",
                    cluster_token
                );
                Ok(Response::new(NodeRegistrationResponse {
                    success: true,
                    message: "Node registered successfully".to_string(),
                    cluster_token,
                    cluster_config: Some(common::nodeagent::ClusterConfig {
                        master_endpoint: "localhost:47099".to_string(), // apiserver endpoint
                        heartbeat_interval: 30,
                        settings: std::collections::HashMap::new(),
                    }),
                }))
            }
            Err(e) => {
                println!("Node registration failed: {}", e);
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

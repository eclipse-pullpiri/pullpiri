/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! gRPC Server implementation for API Server clustering service

use crate::cluster::{NodeInfo, NodeRegistry, NodeResources, NodeRole, NodeStatus};
use common::apiserver::{
    api_server_service_server::{ApiServerService, ApiServerServiceServer},
    *,
};
use tokio::sync::OnceCell;
use tonic::{Request, Response, Status};

// Global node registry instance
static NODE_REGISTRY: OnceCell<NodeRegistry> = OnceCell::const_new();

/// Initialize the clustering gRPC service
pub async fn initialize_clustering_service() -> Result<(), Box<dyn std::error::Error>> {
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

/// Get the global node registry
pub fn get_node_registry() -> Option<&'static NodeRegistry> {
    NODE_REGISTRY.get()
}

#[derive(Debug, Default)]
pub struct ApiServerServiceImpl;

#[tonic::async_trait]
impl ApiServerService for ApiServerServiceImpl {
    /// Get all nodes with optional filtering
    async fn get_nodes(
        &self,
        request: Request<GetNodesRequest>,
    ) -> Result<Response<GetNodesResponse>, Status> {
        let registry =
            get_node_registry().ok_or_else(|| Status::internal("Node registry not initialized"))?;

        let req = request.into_inner();
        let nodes = registry
            .get_all_nodes()
            .await
            .map_err(|e| Status::internal(format!("Failed to get nodes: {}", e)))?;

        // Apply filters if provided
        let filtered_nodes: Vec<Node> = nodes
            .into_iter()
            .filter(|node| {
                if let Some(filter) = &req.filter {
                    // Simple filter implementation - can be extended
                    node.node_name.contains(filter) || node.ip_address.contains(filter)
                } else {
                    true
                }
            })
            .map(|node| convert_node_info_to_grpc(node))
            .collect();

        Ok(Response::new(GetNodesResponse {
            nodes: filtered_nodes,
        }))
    }

    /// Get a specific node by ID
    async fn get_node(
        &self,
        request: Request<GetNodeRequest>,
    ) -> Result<Response<GetNodeResponse>, Status> {
        let registry =
            get_node_registry().ok_or_else(|| Status::internal("Node registry not initialized"))?;

        let req = request.into_inner();
        match registry.get_node(&req.node_id).await {
            Ok(node) => {
                let grpc_node = convert_node_info_to_grpc(node);
                Ok(Response::new(GetNodeResponse {
                    node: Some(grpc_node),
                }))
            }
            Err(_) => Err(Status::not_found("Node not found")),
        }
    }

    /// Register a new node
    async fn register_node(
        &self,
        request: Request<NodeRegistrationRequest>,
    ) -> Result<Response<NodeRegistrationResponse>, Status> {
        let registry =
            get_node_registry().ok_or_else(|| Status::internal("Node registry not initialized"))?;

        let req = request.into_inner();

        // Validate required fields
        if req.node_id.is_empty() || req.hostname.is_empty() || req.ip_address.is_empty() {
            return Err(Status::invalid_argument("Missing required fields"));
        }

        // Convert gRPC request to internal format
        let role = match req.role() {
            common::apiserver::NodeRole::Master => NodeRole::Master,
            common::apiserver::NodeRole::Sub => NodeRole::Sub,
            _ => return Err(Status::invalid_argument("Invalid node role")),
        };

        let mut node_info = NodeInfo::new(req.node_id.clone(), req.hostname, req.ip_address);
        node_info.role = role;

        if let Some(resources) = req.resources {
            node_info.resources = NodeResources {
                cpu_cores: resources.cpu_cores,
                memory_mb: resources.memory_mb,
                disk_gb: resources.disk_gb,
                cpu_usage: resources.cpu_usage,
                memory_usage: resources.memory_usage,
            };
        }

        match registry.register_node(node_info).await {
            Ok(cluster_id) => Ok(Response::new(NodeRegistrationResponse {
                success: true,
                message: "Node registered successfully".to_string(),
                cluster_id,
            })),
            Err(e) => Ok(Response::new(NodeRegistrationResponse {
                success: false,
                message: format!("Failed to register node: {}", e),
                cluster_id: String::new(),
            })),
        }
    }

    /// Get cluster topology
    async fn get_topology(
        &self,
        _request: Request<GetTopologyRequest>,
    ) -> Result<Response<GetTopologyResponse>, Status> {
        let registry =
            get_node_registry().ok_or_else(|| Status::internal("Node registry not initialized"))?;

        match registry.get_cluster_topology("default").await {
            Ok(topology) => {
                let grpc_topology = convert_topology_to_grpc(topology);
                Ok(Response::new(GetTopologyResponse {
                    topology: Some(grpc_topology),
                }))
            }
            Err(e) => Err(Status::internal(format!(
                "Failed to get cluster topology: {}",
                e
            ))),
        }
    }

    /// Update cluster topology
    async fn update_topology(
        &self,
        request: Request<UpdateTopologyRequest>,
    ) -> Result<Response<UpdateTopologyResponse>, Status> {
        let _req = request.into_inner();

        // For now, return not implemented
        // This would need to be implemented based on specific requirements
        Err(Status::unimplemented("Topology update not yet implemented"))
    }
}

/// Convert internal NodeInfo to gRPC Node message
fn convert_node_info_to_grpc(node: NodeInfo) -> Node {
    Node {
        node_id: node.node_id,
        hostname: node.node_name,
        ip_address: node.ip_address,
        role: match node.role {
            NodeRole::Master => common::apiserver::NodeRole::Master as i32,
            NodeRole::Sub => common::apiserver::NodeRole::Sub as i32,
        },
        status: match node.status {
            NodeStatus::Online => common::apiserver::NodeStatus::Ready as i32,
            NodeStatus::Offline => common::apiserver::NodeStatus::NotReady as i32,
            NodeStatus::Initializing => common::apiserver::NodeStatus::Initializing as i32,
            NodeStatus::Error => common::apiserver::NodeStatus::NotReady as i32,
            NodeStatus::Maintenance => common::apiserver::NodeStatus::Maintenance as i32,
        },
        resources: Some(ResourceInfo {
            cpu_cores: node.resources.cpu_cores,
            memory_mb: node.resources.memory_mb,
            disk_gb: node.resources.disk_gb,
            cpu_usage: node.resources.cpu_usage,
            memory_usage: node.resources.memory_usage,
        }),
        labels: node.labels,
        created_at: node.created_at,
        last_heartbeat: node.last_heartbeat,
    }
}

/// Convert internal ClusterTopology to gRPC ClusterTopology message
fn convert_topology_to_grpc(topology: crate::cluster::ClusterTopology) -> ClusterTopology {
    ClusterTopology {
        cluster_id: topology.cluster_id,
        cluster_name: topology.cluster_name,
        r#type: match topology.topology_type {
            crate::cluster::TopologyType::Simple => common::apiserver::TopologyType::Simple as i32,
            crate::cluster::TopologyType::Hierarchical => {
                common::apiserver::TopologyType::Hierarchical as i32
            }
            crate::cluster::TopologyType::Mesh => common::apiserver::TopologyType::Mesh as i32,
            crate::cluster::TopologyType::Hybrid => common::apiserver::TopologyType::Hybrid as i32,
        },
        master_nodes: topology
            .master_nodes
            .into_iter()
            .map(convert_node_info_to_grpc)
            .collect(),
        sub_nodes: topology
            .sub_nodes
            .into_iter()
            .map(convert_node_info_to_grpc)
            .collect(),
        config: topology.config,
    }
}

/// Start the gRPC server for clustering
pub async fn start_grpc_server() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the clustering service
    initialize_clustering_service().await?;

    let addr = "[::1]:50051".parse()?;
    let api_service = ApiServerServiceImpl::default();

    println!("Starting gRPC server at {}", addr);

    tonic::transport::Server::builder()
        .add_service(ApiServerServiceServer::new(api_service))
        .serve(addr)
        .await?;

    Ok(())
}

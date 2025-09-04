/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use crate::node::NodeRegistry;
use common::apiserver::{Node, NodeRegistrationRequest, NodeRegistrationResponse, NodeStatus};
use std::sync::Arc;

/// Node manager for cluster operations
#[derive(Clone)]
pub struct NodeManager {
    registry: Arc<NodeRegistry>,
}

impl NodeManager {
    pub fn new(registry: Arc<NodeRegistry>) -> Self {
        Self { registry }
    }

    /// Process node registration request
    pub async fn register_node(
        &self,
        request: NodeRegistrationRequest,
    ) -> Result<NodeRegistrationResponse, String> {
        // Validate request
        if request.node_id.is_empty()
            || request.hostname.is_empty()
            || request.ip_address.is_empty()
        {
            return Err(
                "Invalid registration request: node_id, hostname, and ip_address are required"
                    .to_string(),
            );
        }

        // Authenticate node (simplified for now)
        self.authenticate_node(&request.credentials)?;

        // Create node from request
        let node = Node {
            id: request.node_id.clone(),
            name: request.hostname.clone(),
            ip: request.ip_address.clone(),
            role: request.role,
            status: NodeStatus::Pending as i32,
            resources: request.resources,
            created_at: 0,     // Will be set by registry
            last_heartbeat: 0, // Will be set by registry
        };

        // Register node
        let node_id = self.registry.register_node(node).await?;

        // Get cluster info
        let cluster_info = self.registry.get_cluster_info().await;

        Ok(NodeRegistrationResponse {
            node_id,
            cluster_info: Some(cluster_info),
            status: NodeStatus::Initializing as i32,
            success: true,
            error_message: None,
        })
    }

    /// Validate node credentials
    fn authenticate_node(
        &self,
        credentials: &Option<common::apiserver::Credentials>,
    ) -> Result<(), String> {
        match credentials {
            Some(creds) => {
                if creds.token.is_empty() {
                    return Err("Authentication token is required".to_string());
                }

                // Simplified authentication - in production, verify token properly
                if creds.token != "piccolo-cluster-token" {
                    return Err("Invalid authentication token".to_string());
                }

                Ok(())
            }
            None => Err("Credentials are required for node registration".to_string()),
        }
    }

    /// Generate node ID if not provided
    pub fn generate_node_id(&self, hostname: &str, ip: &str) -> String {
        format!("node-{}-{}", hostname, ip.replace('.', "-"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::apiserver::{Credentials, NodeRole, ResourceInfo};

    #[tokio::test]
    async fn test_node_registration() {
        let registry = Arc::new(NodeRegistry::new());
        let manager = NodeManager::new(registry);

        let request = NodeRegistrationRequest {
            node_id: "test-node-001".to_string(),
            hostname: "test-host".to_string(),
            ip_address: "192.168.1.100".to_string(),
            role: NodeRole::Sub as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 4,
                memory_mb: 8192,
                disk_gb: 256,
            }),
            credentials: Some(Credentials {
                token: "piccolo-cluster-token".to_string(),
                certificate: "".to_string(),
            }),
        };

        let result = manager.register_node(request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.success);
        assert_eq!(response.node_id, "test-node-001");
        assert!(response.cluster_info.is_some());
    }

    #[tokio::test]
    async fn test_invalid_credentials() {
        let registry = Arc::new(NodeRegistry::new());
        let manager = NodeManager::new(registry);

        let request = NodeRegistrationRequest {
            node_id: "test-node-002".to_string(),
            hostname: "test-host-2".to_string(),
            ip_address: "192.168.1.101".to_string(),
            role: NodeRole::Sub as i32,
            resources: Some(ResourceInfo {
                cpu_cores: 2,
                memory_mb: 4096,
                disk_gb: 128,
            }),
            credentials: Some(Credentials {
                token: "invalid-token".to_string(),
                certificate: "".to_string(),
            }),
        };

        let result = manager.register_node(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid authentication token"));
    }
}

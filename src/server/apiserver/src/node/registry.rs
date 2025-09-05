/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! Node registry for cluster membership management

use common::apiserver::{ClusterTopology, TopologyType};
use common::etcd;
use prost::Message;

/// Node registry for managing cluster topology
#[derive(Clone)]
pub struct NodeRegistry;

impl NodeRegistry {
    /// Get the current cluster topology
    pub async fn get_topology(&self) -> Result<ClusterTopology, Box<dyn std::error::Error + Send + Sync>> {
        let topology_key = "cluster/topology";
        
        match etcd::get(topology_key).await {
            Ok(encoded) => {
                let buf = base64::decode(&encoded)?;
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
    pub async fn update_topology(&self, topology: ClusterTopology) -> Result<ClusterTopology, Box<dyn std::error::Error + Send + Sync>> {
        let topology_key = "cluster/topology";
        
        let mut buf = Vec::new();
        prost::Message::encode(&topology, &mut buf)?;
        let encoded = base64::encode(&buf);
        
        etcd::put(topology_key, &encoded).await?;
        
        println!("Updated cluster topology: {}", topology.cluster_name);
        Ok(topology)
    }

    /// Initialize default cluster topology
    pub async fn initialize_default_topology(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    #[tokio::test]
    async fn test_node_registry_operations() {
        let registry = NodeRegistry;

        // Test getting topology (will use default if etcd not available)
        match registry.get_topology().await {
            Ok(topology) => {
                assert_eq!(topology.cluster_id, "default-cluster");
            }
            Err(_) => {
                // Expected if etcd is not available during testing
            }
        }
    }
}
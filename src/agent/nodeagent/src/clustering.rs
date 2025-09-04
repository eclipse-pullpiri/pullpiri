/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! NodeAgent clustering functionality for node registration and heartbeat

use common::{
    nodeagent::{
        node_agent_connection_client::NodeAgentConnectionClient, HeartbeatRequest,
        NodeRegistrationRequest, NodeResources, NodeRole, NodeStatus,
    },
    setting, Result,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use tokio::time::Duration;
use tonic::{transport::Channel, Request};
use uuid::Uuid;

/// Global connection state
static CONNECTED: AtomicBool = AtomicBool::new(false);

/// Node configuration for clustering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,
    pub node_name: String,
    pub role: String, // "master" or "sub"
    pub master_ip: String,
    pub api_port: u16,
    pub labels: HashMap<String, String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        let hostname = std::process::Command::new("hostname")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            node_id: Uuid::new_v4().to_string(),
            node_name: hostname,
            role: "sub".to_string(), // Default to sub node
            master_ip: "127.0.0.1".to_string(),
            api_port: 47007,
            labels: HashMap::new(),
        }
    }
}

/// Cluster client for managing node operations
pub struct ClusterClient {
    config: NodeConfig,
    client: Option<NodeAgentConnectionClient<Channel>>,
}

impl ClusterClient {
    /// Create a new cluster client
    pub fn new(config: NodeConfig) -> Self {
        Self {
            config,
            client: None,
        }
    }

    /// Initialize cluster operations (registration and heartbeat)
    pub async fn initialize(&mut self) -> Result<()> {
        println!(
            "Initializing cluster client for node: {}",
            self.config.node_name
        );

        // Try to register with master
        if let Err(e) = self.register_node().await {
            eprintln!(
                "Failed to register node: {}. Will retry during heartbeat loop.",
                e
            );
        }

        // Start heartbeat background task
        let config = self.config.clone();
        tokio::spawn(async move {
            heartbeat_loop(config).await;
        });

        Ok(())
    }

    /// Register this node with the master
    pub async fn register_node(&mut self) -> Result<()> {
        let endpoint = format!("http://{}:{}", self.config.master_ip, self.config.api_port);

        match NodeAgentConnectionClient::connect(endpoint.clone()).await {
            Ok(client) => {
                self.client = Some(client);

                // Collect system information
                let mut sys = System::new_all();
                sys.refresh_all();

                let resources = NodeResources {
                    cpu_cores: sys.cpus().len() as u32,
                    memory_mb: sys.total_memory() / 1024 / 1024,
                    disk_gb: 10, // Default value - could be enhanced to detect actual disk space
                    cpu_usage: 0.0,
                    memory_usage: 0.0,
                };

                let role = match self.config.role.to_lowercase().as_str() {
                    "master" => NodeRole::Master as i32,
                    _ => NodeRole::Sub as i32,
                };

                let request = NodeRegistrationRequest {
                    node_id: self.config.node_id.clone(),
                    node_name: self.config.node_name.clone(),
                    ip_address: get_local_ip(),
                    role,
                    resources: Some(resources),
                    labels: self.config.labels.clone(),
                };

                if let Some(ref mut client) = self.client {
                    match client.register_node(Request::new(request)).await {
                        Ok(response) => {
                            let resp = response.into_inner();
                            if resp.success {
                                println!("Node registered successfully: {}", resp.message);
                                CONNECTED.store(true, Ordering::SeqCst);
                                return Ok(());
                            } else {
                                return Err(format!("Registration failed: {}", resp.message).into());
                            }
                        }
                        Err(e) => {
                            return Err(
                                format!("Failed to send registration request: {}", e).into()
                            );
                        }
                    }
                }

                Err("No client available".into())
            }
            Err(e) => Err(format!("Failed to connect to master at {}: {}", endpoint, e).into()),
        }
    }

    /// Send heartbeat to master
    pub async fn send_heartbeat(&mut self) -> Result<()> {
        if self.client.is_none() {
            return Err("Not connected to master".into());
        }

        // Collect current system metrics
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut metrics = HashMap::new();

        // Calculate CPU usage (simplified - average across all CPUs)
        let cpu_usage: f32 = if !sys.cpus().is_empty() {
            sys.cpus().iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / sys.cpus().len() as f32
        } else {
            0.0
        };
        metrics.insert("cpu_usage".to_string(), cpu_usage.to_string());

        // Calculate memory usage percentage
        let memory_usage = if sys.total_memory() > 0 {
            (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0
        } else {
            0.0
        };
        metrics.insert("memory_usage".to_string(), memory_usage.to_string());

        let request = HeartbeatRequest {
            node_id: self.config.node_id.clone(),
            status: NodeStatus::Online as i32,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            metrics,
        };

        if let Some(ref mut client) = self.client {
            match client.send_heartbeat(Request::new(request)).await {
                Ok(response) => {
                    let resp = response.into_inner();
                    if resp.acknowledged {
                        println!("Heartbeat acknowledged: {}", resp.message);
                        return Ok(());
                    } else {
                        return Err(format!("Heartbeat not acknowledged: {}", resp.message).into());
                    }
                }
                Err(e) => {
                    return Err(format!("Failed to send heartbeat: {}", e).into());
                }
            }
        }

        Err("No client available".into())
    }
}

/// Background heartbeat loop
async fn heartbeat_loop(config: NodeConfig) {
    let mut interval = tokio::time::interval(Duration::from_secs(30)); // 30-second intervals
    let mut cluster_client = ClusterClient::new(config);

    loop {
        interval.tick().await;

        // Check if we're connected to master
        if !CONNECTED.load(Ordering::SeqCst) {
            // Try to reconnect and register
            if let Err(e) = cluster_client.register_node().await {
                eprintln!("Failed to register node: {}", e);
                continue;
            }
        }

        // Send heartbeat
        match cluster_client.send_heartbeat().await {
            Ok(_) => {
                println!("Heartbeat sent successfully");
            }
            Err(e) => {
                eprintln!("Heartbeat failed: {}", e);
                CONNECTED.store(false, Ordering::SeqCst);
                // Will try to reconnect on next iteration
            }
        }
    }
}

/// Get local IP address (simplified version)
fn get_local_ip() -> String {
    // Try to get from configuration first
    let config = setting::get_config();
    if !config.host.ip.is_empty() && config.host.ip != "0.0.0.0" {
        return config.host.ip.clone();
    }

    // Fallback to detecting actual IP (simplified - would need more robust implementation)
    "127.0.0.1".to_string()
}

/// Load node configuration from file or environment
pub fn load_node_config() -> NodeConfig {
    // This would ideally load from a configuration file
    // For now, create a default configuration
    let mut config = NodeConfig::default();

    // Override with environment variables if present
    if let Ok(master_ip) = std::env::var("PICCOLO_MASTER_IP") {
        config.master_ip = master_ip;
    }

    if let Ok(node_role) = std::env::var("PICCOLO_NODE_ROLE") {
        config.role = node_role;
    }

    if let Ok(node_name) = std::env::var("PICCOLO_NODE_NAME") {
        config.node_name = node_name;
    }

    config
}

/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use common::apiserver::{
    api_server_service_client::ApiServerServiceClient, Credentials, NodeRegistrationRequest,
    NodeRegistrationResponse, NodeRole, ResourceInfo,
};
use std::collections::HashMap;
use sysinfo::System;
use tonic::{transport::Channel, Request};

/// Cluster client for NodeAgent to communicate with API Server
#[derive(Clone)]
pub struct ClusterClient {
    client: Option<ApiServerServiceClient<Channel>>,
    master_endpoint: String,
    node_id: String,
    hostname: String,
    ip_address: String,
}

impl ClusterClient {
    pub fn new(master_endpoint: String, hostname: String, ip_address: String) -> Self {
        let node_id = Self::generate_node_id(&hostname, &ip_address);

        Self {
            client: None,
            master_endpoint,
            node_id,
            hostname,
            ip_address,
        }
    }

    /// Connect to the master API server
    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = format!("http://{}", self.master_endpoint);
        println!("Connecting to master API server at: {}", endpoint);

        let client = ApiServerServiceClient::connect(endpoint).await?;
        self.client = Some(client);

        Ok(())
    }

    /// Register this node with the cluster
    pub async fn register_node(
        &mut self,
    ) -> Result<NodeRegistrationResponse, Box<dyn std::error::Error>> {
        if self.client.is_none() {
            self.connect().await?;
        }

        // Collect system information first
        let resources = self.collect_system_resources().await;

        let client = self.client.as_mut().unwrap();

        let request = NodeRegistrationRequest {
            node_id: self.node_id.clone(),
            hostname: self.hostname.clone(),
            ip_address: self.ip_address.clone(),
            role: NodeRole::Sub as i32,
            resources: Some(resources),
            credentials: Some(Credentials {
                token: "piccolo-cluster-token".to_string(), // In production, use secure token
                certificate: "".to_string(),
            }),
        };

        println!("Registering node {} with cluster", self.node_id);
        let response = client.register_node(Request::new(request)).await?;

        let registration_response = response.into_inner();
        if registration_response.success {
            println!("Node {} successfully registered with cluster", self.node_id);
        } else {
            eprintln!(
                "Node registration failed: {:?}",
                registration_response.error_message
            );
        }

        Ok(registration_response)
    }

    /// Send heartbeat to master
    pub async fn send_heartbeat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // This would use NodeAgent gRPC to send heartbeat
        // For now, we'll implement this as a placeholder
        println!("Sending heartbeat for node {}", self.node_id);
        Ok(())
    }

    /// Send status report to master
    pub async fn send_status_report(
        &mut self,
        metrics: HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // This would use NodeAgent gRPC to send status report
        // For now, we'll implement this as a placeholder
        println!(
            "Sending status report for node {}: {:?}",
            self.node_id, metrics
        );
        Ok(())
    }

    /// Collect system resource information
    async fn collect_system_resources(&self) -> ResourceInfo {
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_cores = sys.cpus().len() as u32;
        let total_memory = sys.total_memory() / 1024 / 1024; // Convert to MB

        // Estimate disk space (simplified)
        let disk_gb = 100; // Default assumption, could be enhanced with actual disk detection

        ResourceInfo {
            cpu_cores,
            memory_mb: total_memory,
            disk_gb,
        }
    }

    /// Generate node ID from hostname and IP
    fn generate_node_id(hostname: &str, ip: &str) -> String {
        format!("node-{}-{}", hostname, ip.replace('.', "-"))
    }

    /// Get node ID
    pub fn get_node_id(&self) -> &str {
        &self.node_id
    }

    /// Get hostname
    pub fn get_hostname(&self) -> &str {
        &self.hostname
    }

    /// Get IP address
    pub fn get_ip_address(&self) -> &str {
        &self.ip_address
    }

    /// Start background tasks for periodic communication with master
    pub async fn start_background_tasks(&mut self) {
        let mut heartbeat_interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        let mut status_interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        // Clone for background task
        let mut client_clone = self.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = heartbeat_interval.tick() => {
                        if let Err(e) = client_clone.send_heartbeat().await {
                            eprintln!("Failed to send heartbeat: {}", e);
                        }
                    }
                    _ = status_interval.tick() => {
                        let metrics = client_clone.collect_metrics().await;
                        if let Err(e) = client_clone.send_status_report(metrics).await {
                            eprintln!("Failed to send status report: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Collect current metrics
    async fn collect_metrics(&self) -> HashMap<String, String> {
        let mut sys = System::new_all();
        sys.refresh_all();

        let mut metrics = HashMap::new();

        // CPU usage (simplified - just use first CPU)
        if let Some(cpu) = sys.cpus().first() {
            metrics.insert("cpu_usage".to_string(), format!("{:.2}", cpu.cpu_usage()));
        }

        // Memory usage
        let memory_usage = (sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0;
        metrics.insert("memory_usage".to_string(), format!("{:.2}", memory_usage));

        // Total memory
        metrics.insert(
            "total_memory".to_string(),
            (sys.total_memory() / 1024 / 1024).to_string(),
        );
        metrics.insert(
            "used_memory".to_string(),
            (sys.used_memory() / 1024 / 1024).to_string(),
        );

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_node_id() {
        let node_id = ClusterClient::generate_node_id("test-host", "192.168.1.100");
        assert_eq!(node_id, "node-test-host-192-168-1-100");
    }

    #[tokio::test]
    async fn test_collect_system_resources() {
        let client = ClusterClient::new(
            "localhost:47098".to_string(),
            "test-host".to_string(),
            "127.0.0.1".to_string(),
        );

        let resources = client.collect_system_resources().await;
        assert!(resources.cpu_cores > 0);
        assert!(resources.memory_mb > 0);
        assert_eq!(resources.disk_gb, 100); // Default value
    }

    #[tokio::test]
    async fn test_collect_metrics() {
        let client = ClusterClient::new(
            "localhost:47098".to_string(),
            "test-host".to_string(),
            "127.0.0.1".to_string(),
        );

        let metrics = client.collect_metrics().await;
        assert!(metrics.contains_key("cpu_usage"));
        assert!(metrics.contains_key("memory_usage"));
        assert!(metrics.contains_key("total_memory"));
        assert!(metrics.contains_key("used_memory"));
    }
}

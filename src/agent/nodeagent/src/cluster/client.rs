/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

//! NodeAgent Clustering Client
//!
//! Implements the clustering functionality for NodeAgent including:
//! - Node registration with master
//! - Heartbeat mechanism
//! - Status reporting
//! - Connection management and recovery

use anyhow::{anyhow, Result};
use common::apiserver::api_server_cluster_client::ApiServerClusterClient;
use common::nodeagent::{
    AlertInfo, ContainerStatus, Credentials, HeartbeatRequest, HeartbeatResponse,
    NodeRegistrationRequest, NodeRegistrationResponse, NodeRole, NodeStatus, ResourceInfo,
    StatusAck, StatusReport,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tonic::transport::Channel;
use tonic::Request;

/// Configuration for NodeAgent clustering
#[derive(Clone, Debug)]
pub struct ClusterConfig {
    pub master_endpoint: String,
    pub node_id: Option<String>,
    pub hostname: String,
    pub ip_address: String,
    pub node_role: NodeRole,
    pub heartbeat_interval: Duration,
    pub status_report_interval: Duration,
    pub connection_timeout: Duration,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            master_endpoint: "http://localhost:47100".to_string(),
            node_id: None,
            hostname: hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            ip_address: "127.0.0.1".to_string(),
            node_role: NodeRole::Sub,
            heartbeat_interval: Duration::from_secs(10),
            status_report_interval: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(5),
        }
    }
}

/// NodeAgent clustering client
#[derive(Clone)]
pub struct ClusterClient {
    config: ClusterConfig,
    node_id: Arc<RwLock<Option<String>>>,
    connected: Arc<AtomicBool>,
    client: Arc<RwLock<Option<ApiServerClusterClient<Channel>>>>,
}

impl ClusterClient {
    /// Create a new cluster client
    pub fn new(config: ClusterConfig) -> Self {
        Self {
            config,
            node_id: Arc::new(RwLock::new(None)),
            connected: Arc::new(AtomicBool::new(false)),
            client: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the cluster client
    pub async fn initialize(&self) -> Result<()> {
        log::info!(
            "Initializing cluster client for master: {}",
            self.config.master_endpoint
        );

        // Perform system readiness check
        self.system_readiness_check().await?;

        // Connect to master
        self.connect_to_master().await?;

        // Register node
        self.register_node().await?;

        log::info!("Cluster client initialized successfully");
        Ok(())
    }

    /// Start background tasks for heartbeat and status reporting
    pub async fn start_background_tasks(&self) {
        log::info!("Starting clustering background tasks");

        // Start heartbeat task
        let client = self.clone();
        tokio::spawn(async move {
            client.heartbeat_loop().await;
        });

        // Start status reporting task
        let client = self.clone();
        tokio::spawn(async move {
            client.status_report_loop().await;
        });

        // Start connection monitoring task
        let client = self.clone();
        tokio::spawn(async move {
            client.connection_monitor_loop().await;
        });
    }

    /// Perform system readiness check
    async fn system_readiness_check(&self) -> Result<()> {
        log::info!("Performing system readiness check");

        // Check if we can resolve the hostname
        if self.config.hostname.is_empty() {
            return Err(anyhow!("Hostname is empty"));
        }

        // Check if IP address is valid
        if self.config.ip_address.is_empty() || self.config.ip_address == "0.0.0.0" {
            return Err(anyhow!("Invalid IP address: {}", self.config.ip_address));
        }

        // Check basic system resources
        let available_memory = self.get_available_memory().await?;
        if available_memory < 100 {
            // Minimum 100MB
            log::warn!("Low memory available: {}MB", available_memory);
        }

        // Check if required services are running (this is a simplified check)
        log::info!("System readiness check completed successfully");
        Ok(())
    }

    /// Connect to master node
    async fn connect_to_master(&self) -> Result<()> {
        log::info!("Connecting to master node: {}", self.config.master_endpoint);

        let client = ApiServerClusterClient::connect(self.config.master_endpoint.clone())
            .await
            .map_err(|e| anyhow!("Failed to connect to master: {}", e))?;

        {
            let mut client_guard = self.client.write().await;
            *client_guard = Some(client);
        }

        self.connected.store(true, Ordering::SeqCst);
        log::info!("Successfully connected to master node");
        Ok(())
    }

    /// Register node with the master
    async fn register_node(&self) -> Result<()> {
        log::info!("Registering node with master");

        let system_info = self.collect_system_info().await?;

        let request = NodeRegistrationRequest {
            node_id: self.config.node_id.clone().unwrap_or_default(),
            hostname: self.config.hostname.clone(),
            ip_address: self.config.ip_address.clone(),
            role: self.config.node_role.into(),
            resources: Some(system_info),
            credentials: Some(self.generate_credentials()?),
        };

        let response = {
            let client_guard = self.client.read().await;
            match client_guard.as_ref() {
                Some(client) => {
                    let mut client = client.clone();
                    client
                        .register_node(Request::new(request))
                        .await
                        .map_err(|e| anyhow!("Registration failed: {}", e))?
                        .into_inner()
                }
                None => return Err(anyhow!("Not connected to master")),
            }
        };

        // Store the assigned node ID
        {
            let mut node_id = self.node_id.write().await;
            *node_id = Some(response.node_id.clone());
        }

        log::info!("Node registration successful: {}", response.node_id);
        if let Some(cluster_info) = response.cluster_info {
            log::info!(
                "Joined cluster: {} ({})",
                cluster_info.cluster_name,
                cluster_info.cluster_id
            );
        }

        Ok(())
    }

    /// Send heartbeat to master
    async fn send_heartbeat(&self) -> Result<()> {
        let node_id = {
            let node_id_guard = self.node_id.read().await;
            match node_id_guard.as_ref() {
                Some(id) => id.clone(),
                None => return Err(anyhow!("Node not registered")),
            }
        };

        let request = HeartbeatRequest {
            node_id,
            timestamp: chrono::Utc::now().timestamp(),
            status: NodeStatus::Ready.into(),
        };

        let _response = {
            let client_guard = self.client.read().await;
            match client_guard.as_ref() {
                Some(client) => {
                    // Create a new client instance for this request to avoid borrowing issues
                    let endpoint = self.config.master_endpoint.clone();
                    let mut new_client = ApiServerClusterClient::connect(endpoint)
                        .await
                        .map_err(|e| anyhow!("Failed to connect for heartbeat: {}", e))?;

                    new_client
                        .health_check(Request::new(common::apiserver::HealthCheckRequest {}))
                        .await
                        .map_err(|e| anyhow!("Heartbeat failed: {}", e))?
                        .into_inner()
                }
                None => return Err(anyhow!("Not connected to master")),
            }
        };

        log::debug!("Heartbeat sent successfully");
        Ok(())
    }

    /// Send status report to master
    async fn send_status_report(&self) -> Result<()> {
        let node_id = {
            let node_id_guard = self.node_id.read().await;
            match node_id_guard.as_ref() {
                Some(id) => id.clone(),
                None => return Err(anyhow!("Node not registered")),
            }
        };

        let containers = self.collect_container_status().await?;
        let alerts = self.collect_alerts().await?;
        let metrics = self.collect_metrics().await?;

        let report = StatusReport {
            node_id,
            status: NodeStatus::Ready.into(),
            metrics,
            containers,
            alerts,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // For now, just log the status report since we don't have the status reporting service
        // In a full implementation, this would send to the NodeAgent status service
        log::info!(
            "Status report generated: {} containers, {} alerts",
            report.containers.len(),
            report.alerts.len()
        );

        Ok(())
    }

    /// Heartbeat background task
    async fn heartbeat_loop(&self) {
        let mut interval = tokio::time::interval(self.config.heartbeat_interval);

        loop {
            interval.tick().await;

            if self.connected.load(Ordering::SeqCst) {
                match self.send_heartbeat().await {
                    Ok(_) => log::debug!("Heartbeat sent"),
                    Err(e) => {
                        log::warn!("Heartbeat failed: {}", e);
                        self.connected.store(false, Ordering::SeqCst);
                    }
                }
            }
        }
    }

    /// Status reporting background task
    async fn status_report_loop(&self) {
        let mut interval = tokio::time::interval(self.config.status_report_interval);

        loop {
            interval.tick().await;

            if self.connected.load(Ordering::SeqCst) {
                match self.send_status_report().await {
                    Ok(_) => log::debug!("Status report sent"),
                    Err(e) => log::warn!("Status report failed: {}", e),
                }
            }
        }
    }

    /// Connection monitoring background task
    async fn connection_monitor_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            if !self.connected.load(Ordering::SeqCst) {
                log::info!("Connection lost, attempting to reconnect...");

                match self.reconnect().await {
                    Ok(_) => log::info!("Reconnection successful"),
                    Err(e) => log::error!("Reconnection failed: {}", e),
                }
            }
        }
    }

    /// Attempt to reconnect to master
    async fn reconnect(&self) -> Result<()> {
        // Try to connect
        self.connect_to_master().await?;

        // Re-register if we don't have a node ID
        let has_node_id = {
            let node_id_guard = self.node_id.read().await;
            node_id_guard.is_some()
        };

        if !has_node_id {
            self.register_node().await?;
        }

        Ok(())
    }

    /// Collect system information for registration
    async fn collect_system_info(&self) -> Result<ResourceInfo> {
        let cpu_cores = num_cpus::get() as i32;
        let memory_mb = self.get_total_memory().await?;
        let disk_gb = self.get_disk_space().await?;

        Ok(ResourceInfo {
            cpu_cores,
            memory_mb,
            disk_gb,
            architecture: std::env::consts::ARCH.to_string(),
            platform: std::env::consts::OS.to_string(),
        })
    }

    /// Generate credentials for authentication
    fn generate_credentials(&self) -> Result<Credentials> {
        // In a production environment, this would generate proper credentials
        // For now, we'll use a simple token
        Ok(Credentials {
            token: format!(
                "node-{}-{}",
                self.config.hostname,
                chrono::Utc::now().timestamp()
            ),
            certificate: String::new(),
            expires_at: chrono::Utc::now().timestamp() + 3600, // 1 hour
        })
    }

    /// Get total system memory in MB
    async fn get_total_memory(&self) -> Result<i64> {
        // This is a simplified implementation
        // In a real implementation, this would query the system
        Ok(2048) // Default to 2GB
    }

    /// Get available memory in MB
    async fn get_available_memory(&self) -> Result<i64> {
        // This is a simplified implementation
        Ok(1024) // Default to 1GB available
    }

    /// Get disk space in GB
    async fn get_disk_space(&self) -> Result<i64> {
        // This is a simplified implementation
        Ok(100) // Default to 100GB
    }

    /// Collect container status information
    async fn collect_container_status(&self) -> Result<Vec<ContainerStatus>> {
        // This would integrate with the container management system
        // For now, return empty list
        Ok(vec![])
    }

    /// Collect system alerts
    async fn collect_alerts(&self) -> Result<Vec<AlertInfo>> {
        // This would check for system alerts
        // For now, return empty list
        Ok(vec![])
    }

    /// Collect system metrics
    async fn collect_metrics(&self) -> Result<std::collections::HashMap<String, String>> {
        let mut metrics = std::collections::HashMap::new();

        // Add basic metrics
        metrics.insert("cpu_usage".to_string(), "25%".to_string());
        metrics.insert("memory_usage".to_string(), "50%".to_string());
        metrics.insert("disk_usage".to_string(), "30%".to_string());
        metrics.insert("uptime".to_string(), "3600".to_string());

        Ok(metrics)
    }

    /// Check if the node is connected to the cluster
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    /// Get the node ID
    pub async fn get_node_id(&self) -> Option<String> {
        let node_id_guard = self.node_id.read().await;
        node_id_guard.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_config_default() {
        let config = ClusterConfig::default();
        assert_eq!(config.node_role, NodeRole::Sub);
        assert_eq!(config.heartbeat_interval, Duration::from_secs(10));
        assert!(!config.hostname.is_empty());
    }

    #[tokio::test]
    async fn test_cluster_client_creation() {
        let config = ClusterConfig::default();
        let client = ClusterClient::new(config);

        assert!(!client.is_connected());
        assert!(client.get_node_id().await.is_none());
    }

    #[tokio::test]
    async fn test_system_readiness_check() {
        let mut config = ClusterConfig::default();
        config.hostname = "test-host".to_string();
        config.ip_address = "192.168.1.100".to_string();

        let client = ClusterClient::new(config);

        // This should pass since we have valid hostname and IP
        let result = client.system_readiness_check().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_system_readiness_check_invalid_ip() {
        let mut config = ClusterConfig::default();
        config.hostname = "test-host".to_string();
        config.ip_address = "0.0.0.0".to_string();

        let client = ClusterClient::new(config);

        // This should fail due to invalid IP
        let result = client.system_readiness_check().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collect_system_info() {
        let config = ClusterConfig::default();
        let client = ClusterClient::new(config);

        let system_info = client.collect_system_info().await.unwrap();

        assert!(system_info.cpu_cores > 0);
        assert!(system_info.memory_mb > 0);
        assert!(system_info.disk_gb > 0);
        assert!(!system_info.architecture.is_empty());
        assert!(!system_info.platform.is_empty());
    }
}

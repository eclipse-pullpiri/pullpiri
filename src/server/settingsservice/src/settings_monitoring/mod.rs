// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Monitoring and metrics management module
use crate::monitoring_types::{BoardInfo, NodeInfo, SocInfo};
use crate::settings_storage::filter_key;
use crate::settings_storage::Storage;
use crate::settings_utils::error::SettingsError;
use chrono::{DateTime, Utc};
use common::monitoringserver::ContainerInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Metrics filter for data selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsFilter {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub components: Option<Vec<String>>,
    pub metric_types: Option<Vec<String>>,
    pub label_selectors: Option<HashMap<String, String>>,
    pub time_range: Option<TimeRange>,
    pub refresh_interval: Option<u64>, // seconds
    pub max_items: Option<usize>,
    #[serde(default = "default_version")]
    pub version: u64,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub modified_at: DateTime<Utc>,
}

fn default_version() -> u64 {
    1
}

#[derive(Debug, Serialize)]
pub struct NodeListResponse {
    pub nodes: Vec<NodeInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SocListResponse {
    pub socs: Vec<SocInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct BoardListResponse {
    pub boards: Vec<BoardInfo>,
    pub total: usize,
}

/// Time range for metrics filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
}

/// Metric value types - Updated to support resource objects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MetricValue {
    // Traditional metric types
    Counter { value: u64 },
    Gauge { value: f64 },
    Histogram { buckets: Vec<HistogramBucket> },
    Summary { quantiles: Vec<SummaryQuantile> },

    // Resource-based metric types (matching etcd data)
    NodeInfo { value: NodeInfo },
    ContainerInfo { value: ContainerInfo },
    SocInfo { value: SocInfo },
    BoardInfo { value: BoardInfo },
}

/// Enhanced Metric structure to better match usage patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub id: String,
    pub component: String,   // "node", "container", "soc", "board"
    pub metric_type: String, // "NodeInfo", "ContainerInfo", "SocInfo", "BoardInfo"
    pub labels: HashMap<String, String>,
    pub value: MetricValue,
    pub timestamp: DateTime<Utc>,
}

/// Histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

/// Summary quantile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryQuantile {
    pub quantile: f64,
    pub value: f64,
}

/// Filter summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSummary {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub component_count: usize,
    pub metric_type_count: usize,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

/// Cache entry with expiration
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub data: T,
    pub expiry: Instant,
}

/// Monitoring manager for metrics filtering and caching - RESTRUCTURED
pub struct MonitoringManager {
    storage: Box<dyn Storage>, // Only used for filters, not metrics
    cache: RwLock<HashMap<String, CacheEntry<Vec<Metric>>>>,
    cache_ttl: Duration,
}

impl MonitoringManager {
    pub fn new(storage: Box<dyn Storage>, cache_ttl_seconds: u64) -> Self {
        Self {
            storage,
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(cache_ttl_seconds),
        }
    }

    /// Get metrics with optional filtering
    pub async fn get_metrics(
        &mut self,
        filter: Option<&MetricsFilter>,
    ) -> Result<Vec<Metric>, SettingsError> {
        debug!("Getting metrics with filter: {:?}", filter.map(|f| &f.name));

        // Generate cache key
        let cache_key = if let Some(f) = filter {
            format!("filter:{}", f.id)
        } else {
            "all".to_string()
        };

        // Check cache first
        if let Some(cached) = self.get_cached(&cache_key) {
            debug!("Returning cached metrics for key: {}", cache_key);
            return Ok(cached);
        }

        // Fetch from monitoring ETCD directly
        let metrics = self.fetch_metrics_from_monitoring_etcd(filter).await?;

        // Cache the results
        self.set_cached(&cache_key, metrics.clone());

        Ok(metrics)
    }

    /// Fetch metrics directly from monitoring ETCD
    async fn fetch_metrics_from_monitoring_etcd(
        &mut self,
        filter: Option<&MetricsFilter>,
    ) -> Result<Vec<Metric>, SettingsError> {
        let mut metrics = Vec::new();

        // Get nodes and convert to Metric format - using monitoring_etcd
        match crate::monitoring_etcd::get_all_nodes().await {
            Ok(nodes) => {
                for node_info in nodes {
                    let metric = Metric {
                        id: node_info.node_name.clone(),
                        component: "node".to_string(),
                        metric_type: "NodeInfo".to_string(),
                        labels: {
                            let mut labels = HashMap::new();
                            labels.insert("node_name".to_string(), node_info.node_name.clone());
                            labels.insert("ip".to_string(), node_info.ip.clone());
                            labels.insert("os".to_string(), node_info.os.clone());
                            labels.insert("arch".to_string(), node_info.arch.clone());
                            labels
                        },
                        value: MetricValue::NodeInfo { value: node_info },
                        timestamp: Utc::now(),
                    };

                    if self.metric_matches_filter(&metric, filter) {
                        metrics.push(metric);
                    }
                }
            }
            Err(e) => {
                debug!("No node metrics available: {}", e);
            }
        }

        // Get containers and convert to Metric format - using monitoring_etcd
        match crate::monitoring_etcd::get_all_containers().await {
            Ok(containers) => {
                for container_info in containers {
                    let metric = Metric {
                        id: container_info.id.clone(),
                        component: "container".to_string(),
                        metric_type: "ContainerInfo".to_string(),
                        labels: {
                            let mut labels = HashMap::new();
                            labels.insert("container_id".to_string(), container_info.id.clone());
                            labels.insert("image".to_string(), container_info.image.clone());
                            if let Some(name) = container_info.names.first() {
                                labels.insert("container_name".to_string(), name.clone());
                            }
                            if let Some(status) = container_info.state.get("Status") {
                                labels.insert("status".to_string(), status.clone());
                            }
                            labels
                        },
                        value: MetricValue::ContainerInfo {
                            value: container_info,
                        },
                        timestamp: Utc::now(),
                    };

                    if self.metric_matches_filter(&metric, filter) {
                        metrics.push(metric);
                    }
                }
            }
            Err(e) => {
                debug!("No container metrics available: {}", e);
            }
        }

        // Get SoCs and convert to Metric format - using monitoring_etcd
        match crate::monitoring_etcd::get_all_socs().await {
            Ok(socs) => {
                for soc_info in socs {
                    let metric = Metric {
                        id: soc_info.soc_id.clone(),
                        component: "soc".to_string(),
                        metric_type: "SocInfo".to_string(),
                        labels: {
                            let mut labels = HashMap::new();
                            labels.insert("soc_id".to_string(), soc_info.soc_id.clone());
                            labels
                                .insert("node_count".to_string(), soc_info.nodes.len().to_string());
                            labels
                        },
                        value: MetricValue::SocInfo { value: soc_info },
                        timestamp: Utc::now(),
                    };

                    if self.metric_matches_filter(&metric, filter) {
                        metrics.push(metric);
                    }
                }
            }
            Err(e) => {
                debug!("No SoC metrics available: {}", e);
            }
        }

        // Get boards and convert to Metric format - using monitoring_etcd
        match crate::monitoring_etcd::get_all_boards().await {
            Ok(boards) => {
                for board_info in boards {
                    let metric = Metric {
                        id: board_info.board_id.clone(),
                        component: "board".to_string(),
                        metric_type: "BoardInfo".to_string(),
                        labels: {
                            let mut labels = HashMap::new();
                            labels.insert("board_id".to_string(), board_info.board_id.clone());
                            labels.insert(
                                "node_count".to_string(),
                                board_info.nodes.len().to_string(),
                            );
                            labels
                                .insert("soc_count".to_string(), board_info.socs.len().to_string());
                            labels
                        },
                        value: MetricValue::BoardInfo { value: board_info },
                        timestamp: Utc::now(),
                    };

                    if self.metric_matches_filter(&metric, filter) {
                        metrics.push(metric);
                    }
                }
            }
            Err(e) => {
                debug!("No board metrics available: {}", e);
            }
        }

        // Apply limits and sorting
        if let Some(filter) = filter {
            if let Some(max_items) = filter.max_items {
                metrics.truncate(max_items);
            }
        }

        // Sort by timestamp (newest first)
        metrics.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        info!("Fetched {} metrics from monitoring ETCD", metrics.len());
        Ok(metrics)
    }

    /// Get all node metrics - SIMPLIFIED to use monitoring_etcd directly
    pub async fn get_node_metrics(&mut self) -> Result<Vec<NodeInfo>, SettingsError> {
        debug!("Getting all node metrics");

        match crate::monitoring_etcd::get_all_nodes().await {
            Ok(nodes) => {
                info!("Retrieved {} node metrics", nodes.len());
                Ok(nodes)
            }
            Err(e) => {
                warn!("Failed to get node metrics: {}", e);
                Err(SettingsError::Metrics(format!(
                    "Failed to get node metrics: {}",
                    e
                )))
            }
        }
    }

    /// Get all container metrics - SIMPLIFIED to use monitoring_etcd directly
    pub async fn get_container_metrics(&mut self) -> Result<Vec<ContainerInfo>, SettingsError> {
        debug!("Getting all container metrics");

        crate::monitoring_etcd::get_all_containers()
            .await
            .map_err(|e| {
                SettingsError::Storage(crate::settings_utils::error::StorageError::OperationFailed(
                    format!("Failed to get containers: {}", e),
                ))
            })
    }

    /// Get container metric by ID
    pub async fn get_container_metric_by_id(
        &mut self,
        container_id: &str,
    ) -> Result<Option<ContainerInfo>, SettingsError> {
        debug!("Getting container metric for ID: {}", container_id);

        match crate::monitoring_etcd::get_container_info(container_id).await {
            Ok(container) => Ok(Some(container)),
            Err(crate::monitoring_etcd::MonitoringEtcdError::NotFound) => Ok(None),
            Err(e) => Err(SettingsError::Storage(
                crate::settings_utils::error::StorageError::OperationFailed(format!(
                    "Failed to get container: {}",
                    e
                )),
            )),
        }
    }

    /// Get container logs
    pub async fn get_container_logs(
        &mut self,
        container_id: &str,
    ) -> Result<Vec<String>, SettingsError> {
        debug!("Getting logs for container: {}", container_id);

        crate::monitoring_etcd::get_container_logs(container_id)
            .await
            .map_err(|e| {
                SettingsError::Storage(crate::settings_utils::error::StorageError::OperationFailed(
                    format!("Failed to get container logs: {}", e),
                ))
            })
    }

    /// Get all soc metrics - SIMPLIFIED to use monitoring_etcd directly
    pub async fn get_soc_metrics(&mut self) -> Result<Vec<SocInfo>, SettingsError> {
        debug!("Getting all soc metrics");

        match crate::monitoring_etcd::get_all_socs().await {
            Ok(socs) => {
                info!("Retrieved {} soc metrics", socs.len());
                Ok(socs)
            }
            Err(e) => {
                warn!("Failed to get soc metrics: {}", e);
                Err(SettingsError::Metrics(format!(
                    "Failed to get soc metrics: {}",
                    e
                )))
            }
        }
    }

    /// Get all board metrics - SIMPLIFIED to use monitoring_etcd directly
    pub async fn get_board_metrics(&mut self) -> Result<Vec<BoardInfo>, SettingsError> {
        debug!("Getting all board metrics");

        match crate::monitoring_etcd::get_all_boards().await {
            Ok(boards) => {
                info!("Retrieved {} board metrics", boards.len());
                Ok(boards)
            }
            Err(e) => {
                warn!("Failed to get board metrics: {}", e);
                Err(SettingsError::Metrics(format!(
                    "Failed to get board metrics: {}",
                    e
                )))
            }
        }
    }

    /// Get node metric by name - SIMPLIFIED to use monitoring_etcd directly
    pub async fn get_node_metric_by_name(
        &mut self,
        node_name: &str,
    ) -> Result<Option<NodeInfo>, SettingsError> {
        debug!("Getting node metric for: {}", node_name);

        match crate::monitoring_etcd::get_node_info(node_name).await {
            Ok(node_info) => Ok(Some(node_info)),
            Err(_) => Ok(None),
        }
    }

    /// Delete a metric by component and ID
    pub async fn delete_metric(
        &mut self,
        component: &str,
        metric_id: &str,
    ) -> Result<(), SettingsError> {
        debug!(
            "Deleting metric: {} from component: {}",
            metric_id, component
        );

        match component {
            "nodes" => {
                crate::monitoring_etcd::delete_node_info(metric_id)
                    .await
                    .map_err(|e| {
                        SettingsError::Metrics(format!("Failed to delete node metric: {}", e))
                    })?;
            }
            "containers" => {
                crate::monitoring_etcd::delete_container_info(metric_id)
                    .await
                    .map_err(|e| {
                        SettingsError::Metrics(format!("Failed to delete container metric: {}", e))
                    })?;
            }
            "socs" => {
                crate::monitoring_etcd::delete_soc_info(metric_id)
                    .await
                    .map_err(|e| {
                        SettingsError::Metrics(format!("Failed to delete SoC metric: {}", e))
                    })?;
            }
            "boards" => {
                crate::monitoring_etcd::delete_board_info(metric_id)
                    .await
                    .map_err(|e| {
                        SettingsError::Metrics(format!("Failed to delete board metric: {}", e))
                    })?;
            }
            _ => {
                return Err(SettingsError::Metrics(format!(
                    "Unknown component: {}",
                    component
                )));
            }
        }

        // Clear cache
        let mut cache = self.cache.write().unwrap();
        cache.clear();

        info!("Deleted metric {} from component {}", metric_id, component);
        Ok(())
    }

    /// Get metric summary statistics
    pub async fn get_metric_stats(&mut self) -> Result<HashMap<String, usize>, SettingsError> {
        let metrics = self.get_metrics(None).await?;
        let mut stats = HashMap::new();

        // Count by component
        for metric in &metrics {
            let count = stats
                .entry(format!("{}_count", metric.component))
                .or_insert(0);
            *count += 1;
        }

        stats.insert("total_metrics".to_string(), metrics.len());
        Ok(stats)
    }

    /// Create a new metrics filter
    pub async fn create_filter(&mut self, filter: &MetricsFilter) -> Result<String, SettingsError> {
        let filter_id = Uuid::new_v4().to_string();
        let mut filter_with_id = filter.clone();
        filter_with_id.id = filter_id.clone();
        filter_with_id.version = 1;
        filter_with_id.created_at = Utc::now();
        filter_with_id.modified_at = Utc::now();

        info!("Creating metrics filter: {} ({})", filter.name, filter_id);

        let key = filter_key(&filter_id);
        let filter_value = serde_json::to_value(&filter_with_id)
            .map_err(|e| SettingsError::Metrics(format!("Failed to serialize filter: {}", e)))?;

        self.storage.put_json(&key, &filter_value).await?;
        Ok(filter_id)
    }

    /// Get filter by ID
    pub async fn get_filter(&mut self, id: &str) -> Result<MetricsFilter, SettingsError> {
        debug!("Getting filter: {}", id);

        let key = filter_key(id);
        if let Some(filter_data) = self.storage.get_json(&key).await? {
            let filter: MetricsFilter = serde_json::from_value(filter_data).map_err(|e| {
                SettingsError::Metrics(format!("Failed to deserialize filter: {}", e))
            })?;
            Ok(filter)
        } else {
            Err(SettingsError::Metrics(format!("Filter not found: {}", id)))
        }
    }

    /// Update filter
    pub async fn update_filter(
        &mut self,
        id: &str,
        filter: &MetricsFilter,
    ) -> Result<(), SettingsError> {
        info!("Updating filter: {}", id);

        let existing_filter = self.get_filter(id).await?;

        let mut updated_filter = filter.clone();
        updated_filter.id = id.to_string();
        updated_filter.version = existing_filter.version + 1;
        updated_filter.created_at = existing_filter.created_at;
        updated_filter.modified_at = Utc::now();

        let key = filter_key(id);
        let filter_value = serde_json::to_value(&updated_filter)
            .map_err(|e| SettingsError::Metrics(format!("Failed to serialize filter: {}", e)))?;

        self.storage.put_json(&key, &filter_value).await?;
        self.invalidate_cache(&format!("filter:{}", id));

        Ok(())
    }

    /// Get metric by ID
    pub async fn get_metric_by_id(
        &mut self,
        metric_id: &str,
    ) -> Result<Option<Metric>, SettingsError> {
        debug!("Getting metric by ID: {}", metric_id);

        // Try to find in different component types

        // Check if it's a node metric
        if let Ok(node_info) = crate::monitoring_etcd::get_node_info(metric_id).await {
            let metric = Metric {
                id: node_info.node_name.clone(),
                component: "node".to_string(),
                metric_type: "NodeInfo".to_string(),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("node_name".to_string(), node_info.node_name.clone());
                    labels.insert("ip".to_string(), node_info.ip.clone());
                    labels
                },
                value: MetricValue::NodeInfo { value: node_info },
                timestamp: Utc::now(),
            };
            return Ok(Some(metric));
        }

        // Check if it's a container metric
        if let Ok(container_info) = crate::monitoring_etcd::get_container_info(metric_id).await {
            let metric = Metric {
                id: container_info.id.clone(),
                component: "container".to_string(),
                metric_type: "ContainerInfo".to_string(),
                labels: {
                    let mut labels = HashMap::new();
                    labels.insert("container_id".to_string(), container_info.id.clone());
                    labels.insert("image".to_string(), container_info.image.clone());
                    labels
                },
                value: MetricValue::ContainerInfo {
                    value: container_info,
                },
                timestamp: Utc::now(),
            };
            return Ok(Some(metric));
        }

        Ok(None)
    }

    /// Get metrics by component
    pub async fn get_metrics_by_component(
        &mut self,
        component: &str,
    ) -> Result<Vec<Metric>, SettingsError> {
        debug!("Getting metrics for component: {}", component);

        let filter = MetricsFilter {
            id: format!("component_{}", component),
            name: format!("Filter for component {}", component),
            enabled: true,
            components: Some(vec![component.to_string()]),
            metric_types: None,
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        self.get_metrics(Some(&filter)).await
    }

    /// Get metrics by type
    pub async fn get_metrics_by_type(
        &mut self,
        metric_type: &str,
    ) -> Result<Vec<Metric>, SettingsError> {
        debug!("Getting metrics for type: {}", metric_type);

        let filter = MetricsFilter {
            id: format!("type_{}", metric_type),
            name: format!("Filter for type {}", metric_type),
            enabled: true,
            components: None,
            metric_types: Some(vec![metric_type.to_string()]),
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        self.get_metrics(Some(&filter)).await
    }

    /// Delete filter
    pub async fn delete_filter(&mut self, id: &str) -> Result<(), SettingsError> {
        info!("Deleting filter: {}", id);

        let key = filter_key(id);
        if !self.storage.delete(&key).await? {
            warn!("Filter not found for deletion: {}", id);
        }

        self.invalidate_cache(&format!("filter:{}", id));
        Ok(())
    }

    /// List all filters
    pub async fn list_filters(&mut self) -> Result<Vec<FilterSummary>, SettingsError> {
        debug!("Listing all filters");

        let prefix = crate::settings_storage::KeyPrefixes::FILTERS;
        let entries = self.storage.list(prefix).await?;
        let mut summaries = Vec::new();

        for (key, value) in entries {
            match serde_json::from_str::<MetricsFilter>(&value) {
                Ok(filter) => {
                    summaries.push(FilterSummary {
                        id: filter.id,
                        name: filter.name,
                        enabled: filter.enabled,
                        component_count: filter.components.as_ref().map_or(0, |c| c.len()),
                        metric_type_count: filter.metric_types.as_ref().map_or(0, |t| t.len()),
                        version: filter.version,
                        created_at: filter.created_at,
                        modified_at: filter.modified_at,
                    });
                }
                Err(e) => {
                    warn!("Failed to parse filter from key {}: {}", key, e);
                }
            }
        }

        Ok(summaries)
    }

    /// Cache management methods
    fn get_cached(&self, key: &str) -> Option<Vec<Metric>> {
        let cache = self.cache.read().ok()?;
        let entry = cache.get(key)?;

        if entry.expiry > Instant::now() {
            Some(entry.data.clone())
        } else {
            None
        }
    }

    fn set_cached(&self, key: &str, metrics: Vec<Metric>) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                key.to_string(),
                CacheEntry {
                    data: metrics,
                    expiry: Instant::now() + self.cache_ttl,
                },
            );
        }
    }

    fn invalidate_cache(&self, key: &str) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(key);
        }
    }

    /// Clear all cache entries
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        debug!("Cleared metrics cache");
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        if let Ok(cache) = self.cache.read() {
            stats.insert("total_entries".to_string(), cache.len());

            let valid_entries = cache
                .values()
                .filter(|entry| entry.expiry > Instant::now())
                .count();

            stats.insert("valid_entries".to_string(), valid_entries);
            stats.insert("expired_entries".to_string(), cache.len() - valid_entries);
        }

        stats
    }

    /// Check if a metric matches the given filter criteria
    fn metric_matches_filter(&self, metric: &Metric, filter: Option<&MetricsFilter>) -> bool {
        let Some(filter) = filter else {
            return true; // No filter means all metrics match
        };

        if !filter.enabled {
            return false;
        }

        // Check component filter
        if let Some(ref components) = filter.components {
            if !components.contains(&metric.component) {
                return false;
            }
        }

        // Check metric type filter
        if let Some(ref metric_types) = filter.metric_types {
            if !metric_types.contains(&metric.metric_type) {
                return false;
            }
        }

        // Check label selectors
        if let Some(ref label_selectors) = filter.label_selectors {
            for (selector_key, selector_value) in label_selectors {
                match metric.labels.get(selector_key) {
                    Some(metric_value) => {
                        // Support simple wildcard matching with '*'
                        if selector_value.contains('*') {
                            if !self.simple_wildcard_match(selector_value, metric_value) {
                                return false;
                            }
                        } else {
                            // Exact match
                            if metric_value != selector_value {
                                return false;
                            }
                        }
                    }
                    None => return false, // Label not found
                }
            }
        }

        // Check time range filter
        if let Some(ref time_range) = filter.time_range {
            if metric.timestamp < time_range.start {
                return false;
            }

            if let Some(end) = time_range.end {
                if metric.timestamp > end {
                    return false;
                }
            }
        }

        true // All checks passed
    }

    /// Simple wildcard matching for label selectors
    /// Supports '*' as a wildcard character
    fn simple_wildcard_match(&self, pattern: &str, text: &str) -> bool {
        if pattern == "*" {
            return true; // Match everything
        }

        if pattern.starts_with('*') && pattern.ends_with('*') {
            // Pattern like "*abc*" - check if text contains the middle part
            let middle = &pattern[1..pattern.len() - 1];
            text.contains(middle)
        } else if pattern.starts_with('*') {
            // Pattern like "*abc" - check if text ends with the suffix
            let suffix = &pattern[1..];
            text.ends_with(suffix)
        } else if pattern.ends_with('*') {
            // Pattern like "abc*" - check if text starts with the prefix
            let prefix = &pattern[..pattern.len() - 1];
            text.starts_with(prefix)
        } else {
            // No wildcards - exact match
            pattern == text
        }
    }

    /// Get containers by node name with proper hostname extraction
    pub async fn get_containers_by_node(
        &mut self,
        node_name: &str,
    ) -> Result<Vec<ContainerInfo>, SettingsError> {
        debug!("Getting containers for node: {}", node_name);

        // Get all containers first
        let all_containers = self.get_container_metrics().await?;

        // Filter containers by node_name using hostname or node_name fields
        let filtered_containers: Vec<ContainerInfo> = all_containers
            .into_iter()
            .filter(|container| {
                // Check multiple sources for node identification
                self.container_belongs_to_node(container, node_name)
            })
            .collect();

        debug!(
            "Found {} containers for node {}",
            filtered_containers.len(),
            node_name
        );
        Ok(filtered_containers)
    }

    /// Check if container belongs to specific node using hostname and node_name
    fn container_belongs_to_node(&self, container: &ContainerInfo, node_name: &str) -> bool {
        // Match node_name against config.Hostname, state["Hostname"], annotation["hostname"], etc.
        container
            .config
            .get("Hostname")
            .map_or(false, |h| h == node_name)
            || container
                .state
                .get("Hostname")
                .map_or(false, |h| h == node_name)
            || container
                .annotation
                .get("hostname")
                .map_or(false, |h| h == node_name)
            || container
                .annotation
                .get("node_name")
                .map_or(false, |h| h == node_name)
            || container
                .state
                .get("node_name")
                .map_or(false, |h| h == node_name)
            || container
                .config
                .get("node_name")
                .map_or(false, |h| h == node_name)
    }

    /// Get pod metrics for a specific node (enhanced to include hostname)
    pub async fn get_pod_metrics_for_node(
        &mut self,
        node_name: &str,
    ) -> Result<Vec<Metric>, SettingsError> {
        debug!("Getting pod metrics for node: {}", node_name);

        // Get containers for the specific node
        let containers = self.get_containers_by_node(node_name).await?;

        let mut metrics = Vec::new();

        for container in containers {
            // Create container metric with node_name and hostname labels
            let mut labels = HashMap::new();
            labels.insert("container_id".to_string(), container.id.clone());
            labels.insert("image".to_string(), container.image.clone());
            labels.insert("node_name".to_string(), node_name.to_string());

            // Add container name if available
            if let Some(name) = container.names.first() {
                labels.insert("container_name".to_string(), name.clone());
            }

            // Add hostname from container annotation or state
            if let Some(hostname) = container
                .annotation
                .get("hostname")
                .or_else(|| container.state.get("hostname"))
                .or_else(|| container.config.get("hostname"))
            {
                labels.insert("hostname".to_string(), hostname.clone());
            }

            // Add status if available
            if let Some(status) = container.state.get("Status") {
                labels.insert("status".to_string(), status.clone());
            }

            let metric = Metric {
                id: format!("pod_{}_{}", node_name, container.id),
                component: "pod".to_string(),
                metric_type: "PodInfo".to_string(),
                labels,
                value: MetricValue::ContainerInfo { value: container },
                timestamp: Utc::now(),
            };

            metrics.push(metric);
        }

        info!(
            "Retrieved {} pod metrics for node {}",
            metrics.len(),
            node_name
        );
        Ok(metrics)
    }

    /// Get containers with enhanced node information
    pub async fn get_container_metrics_with_node_info(
        &mut self,
    ) -> Result<Vec<ContainerInfo>, SettingsError> {
        debug!("Getting all container metrics with node information");

        let mut containers = self.get_container_metrics().await?;

        // Enhance each container with node information
        for container in &mut containers {
            self.ensure_node_hostname_in_container(container);
        }

        Ok(containers)
    }

    /// Ensure container has both node_name and hostname information
    fn ensure_node_hostname_in_container(&self, container: &mut ContainerInfo) {
        // If we have hostname but not node_name, use hostname as node_name
        if !container.annotation.contains_key("node_name") {
            let hostname = container
                .annotation
                .get("hostname")
                .or_else(|| container.state.get("hostname"))
                .or_else(|| container.config.get("hostname"))
                .cloned();

            if let Some(hostname) = hostname {
                container
                    .annotation
                    .insert("node_name".to_string(), hostname.clone());
                container.state.insert("node_name".to_string(), hostname);
            }
        }

        // If we have node_name but not hostname, use node_name as hostname
        if !container.annotation.contains_key("hostname") {
            let node_name = container
                .annotation
                .get("node_name")
                .or_else(|| container.state.get("node_name"))
                .or_else(|| container.config.get("node_name"))
                .cloned();

            if let Some(node_name) = node_name {
                container
                    .annotation
                    .insert("hostname".to_string(), node_name.clone());
                container.state.insert("hostname".to_string(), node_name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_storage::Storage;
    use crate::settings_utils::error::StorageError;
    use async_trait::async_trait;
    use serde_json::Value;

    // Mock storage implementation for testing
    #[derive(Debug)]
    struct MockStorage {
        data: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                data: std::sync::Arc::new(tokio::sync::RwLock::new(
                    std::collections::HashMap::new(),
                )),
            }
        }
    }

    #[async_trait]
    impl Storage for MockStorage {
        async fn get(&mut self, key: &str) -> Result<Option<String>, StorageError> {
            let data = self.data.read().await;
            Ok(data.get(key).cloned())
        }

        async fn put(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
            let mut data = self.data.write().await;
            data.insert(key.to_string(), value.to_string());
            Ok(())
        }

        async fn delete(&mut self, key: &str) -> Result<bool, StorageError> {
            let mut data = self.data.write().await;
            Ok(data.remove(key).is_some())
        }

        async fn list(&mut self, prefix: &str) -> Result<Vec<(String, String)>, StorageError> {
            let data = self.data.read().await;
            Ok(data
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect())
        }

        async fn get_json(&mut self, key: &str) -> Result<Option<Value>, StorageError> {
            if let Some(value) = self.get(key).await? {
                let json_value: Value = serde_json::from_str(&value)
                    .map_err(|e| StorageError::SerializationError(e.to_string()))?;
                Ok(Some(json_value))
            } else {
                Ok(None)
            }
        }

        async fn put_json(&mut self, key: &str, value: &Value) -> Result<(), StorageError> {
            let json_string = serde_json::to_string(value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            self.put(key, &json_string).await
        }
    }
    use chrono::TimeZone;
    use std::collections::HashMap;
    use tokio::time::{sleep, Duration as TokioDuration};

    fn create_test_node_info() -> NodeInfo {
        NodeInfo {
            node_name: "test-node".to_string(),
            cpu_usage: 50.0,
            cpu_count: 8,
            gpu_count: 1,
            used_memory: 8192,
            total_memory: 16384,
            mem_usage: 50.0,
            rx_bytes: 1024,
            tx_bytes: 2048,
            read_bytes: 4096,
            write_bytes: 8192,
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            ip: "192.168.1.100".to_string(),
        }
    }

    fn create_test_container_info() -> ContainerInfo {
        let mut state = HashMap::new();
        state.insert("Status".to_string(), "running".to_string());
        state.insert("node_name".to_string(), "test-node".to_string());

        let mut config = HashMap::new();
        config.insert("Image".to_string(), "test:latest".to_string());

        let mut annotation = HashMap::new();
        annotation.insert("hostname".to_string(), "test-node".to_string());

        ContainerInfo {
            id: "container-123".to_string(),
            names: vec!["test-container".to_string()],
            image: "test:latest".to_string(),
            state,
            config,
            annotation,
            stats: HashMap::new(),
        }
    }

    fn create_test_metrics_filter() -> MetricsFilter {
        MetricsFilter {
            id: "test-filter".to_string(),
            name: "Test Filter".to_string(),
            enabled: true,
            components: Some(vec!["node".to_string(), "container".to_string()]),
            metric_types: Some(vec!["NodeInfo".to_string()]),
            label_selectors: None,
            time_range: None,
            refresh_interval: Some(60),
            max_items: Some(100),
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        }
    }

    async fn create_test_monitoring_manager() -> MonitoringManager {
        let storage = Box::new(MockStorage::new());
        MonitoringManager::new(storage, 300) // 5 minutes cache TTL
    }

    #[test]
    fn test_metrics_filter_creation() {
        let filter = create_test_metrics_filter();

        assert_eq!(filter.id, "test-filter");
        assert_eq!(filter.name, "Test Filter");
        assert!(filter.enabled);
        assert_eq!(filter.components.as_ref().unwrap().len(), 2);
        assert_eq!(filter.metric_types.as_ref().unwrap().len(), 1);
        assert_eq!(filter.refresh_interval, Some(60));
        assert_eq!(filter.max_items, Some(100));
        assert_eq!(filter.version, 1);
    }

    #[test]
    fn test_metrics_filter_serialization() {
        let filter = create_test_metrics_filter();

        let serialized = serde_json::to_string(&filter).expect("Failed to serialize filter");
        assert!(serialized.contains("test-filter"));
        assert!(serialized.contains("Test Filter"));
        assert!(serialized.contains("NodeInfo"));

        let deserialized: MetricsFilter =
            serde_json::from_str(&serialized).expect("Failed to deserialize filter");
        assert_eq!(deserialized.id, filter.id);
        assert_eq!(deserialized.name, filter.name);
        assert_eq!(deserialized.enabled, filter.enabled);
    }

    #[test]
    fn test_time_range_creation() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);

        let time_range = TimeRange {
            start,
            end: Some(end),
        };

        assert_eq!(time_range.start, start);
        assert_eq!(time_range.end, Some(end));
    }

    #[test]
    fn test_metric_value_variants() {
        let counter = MetricValue::Counter { value: 42 };
        let gauge = MetricValue::Gauge {
            value: std::f64::consts::PI,
        };
        let node_info = MetricValue::NodeInfo {
            value: create_test_node_info(),
        };
        let container_info = MetricValue::ContainerInfo {
            value: create_test_container_info(),
        };

        match counter {
            MetricValue::Counter { value } => assert_eq!(value, 42),
            _ => panic!("Expected Counter variant"),
        }

        match gauge {
            MetricValue::Gauge { value } => {
                assert!((value - std::f64::consts::PI).abs() < f64::EPSILON)
            }
            _ => panic!("Expected Gauge variant"),
        }

        match node_info {
            MetricValue::NodeInfo { value } => assert_eq!(value.node_name, "test-node"),
            _ => panic!("Expected NodeInfo variant"),
        }

        match container_info {
            MetricValue::ContainerInfo { value } => assert_eq!(value.id, "container-123"),
            _ => panic!("Expected ContainerInfo variant"),
        }
    }

    #[test]
    fn test_histogram_and_summary_types() {
        let buckets = vec![
            HistogramBucket {
                upper_bound: 1.0,
                count: 10,
            },
            HistogramBucket {
                upper_bound: 5.0,
                count: 25,
            },
            HistogramBucket {
                upper_bound: 10.0,
                count: 50,
            },
        ];

        let histogram = MetricValue::Histogram {
            buckets: buckets.clone(),
        };

        match histogram {
            MetricValue::Histogram { buckets: h_buckets } => {
                assert_eq!(h_buckets.len(), 3);
                assert_eq!(h_buckets[0].upper_bound, 1.0);
                assert_eq!(h_buckets[0].count, 10);
            }
            _ => panic!("Expected Histogram variant"),
        }

        let quantiles = vec![
            SummaryQuantile {
                quantile: 0.5,
                value: 10.0,
            },
            SummaryQuantile {
                quantile: 0.95,
                value: 50.0,
            },
            SummaryQuantile {
                quantile: 0.99,
                value: 100.0,
            },
        ];

        let summary = MetricValue::Summary {
            quantiles: quantiles.clone(),
        };

        match summary {
            MetricValue::Summary {
                quantiles: s_quantiles,
            } => {
                assert_eq!(s_quantiles.len(), 3);
                assert_eq!(s_quantiles[0].quantile, 0.5);
                assert_eq!(s_quantiles[0].value, 10.0);
            }
            _ => panic!("Expected Summary variant"),
        }
    }

    #[test]
    fn test_metric_creation() {
        let mut labels = HashMap::new();
        labels.insert("node_name".to_string(), "test-node".to_string());
        labels.insert("component".to_string(), "cpu".to_string());

        let metric = Metric {
            id: "test-metric".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels,
            value: MetricValue::NodeInfo {
                value: create_test_node_info(),
            },
            timestamp: Utc::now(),
        };

        assert_eq!(metric.id, "test-metric");
        assert_eq!(metric.component, "node");
        assert_eq!(metric.metric_type, "NodeInfo");
        assert_eq!(metric.labels.len(), 2);
        assert!(metric.labels.contains_key("node_name"));
        assert!(metric.labels.contains_key("component"));
    }

    #[test]
    fn test_filter_summary_creation() {
        let now = Utc::now();
        let summary = FilterSummary {
            id: "filter-1".to_string(),
            name: "Test Summary".to_string(),
            enabled: true,
            component_count: 3,
            metric_type_count: 2,
            version: 5,
            created_at: now,
            modified_at: now,
        };

        assert_eq!(summary.id, "filter-1");
        assert_eq!(summary.name, "Test Summary");
        assert!(summary.enabled);
        assert_eq!(summary.component_count, 3);
        assert_eq!(summary.metric_type_count, 2);
        assert_eq!(summary.version, 5);
    }

    #[test]
    fn test_cache_entry_creation() {
        let metrics = vec![Metric {
            id: "metric-1".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels: HashMap::new(),
            value: MetricValue::Counter { value: 42 },
            timestamp: Utc::now(),
        }];

        let expiry = Instant::now() + Duration::from_secs(300);
        let cache_entry = CacheEntry {
            data: metrics.clone(),
            expiry,
        };

        assert_eq!(cache_entry.data.len(), 1);
        assert_eq!(cache_entry.data[0].id, "metric-1");
        assert!(cache_entry.expiry > Instant::now());
    }

    #[tokio::test]
    async fn test_monitoring_manager_creation() {
        let manager = create_test_monitoring_manager().await;
        let stats = manager.get_cache_stats();

        assert_eq!(stats.get("total_entries").unwrap_or(&0), &0);
        assert_eq!(stats.get("valid_entries").unwrap_or(&0), &0);
        assert_eq!(stats.get("expired_entries").unwrap_or(&0), &0);
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let manager = create_test_monitoring_manager().await;

        // Initially empty cache
        let stats = manager.get_cache_stats();
        assert_eq!(stats.get("total_entries").unwrap_or(&0), &0);

        // Add to cache
        let metrics = vec![Metric {
            id: "test-metric".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels: HashMap::new(),
            value: MetricValue::Counter { value: 100 },
            timestamp: Utc::now(),
        }];

        manager.set_cached("test-key", metrics.clone());

        // Check cache stats
        let stats = manager.get_cache_stats();
        assert_eq!(stats.get("total_entries").unwrap_or(&0), &1);
        assert_eq!(stats.get("valid_entries").unwrap_or(&0), &1);

        // Retrieve from cache
        let cached_metrics = manager.get_cached("test-key");
        assert!(cached_metrics.is_some());
        assert_eq!(cached_metrics.unwrap().len(), 1);

        // Clear cache
        manager.clear_cache();
        let stats = manager.get_cache_stats();
        assert_eq!(stats.get("total_entries").unwrap_or(&0), &0);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let storage = Box::new(MockStorage::new());
        let manager = MonitoringManager::new(storage, 1); // 1 second TTL

        let metrics = vec![Metric {
            id: "expiring-metric".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels: HashMap::new(),
            value: MetricValue::Counter { value: 200 },
            timestamp: Utc::now(),
        }];

        manager.set_cached("expiring-key", metrics);

        // Should be cached initially
        assert!(manager.get_cached("expiring-key").is_some());

        // Wait for expiration
        sleep(TokioDuration::from_secs(2)).await;

        // Should be expired now
        assert!(manager.get_cached("expiring-key").is_none());
    }

    #[test]
    fn test_simple_wildcard_matching() {
        let manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        // Test exact match
        assert!(manager.simple_wildcard_match("test", "test"));
        assert!(!manager.simple_wildcard_match("test", "other"));

        // Test wildcard match
        assert!(manager.simple_wildcard_match("*", "anything"));
        assert!(manager.simple_wildcard_match("test*", "test123"));
        assert!(manager.simple_wildcard_match("*test", "123test"));
        assert!(manager.simple_wildcard_match("*test*", "123test456"));

        // Test negative cases
        assert!(!manager.simple_wildcard_match("test*", "other123"));
        assert!(!manager.simple_wildcard_match("*test", "123other"));
        assert!(!manager.simple_wildcard_match("*test*", "123other456"));
    }

    #[test]
    fn test_metric_matches_filter() {
        let manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        let mut labels = HashMap::new();
        labels.insert("node_name".to_string(), "test-node".to_string());
        labels.insert("env".to_string(), "production".to_string());

        let metric = Metric {
            id: "test-metric".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels,
            value: MetricValue::Counter { value: 42 },
            timestamp: Utc::now(),
        };

        // Test with no filter (should match)
        assert!(manager.metric_matches_filter(&metric, None));

        // Test with disabled filter (should not match)
        let disabled_filter = MetricsFilter {
            id: "disabled".to_string(),
            name: "Disabled Filter".to_string(),
            enabled: false,
            components: None,
            metric_types: None,
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(!manager.metric_matches_filter(&metric, Some(&disabled_filter)));

        // Test with component filter (should match)
        let component_filter = MetricsFilter {
            id: "component".to_string(),
            name: "Component Filter".to_string(),
            enabled: true,
            components: Some(vec!["node".to_string()]),
            metric_types: None,
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(manager.metric_matches_filter(&metric, Some(&component_filter)));

        // Test with wrong component filter (should not match)
        let wrong_component_filter = MetricsFilter {
            id: "wrong".to_string(),
            name: "Wrong Component Filter".to_string(),
            enabled: true,
            components: Some(vec!["container".to_string()]),
            metric_types: None,
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(!manager.metric_matches_filter(&metric, Some(&wrong_component_filter)));
    }

    #[test]
    fn test_metric_matches_filter_with_labels() {
        let manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        let mut labels = HashMap::new();
        labels.insert("node_name".to_string(), "test-node-001".to_string());
        labels.insert("env".to_string(), "production".to_string());

        let metric = Metric {
            id: "test-metric".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels,
            value: MetricValue::Counter { value: 42 },
            timestamp: Utc::now(),
        };

        // Test with exact label selector (should match)
        let mut label_selectors = HashMap::new();
        label_selectors.insert("env".to_string(), "production".to_string());

        let label_filter = MetricsFilter {
            id: "label".to_string(),
            name: "Label Filter".to_string(),
            enabled: true,
            components: None,
            metric_types: None,
            label_selectors: Some(label_selectors),
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(manager.metric_matches_filter(&metric, Some(&label_filter)));

        // Test with wildcard label selector (should match)
        let mut wildcard_selectors = HashMap::new();
        wildcard_selectors.insert("node_name".to_string(), "test-node-*".to_string());

        let wildcard_filter = MetricsFilter {
            id: "wildcard".to_string(),
            name: "Wildcard Filter".to_string(),
            enabled: true,
            components: None,
            metric_types: None,
            label_selectors: Some(wildcard_selectors),
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(manager.metric_matches_filter(&metric, Some(&wildcard_filter)));
    }

    #[test]
    fn test_metric_matches_filter_with_time_range() {
        let manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        let now = Utc::now();
        let metric = Metric {
            id: "test-metric".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels: HashMap::new(),
            value: MetricValue::Counter { value: 42 },
            timestamp: now,
        };

        // Test with time range that includes the metric (should match)
        let time_range = TimeRange {
            start: now - chrono::Duration::minutes(5),
            end: Some(now + chrono::Duration::minutes(5)),
        };

        let time_filter = MetricsFilter {
            id: "time".to_string(),
            name: "Time Filter".to_string(),
            enabled: true,
            components: None,
            metric_types: None,
            label_selectors: None,
            time_range: Some(time_range),
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(manager.metric_matches_filter(&metric, Some(&time_filter)));

        // Test with time range that excludes the metric (should not match)
        let past_time_range = TimeRange {
            start: now - chrono::Duration::hours(2),
            end: Some(now - chrono::Duration::hours(1)),
        };

        let past_time_filter = MetricsFilter {
            id: "past".to_string(),
            name: "Past Time Filter".to_string(),
            enabled: true,
            components: None,
            metric_types: None,
            label_selectors: None,
            time_range: Some(past_time_range),
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        assert!(!manager.metric_matches_filter(&metric, Some(&past_time_filter)));
    }

    #[test]
    fn test_container_belongs_to_node() {
        let manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        // Test container with hostname in config
        let mut config_container = create_test_container_info();
        config_container
            .config
            .insert("Hostname".to_string(), "test-node".to_string());
        assert!(manager.container_belongs_to_node(&config_container, "test-node"));
        assert!(!manager.container_belongs_to_node(&config_container, "other-node"));

        // Test container with hostname in state
        let mut state_container = create_test_container_info();
        state_container
            .state
            .insert("Hostname".to_string(), "test-node".to_string());
        assert!(manager.container_belongs_to_node(&state_container, "test-node"));

        // Test container with hostname in annotation
        let mut annotation_container = create_test_container_info();
        annotation_container
            .annotation
            .insert("hostname".to_string(), "test-node".to_string());
        assert!(manager.container_belongs_to_node(&annotation_container, "test-node"));

        // Test container with node_name in annotation
        let mut node_name_container = create_test_container_info();
        node_name_container
            .annotation
            .insert("node_name".to_string(), "test-node".to_string());
        assert!(manager.container_belongs_to_node(&node_name_container, "test-node"));
    }

    #[test]
    fn test_ensure_node_hostname_in_container() {
        let manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        // Test container with only hostname
        let mut container_with_hostname = create_test_container_info();
        container_with_hostname.annotation.clear();
        container_with_hostname
            .annotation
            .insert("hostname".to_string(), "test-host".to_string());

        manager.ensure_node_hostname_in_container(&mut container_with_hostname);

        assert_eq!(
            container_with_hostname.annotation.get("node_name"),
            Some(&"test-host".to_string())
        );
        assert_eq!(
            container_with_hostname.state.get("node_name"),
            Some(&"test-host".to_string())
        );

        // Test container with only node_name
        let mut container_with_node_name = create_test_container_info();
        container_with_node_name.annotation.clear();
        container_with_node_name
            .annotation
            .insert("node_name".to_string(), "test-node".to_string());

        manager.ensure_node_hostname_in_container(&mut container_with_node_name);

        assert_eq!(
            container_with_node_name.annotation.get("hostname"),
            Some(&"test-node".to_string())
        );
        assert_eq!(
            container_with_node_name.state.get("hostname"),
            Some(&"test-node".to_string())
        );
    }

    #[test]
    fn test_default_version() {
        assert_eq!(default_version(), 1);
    }

    #[test]
    fn test_response_structures() {
        let nodes = vec![create_test_node_info()];
        let node_response = NodeListResponse {
            nodes: nodes.clone(),
            total: nodes.len(),
        };

        assert_eq!(node_response.nodes.len(), 1);
        assert_eq!(node_response.total, 1);
        assert_eq!(node_response.nodes[0].node_name, "test-node");

        let containers = vec![create_test_container_info()];
        let container_response = SocListResponse {
            socs: vec![], // Using empty socs for this test
            total: 0,
        };

        assert_eq!(container_response.socs.len(), 0);
        assert_eq!(container_response.total, 0);
    }

    #[tokio::test]
    async fn test_filter_operations() {
        let mut manager = create_test_monitoring_manager().await;
        let filter = create_test_metrics_filter();

        // Create filter
        let filter_id = manager.create_filter(&filter).await;
        assert!(filter_id.is_ok());

        let created_id = filter_id.unwrap();
        assert!(!created_id.is_empty());

        // Get filter
        let retrieved_filter = manager.get_filter(&created_id).await;
        assert!(retrieved_filter.is_ok());

        let retrieved = retrieved_filter.unwrap();
        assert_eq!(retrieved.name, filter.name);
        assert_eq!(retrieved.enabled, filter.enabled);

        // Update filter
        let mut updated_filter = retrieved.clone();
        updated_filter.name = "Updated Test Filter".to_string();
        updated_filter.enabled = false;

        let update_result = manager.update_filter(&created_id, &updated_filter).await;
        assert!(update_result.is_ok());

        // Verify update
        let updated_retrieved = manager.get_filter(&created_id).await.unwrap();
        assert_eq!(updated_retrieved.name, "Updated Test Filter");
        assert!(!updated_retrieved.enabled);
        assert_eq!(updated_retrieved.version, retrieved.version + 1);

        // Delete filter
        let delete_result = manager.delete_filter(&created_id).await;
        assert!(delete_result.is_ok());

        // Verify deletion
        let deleted_retrieved = manager.get_filter(&created_id).await;
        assert!(deleted_retrieved.is_err());
    }

    #[tokio::test]
    async fn test_list_filters() {
        let mut manager = create_test_monitoring_manager().await;

        // Initially empty
        let initial_list = manager.list_filters().await.unwrap();
        assert_eq!(initial_list.len(), 0);

        // Create some filters
        let filter1 = MetricsFilter {
            id: "".to_string(),
            name: "Filter 1".to_string(),
            enabled: true,
            components: Some(vec!["node".to_string()]),
            metric_types: Some(vec!["NodeInfo".to_string()]),
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        let filter2 = MetricsFilter {
            id: "".to_string(),
            name: "Filter 2".to_string(),
            enabled: false,
            components: Some(vec!["container".to_string(), "pod".to_string()]),
            metric_types: Some(vec!["ContainerInfo".to_string(), "PodInfo".to_string()]),
            label_selectors: None,
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        let _id1 = manager.create_filter(&filter1).await.unwrap();
        let _id2 = manager.create_filter(&filter2).await.unwrap();

        // List filters
        let filter_list = manager.list_filters().await.unwrap();
        assert_eq!(filter_list.len(), 2);

        // Check filter summaries
        let summary1 = filter_list.iter().find(|f| f.name == "Filter 1").unwrap();
        assert!(summary1.enabled);
        assert_eq!(summary1.component_count, 1);
        assert_eq!(summary1.metric_type_count, 1);

        let summary2 = filter_list.iter().find(|f| f.name == "Filter 2").unwrap();
        assert!(!summary2.enabled);
        assert_eq!(summary2.component_count, 2);
        assert_eq!(summary2.metric_type_count, 2);
    }

    #[test]
    fn test_metric_serialization() {
        let mut labels = HashMap::new();
        labels.insert("test_label".to_string(), "test_value".to_string());

        let metric = Metric {
            id: "serialize-test".to_string(),
            component: "test".to_string(),
            metric_type: "TestInfo".to_string(),
            labels,
            value: MetricValue::Counter { value: 999 },
            timestamp: Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap(),
        };

        let serialized = serde_json::to_string(&metric).expect("Failed to serialize metric");
        assert!(serialized.contains("serialize-test"));
        assert!(serialized.contains("test"));
        assert!(serialized.contains("TestInfo"));
        assert!(serialized.contains("999"));

        let deserialized: Metric =
            serde_json::from_str(&serialized).expect("Failed to deserialize metric");
        assert_eq!(deserialized.id, metric.id);
        assert_eq!(deserialized.component, metric.component);
        assert_eq!(deserialized.metric_type, metric.metric_type);
        assert_eq!(deserialized.labels.len(), 1);
    }

    #[test]
    fn test_error_scenarios() {
        let mut manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        // Test invalid cache operations
        let invalid_cached = manager.get_cached("non-existent-key");
        assert!(invalid_cached.is_none());

        // Test cache invalidation
        manager.set_cached("test-key", vec![]);
        assert!(manager.get_cached("test-key").is_some());

        manager.invalidate_cache("test-key");
        assert!(manager.get_cached("test-key").is_none());
    }

    #[tokio::test]
    async fn test_complex_filtering_scenarios() {
        let manager = MonitoringManager {
            storage: Box::new(MockStorage::new()),
            cache: RwLock::new(HashMap::new()),
            cache_ttl: Duration::from_secs(300),
        };

        // Create metric with multiple labels
        let mut labels = HashMap::new();
        labels.insert("env".to_string(), "production".to_string());
        labels.insert("region".to_string(), "us-west-2".to_string());
        labels.insert("cluster".to_string(), "main-cluster".to_string());

        let metric = Metric {
            id: "complex-metric".to_string(),
            component: "node".to_string(),
            metric_type: "NodeInfo".to_string(),
            labels,
            value: MetricValue::Gauge { value: 75.5 },
            timestamp: Utc::now(),
        };

        // Test multiple label selectors
        let mut complex_selectors = HashMap::new();
        complex_selectors.insert("env".to_string(), "production".to_string());
        complex_selectors.insert("region".to_string(), "us-*".to_string());

        let complex_filter = MetricsFilter {
            id: "complex".to_string(),
            name: "Complex Filter".to_string(),
            enabled: true,
            components: Some(vec!["node".to_string()]),
            metric_types: Some(vec!["NodeInfo".to_string()]),
            label_selectors: Some(complex_selectors),
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        // Should match all criteria
        assert!(manager.metric_matches_filter(&metric, Some(&complex_filter)));

        // Test with mismatched label
        let mut mismatched_selectors = HashMap::new();
        mismatched_selectors.insert("env".to_string(), "staging".to_string());

        let mismatched_filter = MetricsFilter {
            id: "mismatched".to_string(),
            name: "Mismatched Filter".to_string(),
            enabled: true,
            components: Some(vec!["node".to_string()]),
            metric_types: Some(vec!["NodeInfo".to_string()]),
            label_selectors: Some(mismatched_selectors),
            time_range: None,
            refresh_interval: None,
            max_items: None,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };

        // Should not match
        assert!(!manager.metric_matches_filter(&metric, Some(&mismatched_filter)));
    }
}

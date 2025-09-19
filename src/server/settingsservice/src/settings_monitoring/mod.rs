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

#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub name: String,
    pub image: String,
    pub labels: std::collections::HashMap<String, String>,
    pub ip: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSocRequest {
    pub name: String,
    pub description: Option<String>,
    pub labels: std::collections::HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
    pub description: Option<String>,
    pub labels: std::collections::HashMap<String, String>,
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
                            labels // RETURN the labels HashMap
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

        match crate::monitoring_etcd::get_all_containers().await {
            Ok(containers) => {
                info!("Retrieved {} container metrics", containers.len());
                Ok(containers)
            }
            Err(e) => {
                warn!("Failed to get container metrics: {}", e);
                Err(SettingsError::Metrics(format!(
                    "Failed to get container metrics: {}",
                    e
                )))
            }
        }
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

    /// Get container metric by ID - SIMPLIFIED to use monitoring_etcd directly
    pub async fn get_container_metric_by_id(
        &mut self,
        container_id: &str,
    ) -> Result<Option<ContainerInfo>, SettingsError> {
        debug!("Getting container metric for: {}", container_id);

        match crate::monitoring_etcd::get_container_info(container_id).await {
            Ok(container_info) => Ok(Some(container_info)),
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
}

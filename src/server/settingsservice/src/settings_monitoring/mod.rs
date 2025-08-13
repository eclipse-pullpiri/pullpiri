// SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
// SPDX-License-Identifier: Apache-2.0

//! Monitoring and metrics management module

use crate::settings_storage::{Storage, metrics_key, filter_key};
use crate::settings_utils::error::SettingsError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

fn default_version() -> u64 { 1 }

/// Time range for metrics filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
}

/// Metrics data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub id: String,
    pub component: String,
    pub metric_type: String,
    pub labels: HashMap<String, String>,
    pub value: MetricValue,
    pub timestamp: DateTime<Utc>,
}

/// Metric value types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MetricValue {
    Counter { value: u64 },
    Gauge { value: f64 },
    Histogram { buckets: Vec<HistogramBucket> },
    Summary { quantiles: Vec<SummaryQuantile> },
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

/// Monitoring manager for metrics filtering and caching
pub struct MonitoringManager {
    storage: Box<dyn Storage>,
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

        // Fetch from storage
        let metrics = self.fetch_metrics_from_storage(filter).await?;

        // Cache the results
        self.set_cached(&cache_key, metrics.clone());

        Ok(metrics)
    }

    /// Fetch metrics from ETCD storage
    async fn fetch_metrics_from_storage(
        &mut self,
        filter: Option<&MetricsFilter>,
    ) -> Result<Vec<Metric>, SettingsError> {
        let prefix = crate::settings_storage::KeyPrefixes::METRICS;
        let entries = self.storage.list(prefix).await?;

        let mut metrics = Vec::new();

        for (key, value) in entries {
            match serde_json::from_str::<Metric>(&value) {
                Ok(metric) => {
                    if self.metric_matches_filter(&metric, filter) {
                        metrics.push(metric);
                    }
                }
                Err(e) => {
                    warn!("Failed to parse metric from key {}: {}", key, e);
                }
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

        Ok(metrics)
    }

    /// Check if metric matches filter criteria
    fn metric_matches_filter(&self, metric: &Metric, filter: Option<&MetricsFilter>) -> bool {
        let Some(filter) = filter else {
            return true;
        };

        if !filter.enabled {
            return false;
        }

        // Filter by components
        if let Some(components) = &filter.components {
            if !components.contains(&metric.component) {
                return false;
            }
        }

        // Filter by metric types
        if let Some(metric_types) = &filter.metric_types {
            if !metric_types.contains(&metric.metric_type) {
                return false;
            }
        }

        // Filter by labels
        if let Some(label_selectors) = &filter.label_selectors {
            for (key, value) in label_selectors {
                if metric.labels.get(key) != Some(value) {
                    return false;
                }
            }
        }

        // Filter by time range
        if let Some(time_range) = &filter.time_range {
            if metric.timestamp < time_range.start {
                return false;
            }
            if let Some(end) = time_range.end {
                if metric.timestamp > end {
                    return false;
                }
            }
        }

        true
    }

    /// Get metric by ID
    pub async fn get_metric_by_id(&mut self, id: &str) -> Result<Option<Metric>, SettingsError> {
        debug!("Getting metric by ID: {}", id);

        let metrics = self.get_metrics(None).await?;
        Ok(metrics.into_iter().find(|m| m.id == id))
    }

    /// Get metrics by component
    pub async fn get_metrics_by_component(
        &mut self,
        component: &str,
    ) -> Result<Vec<Metric>, SettingsError> {
        debug!("Getting metrics for component: {}", component);

        let filter = MetricsFilter {
            id: format!("component:{}", component),
            name: format!("Component: {}", component),
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
            id: format!("type:{}", metric_type),
            name: format!("Type: {}", metric_type),
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
            let filter: MetricsFilter = serde_json::from_value(filter_data)
                .map_err(|e| SettingsError::Metrics(format!("Failed to deserialize filter: {}", e)))?;
            Ok(filter)
        } else {
            Err(SettingsError::Metrics(format!("Filter not found: {}", id)))
        }
    }

    /// Update filter
    pub async fn update_filter(&mut self, id: &str, filter: &MetricsFilter) -> Result<(), SettingsError> {
        info!("Updating filter: {}", id);

        // Get the existing filter to preserve version and creation time
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

        // Invalidate related cache
        self.invalidate_cache(&format!("filter:{}", id));

        Ok(())
    }

    /// Delete filter
    pub async fn delete_filter(&mut self, id: &str) -> Result<(), SettingsError> {
        info!("Deleting filter: {}", id);

        let key = filter_key(id);
        if !self.storage.delete(&key).await? {
            warn!("Filter not found for deletion: {}", id);
        }

        // Invalidate related cache
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
}
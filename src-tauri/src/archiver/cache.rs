// cache.rs

use crate::{
    core::error::{ArchiverError, Result},
    types::*,
    metrics::ApiMetrics,
};

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::VecDeque;
use tracing::{debug, warn, error};

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    data: T,
    metadata: CacheMetadata,
    size_bytes: u64,
    timestamp: DateTime<Utc>,
    expires: DateTime<Utc>,
}

/// Metadata for cache entries
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    created_at: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
    resolution: DataResolution,
    validity_period: Duration,
}

/// Cache key components
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pv: String,
    start_time: i64,
    end_time: i64,
    resolution: DataResolution,
}

impl CacheKey {
    pub fn new(pv: String, start_time: i64, end_time: i64, resolution: DataResolution) -> Self {
        Self {
            pv,
            start_time,
            end_time,
            resolution,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}:{}:{}", self.pv, self.start_time, self.end_time, self.resolution.to_string())
    }
}

// cache.rs (continued)

impl CacheManager {
    // Continuing from previous implementation...

    async fn cleanup_caches(
        data_cache: &DashMap<String, CacheEntry<Vec<ProcessedPoint>>>,
        metadata_cache: &DashMap<String, CacheEntry<Meta>>,
        metrics: &ApiMetrics,
        max_memory: u64,
        access_history: &RwLock<VecDeque<(String, DateTime<Utc>)>>,
    ) {
        // ... (previous cleanup code)

        // If still over memory limit, use LRU to remove entries
        if total_memory > max_memory {
            let access_history_guard = access_history.read();
            let lru_entries: Vec<_> = access_history_guard
                .iter()
                .map(|(key, _)| key.clone())
                .collect();

            for key in lru_entries {
                if total_memory <= max_memory {
                    break;
                }

                if let Some(entry) = data_cache.remove(&key) {
                    total_memory -= entry.1.size_bytes;
                    metrics.record_cache_eviction();
                }
            }
        }

        metrics.update_memory_usage(total_memory);
    }

    fn is_entry_expired<T>(&self, entry: &CacheEntry<T>) -> bool {
        Utc::now() - entry.metadata.last_accessed > entry.metadata.validity_period
    }

    fn update_access_stats<T>(&self, key: &str, entry: &mut CacheEntry<T>) {
        entry.metadata.last_accessed = Utc::now();
        entry.metadata.access_count += 1;

        let mut history = self.access_history.write();
        history.push_back((key.to_string(), Utc::now()));

        // Keep history size bounded
        while history.len() > 1000 {
            history.pop_front();
        }
    }

    fn store_with_memory_check(
        &self,
        key: String,
        entry: CacheEntry<Vec<ProcessedPoint>>,
    ) -> Result<()> {
        let new_size = entry.size_bytes;
        let current_size: u64 = self.data_cache.iter().map(|e| e.size_bytes).sum();

        if current_size + new_size > self.max_memory_bytes {
            // Need to make space
            let mut to_remove = Vec::new();
            let mut freed_space = 0u64;
            let needed_space = new_size;

            // Find least recently used entries to remove
            let history = self.access_history.read();
            for (key, _) in history.iter().rev() {
                if let Some(entry) = self.data_cache.get(key) {
                    to_remove.push(key.clone());
                    freed_space += entry.size_bytes;
                    if freed_space >= needed_space {
                        break;
                    }
                }
            }

            // Remove selected entries
            for key in to_remove {
                if let Some(entry) = self.data_cache.remove(&key) {
                    self.metrics.record_cache_eviction();
                    debug!("Evicted cache entry: {}", key);
                }
            }
        }

        self.data_cache.insert(key, entry);
        Ok(())
    }

    fn calculate_size(&self, data: &[ProcessedPoint]) -> u64 {
        let base_size = std::mem::size_of::<ProcessedPoint>();
        (base_size * data.len()) as u64
    }

    fn get_validity_period(resolution: &DataResolution) -> Duration {
        match resolution {
            DataResolution::Raw => Duration::minutes(5),
            DataResolution::Optimized { .. } => Duration::minutes(15),
            DataResolution::Binned { bin_size, .. } => {
                let bin_size_secs = *bin_size as i64;
                if bin_size_secs <= 60 {
                    Duration::minutes(15)
                } else if bin_size_secs <= 3600 {
                    Duration::hours(1)
                } else {
                    Duration::hours(4)
                }
            }
        }
    }

    /// Gets cache statistics
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            data_entries: self.data_cache.len(),
            metadata_entries: self.metadata_cache.len(),
            total_memory: self.data_cache.iter().map(|e| e.size_bytes).sum::<u64>()
                + self.metadata_cache.iter().map(|e| e.size_bytes).sum::<u64>(),
            max_memory: self.max_memory_bytes,
            hits: self.metrics.get_cache_hits(),
            misses: self.metrics.get_cache_misses(),
            evictions: self.metrics.get_cache_evictions(),
        }
    }

    /// Clears all caches
    pub fn clear(&self) {
        self.data_cache.clear();
        self.metadata_cache.clear();
        self.access_history.write().clear();
        debug!("Cache cleared");
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub data_entries: usize,
    pub metadata_entries: usize,
    pub total_memory: u64,
    pub max_memory: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    async fn create_test_cache() -> CacheManager {
        let metrics = Arc::new(ApiMetrics::new());
        CacheManager::new(100, 64, metrics)
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = create_test_cache().await;
        let key = CacheKey::new(
            "TEST:PV1".to_string(),
            0,
            100,
            DataResolution::Raw,
        );

        // Test data fetching
        let data = cache.get_or_fetch_data(
            key.clone(),
            || async {
                Ok(vec![ProcessedPoint {
                    timestamp: 0,
                    severity: 0,
                    status: 0,
                    value: 42.0,
                    min: 42.0,
                    max: 42.0,
                    stddev: 0.0,
                    count: 1,
                }])
            },
            DataResolution::Raw,
        ).await;

        assert!(data.is_ok());
        let fetched = data.unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].value, 42.0);
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = create_test_cache().await;
        let key = CacheKey::new(
            "TEST:PV1".to_string(),
            0,
            100,
            DataResolution::Raw,
        );

        // Insert initial data
        let _ = cache.get_or_fetch_data(
            key.clone(),
            || async { Ok(vec![]) },
            DataResolution::Raw,
        ).await;

        // Wait for expiration
        tokio::time::sleep(StdDuration::from_secs(1)).await;

        // Should trigger a new fetch
        let fetch_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let fetch_count_clone = fetch_count.clone();

        let _ = cache.get_or_fetch_data(
            key.clone(),
            || async {
                fetch_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(vec![])
            },
            DataResolution::Raw,
        ).await;

        assert_eq!(fetch_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_memory_limits() {
        let cache = create_test_cache().await;
        let large_data = vec![ProcessedPoint {
            timestamp: 0,
            severity: 0,
            status: 0,
            value: 0.0,
            min: 0.0,
            max: 0.0,
            stddev: 0.0,
            count: 1,
        }; 1000];

        // Try to cache data larger than the memory limit
        let result = cache.store_with_memory_check(
            "test_key".to_string(),
            CacheEntry {
                data: large_data,
                metadata: CacheMetadata {
                    created_at: Utc::now(),
                    last_accessed: Utc::now(),
                    access_count: 1,
                    resolution: DataResolution::Raw,
                    validity_period: Duration::minutes(5),
                },
                size_bytes: 1024 * 1024 * 100, // 100MB
            },
        );

        assert!(result.is_ok()); // Should succeed but trigger evictions
        let stats = cache.get_stats();
        assert!(stats.evictions > 0);
    }
}
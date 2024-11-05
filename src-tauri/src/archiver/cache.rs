// cache.rs

use crate::archiver::{
    error::{ArchiverError, Result},
    types::*,
    metrics::ApiMetrics,
};

use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use std::collections::VecDeque;
use tracing::{debug, warn, error};

/// Cache key components
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pv: String,
    start_time: i64,
    end_time: i64,
    resolution: String,
}

impl CacheKey {
    pub fn new(pv: String, start_time: i64, end_time: i64, resolution: String) -> Self {
        Self {
            pv,
            start_time,
            end_time,
            resolution,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}:{}:{}", self.pv, self.start_time, self.end_time, self.resolution)
    }
}

#[derive(Debug)]
pub struct CacheManager {
    data_cache: DashMap<String, CacheEntry<Vec<ProcessedPoint>>>,
    metadata_cache: DashMap<String, CacheEntry<Meta>>,
    access_history: RwLock<VecDeque<(String, DateTime<Utc>)>>,
    max_memory_bytes: u64,
    metrics: Arc<ApiMetrics>,
}

impl CacheManager {
    pub fn new(max_entries: usize, max_memory_mb: u64, metrics: Arc<ApiMetrics>) -> Self {
        Self {
            data_cache: DashMap::new(),
            metadata_cache: DashMap::new(),
            access_history: RwLock::new(VecDeque::with_capacity(max_entries)),
            max_memory_bytes: max_memory_mb * 1024 * 1024,
            metrics,
        }
    }

    pub async fn get_or_fetch_data<F, Fut>(
        &self,
        key: CacheKey,
        fetch_fn: F,
        resolution: String,
    ) -> Result<Vec<ProcessedPoint>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Vec<ProcessedPoint>>>,
    {
        let cache_key = key.to_string();

        // Try to get from cache
        if let Some(mut entry) = self.data_cache.get_mut(&cache_key) {
            if !self.is_entry_expired(&entry) {
                self.update_access_stats(&cache_key, &mut entry);
                self.metrics.record_cache_hit();
                return Ok(entry.data.clone());
            }
            self.data_cache.remove(&cache_key);
        }

        // Fetch new data
        self.metrics.record_cache_miss();
        let data = fetch_fn().await?;

        // Cache the result
        let validity_period = match resolution.as_str() {
            "raw" => Duration::minutes(5),
            _ => Duration::minutes(15),
        };

        let entry = CacheEntry {
            data: data.clone(),
            timestamp: Utc::now(),
            expires: Utc::now() + validity_period,
            size_bytes: self.calculate_size(&data),
        };

        self.store_with_memory_check(cache_key, entry)?;
        Ok(data)
    }

    pub async fn get_metadata<F, Fut>(
        &self,
        pv: &str,
        fetch_fn: F,
    ) -> Result<Meta>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Meta>>,
    {
        if let Some(mut entry) = self.metadata_cache.get_mut(pv) {
            if !self.is_entry_expired(&entry) {
                self.update_access_stats(pv, &mut entry);
                self.metrics.record_cache_hit();
                return Ok(entry.data.clone());
            }
            self.metadata_cache.remove(pv);
        }

        self.metrics.record_cache_miss();
        let metadata = fetch_fn().await?;
        
        let entry = CacheEntry {
            data: metadata.clone(),
            timestamp: Utc::now(),
            expires: Utc::now() + Duration::hours(1),
            size_bytes: std::mem::size_of::<Meta>() as u64,
        };

        self.metadata_cache.insert(pv.to_string(), entry);
        Ok(metadata)
    }

    fn is_entry_expired<T>(&self, entry: &CacheEntry<T>) -> bool {
        Utc::now() >= entry.expires
    }

    fn update_access_stats<T>(&self, key: &str, entry: &mut CacheEntry<T>) {
        entry.timestamp = Utc::now();
        
        let mut history = self.access_history.write();
        history.push_back((key.to_string(), Utc::now()));

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
        let mut current_size: u64 = self.data_cache.iter().map(|e| e.size_bytes).sum();

        if current_size + new_size > self.max_memory_bytes {
            let mut to_remove = Vec::new();
            let needed_space = new_size;
            
            let history = self.access_history.read();
            for (key, _) in history.iter().rev() {
                if let Some(entry) = self.data_cache.get(key) {
                    to_remove.push(key.clone());
                    current_size = current_size.saturating_sub(entry.size_bytes);
                    if current_size + new_size <= self.max_memory_bytes {
                        break;
                    }
                }
            }

            for key in to_remove {
                if let Some(_) = self.data_cache.remove(&key) {
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

    pub fn clear(&self) {
        self.data_cache.clear();
        self.metadata_cache.clear();
        self.access_history.write().clear();
        debug!("Cache cleared");
    }
}

#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    data: T,
    timestamp: DateTime<Utc>,
    expires: DateTime<Utc>,
    size_bytes: u64,
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
            "raw".to_string(),
        );

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
            "raw".to_string(),
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
            "raw".to_string(),
        );

        let _ = cache.get_or_fetch_data(
            key.clone(),
            || async { Ok(vec![]) },
            "raw".to_string(),
        ).await;

        tokio::time::sleep(StdDuration::from_secs(1)).await;

        let fetch_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let fetch_count_clone = fetch_count.clone();

        let _ = cache.get_or_fetch_data(
            key.clone(),
            || async {
                fetch_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(vec![])
            },
            "raw".to_string(),
        ).await;

        assert_eq!(fetch_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
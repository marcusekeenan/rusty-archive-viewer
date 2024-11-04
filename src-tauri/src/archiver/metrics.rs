// metrics.rs

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use std::collections::VecDeque;
use metrics::{counter, gauge, histogram};
use tracing::debug;

#[derive(Debug)]
pub struct ApiMetrics {
    // Request metrics
    requests_total: AtomicU64,
    failed_requests: AtomicU64,
    request_latencies: Arc<RwLock<VecDeque<f64>>>,
    
    // Cache metrics
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    cache_evictions: AtomicU64,
    cache_memory_usage: AtomicU64,
    
    // Connection metrics
    active_connections: AtomicUsize,
    
    // Performance tracking
    start_time: Instant,
    last_error: Arc<RwLock<Option<String>>>,
    last_error_time: Arc<RwLock<Option<Instant>>>,
    
    // Memory tracking
    memory_usage: AtomicU64,
    peak_memory_usage: AtomicU64,
    gc_count: AtomicU64,
    
    // Statistical windows
    latency_window: Arc<RwLock<TimeWindow<f64>>>,
    error_window: Arc<RwLock<TimeWindow<bool>>>,
}

#[derive(Debug)]
struct TimeWindow<T> {
    data: VecDeque<(Instant, T)>,
    window_size: Duration,
}

impl<T> TimeWindow<T> {
    fn new(window_size: Duration) -> Self {
        Self {
            data: VecDeque::new(),
            window_size,
        }
    }

    fn add(&mut self, value: T) {
        let now = Instant::now();
        self.data.push_back((now, value));
        self.cleanup();
    }

    fn cleanup(&mut self) {
        let cutoff = Instant::now() - self.window_size;
        while self.data.front().map_or(false, |(t, _)| *t < cutoff) {
            self.data.pop_front();
        }
    }
}

// metrics.rs (continued)

impl ApiMetrics {
    // Cache metrics continued...
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        counter!("api.cache.misses").increment(1);
    }

    pub fn record_cache_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
        counter!("api.cache.evictions").increment(1);
    }

    pub fn update_cache_memory(&self, bytes: u64) {
        self.cache_memory_usage.store(bytes, Ordering::Relaxed);
        gauge!("api.cache.memory_usage_bytes").set(bytes as f64);
    }

    // Connection tracking
    pub fn record_connection_opened(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        gauge!("api.connections.active").set(self.active_connections.load(Ordering::Relaxed) as f64);
    }

    pub fn record_connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
        gauge!("api.connections.active").set(self.active_connections.load(Ordering::Relaxed) as f64);
    }

    // Memory tracking
    pub fn update_memory_usage(&self, bytes: u64) {
        self.memory_usage.store(bytes, Ordering::Relaxed);
        let peak = self.peak_memory_usage.load(Ordering::Relaxed);
        if bytes > peak {
            self.peak_memory_usage.store(bytes, Ordering::Relaxed);
        }
        gauge!("api.memory.current_bytes").set(bytes as f64);
        gauge!("api.memory.peak_bytes").set(peak as f64);
    }

    pub fn record_gc(&self) {
        self.gc_count.fetch_add(1, Ordering::Relaxed);
        counter!("api.memory.gc_count").increment(1);
    }

    // Metric retrieval
    pub fn get_current_metrics(&self) -> MetricsSnapshot {
        let latency_window = self.latency_window.read();
        let error_window = self.error_window.read();
        
        let total_requests = self.requests_total.load(Ordering::Relaxed);
        let failed_requests = self.failed_requests.load(Ordering::Relaxed);
        
        let latencies: Vec<f64> = latency_window.data.iter()
            .map(|(_, v)| *v)
            .collect();
        
        let error_rate = if !error_window.data.is_empty() {
            error_window.data.iter()
                .filter(|(_, is_error)| *is_error)
                .count() as f64 / error_window.data.len() as f64
        } else {
            0.0
        };

        MetricsSnapshot {
            timestamp: std::time::SystemTime::now(),
            uptime: self.start_time.elapsed(),
            total_requests,
            failed_requests,
            error_rate,
            requests_per_second: self.calculate_request_rate(),
            average_latency: self.calculate_average_latency(&latencies),
            p95_response_time: self.calculate_percentile(&latencies, 0.95),
            p99_response_time: self.calculate_percentile(&latencies, 0.99),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
            peak_memory_usage: self.peak_memory_usage.load(Ordering::Relaxed),
            cache_memory_usage: self.cache_memory_usage.load(Ordering::Relaxed),
            cache_hit_rate: self.calculate_cache_hit_rate(),
            cache_entries: 0, // This should be provided by the cache manager
            cache_evictions: self.cache_evictions.load(Ordering::Relaxed),
            gc_count: self.gc_count.load(Ordering::Relaxed),
            last_error: self.last_error.read().clone(),
            last_error_time: self.last_error_time.read().map(|t| t.elapsed()),
        }
    }

    // Calculation helpers
    fn calculate_request_rate(&self) -> f64 {
        let window = self.latency_window.read();
        if window.data.is_empty() {
            return 0.0;
        }

        let window_duration = window.data.back()
            .map(|(t, _)| t.duration_since(window.data.front().unwrap().0))
            .unwrap_or_default();
        
        window.data.len() as f64 / window_duration.as_secs_f64()
    }

    fn calculate_average_latency(&self, latencies: &[f64]) -> f64 {
        if latencies.is_empty() {
            return 0.0;
        }
        latencies.iter().sum::<f64>() / latencies.len() as f64
    }

    fn calculate_percentile(&self, values: &[f64], percentile: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let index = (sorted.len() as f64 * percentile).floor() as usize;
        sorted[index.min(sorted.len() - 1)]
    }

    fn calculate_cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        
        if total > 0.0 {
            hits / total
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: std::time::SystemTime,
    pub uptime: Duration,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub error_rate: f64,
    pub requests_per_second: f64,
    pub average_latency: f64,
    pub p95_response_time: f64,
    pub p99_response_time: f64,
    pub active_connections: usize,
    pub memory_usage: u64,
    pub peak_memory_usage: u64,
    pub cache_memory_usage: u64,
    pub cache_hit_rate: f64,
    pub cache_entries: usize,
    pub cache_evictions: u64,
    pub gc_count: u64,
    pub last_error: Option<String>,
    pub last_error_time: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::test;

    #[test]
    async fn test_basic_metrics() {
        let metrics = ApiMetrics::new();

        // Record some test data
        metrics.record_request();
        metrics.record_request();
        metrics.record_error(Some("Test error".to_string()));
        metrics.record_latency(Duration::from_millis(100));
        
        let snapshot = metrics.get_current_metrics();
        
        assert_eq!(snapshot.total_requests, 2);
        assert_eq!(snapshot.failed_requests, 1);
        assert!(snapshot.error_rate > 0.0);
        assert!(snapshot.average_latency > 0.0);
    }

    #[test]
    async fn test_cache_metrics() {
        let metrics = ApiMetrics::new();

        // Record cache operations
        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        
        let snapshot = metrics.get_current_metrics();
        
        assert_eq!(snapshot.cache_hit_rate, 2.0/3.0);
        
        // Test memory tracking
        metrics.update_cache_memory(1024 * 1024); // 1MB
        assert_eq!(metrics.get_current_metrics().cache_memory_usage, 1024 * 1024);
    }

    #[test]
    async fn test_percentile_calculation() {
        let metrics = ApiMetrics::new();
        
        // Record various latencies
        for ms in &[10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
            metrics.record_latency(Duration::from_millis(*ms as u64));
        }
        
        let snapshot = metrics.get_current_metrics();
        
        assert!(snapshot.p95_response_time > snapshot.average_latency);
        assert!(snapshot.p99_response_time > snapshot.p95_response_time);
    }

    #[test]
    async fn test_memory_tracking() {
        let metrics = ApiMetrics::new();

        metrics.update_memory_usage(1000);
        metrics.update_memory_usage(2000);
        metrics.update_memory_usage(1500);

        let snapshot = metrics.get_current_metrics();
        assert_eq!(snapshot.memory_usage, 1500);
        assert_eq!(snapshot.peak_memory_usage, 2000);
    }
}




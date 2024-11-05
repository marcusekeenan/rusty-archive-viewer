// metrics.rs

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;
use std::collections::VecDeque;
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

impl ApiMetrics {
    pub fn new() -> Self {
        let window_size = Duration::from_secs(300); // 5 minute window
        Self {
            requests_total: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            request_latencies: Arc::new(RwLock::new(VecDeque::new())),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_evictions: AtomicU64::new(0),
            cache_memory_usage: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            start_time: Instant::now(),
            last_error: Arc::new(RwLock::new(None)),
            last_error_time: Arc::new(RwLock::new(None)),
            memory_usage: AtomicU64::new(0),
            peak_memory_usage: AtomicU64::new(0),
            gc_count: AtomicU64::new(0),
            latency_window: Arc::new(RwLock::new(TimeWindow::new(window_size))),
            error_window: Arc::new(RwLock::new(TimeWindow::new(window_size))),
        }
    }

    pub fn reset_errors(&self) {
        self.failed_requests.store(0, Ordering::Relaxed);
        *self.last_error.write() = None;
        *self.last_error_time.write() = None;
    }

    pub fn update_memory_usage(&self, usage: u64) {
        self.memory_usage.store(usage, Ordering::Relaxed);
        let peak = self.peak_memory_usage.load(Ordering::Relaxed);
        if usage > peak {
            self.peak_memory_usage.store(usage, Ordering::Relaxed);
        }
    }

    pub fn record_request(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self, error_msg: Option<String>) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        if let Some(msg) = error_msg {
            *self.last_error.write() = Some(msg);
            *self.last_error_time.write() = Some(Instant::now());
        }
        self.error_window.write().add(true);
    }

    pub fn record_latency(&self, duration: Duration) {
        let latency = duration.as_secs_f64() * 1000.0; // Convert to milliseconds
        self.request_latencies.write().push_back(latency);
        self.latency_window.write().add(latency);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_cache_hits(&self) -> u64 {
        self.cache_hits.load(Ordering::Relaxed)
    }

    pub fn get_cache_misses(&self) -> u64 {
        self.cache_misses.load(Ordering::Relaxed)
    }

    pub fn get_cache_evictions(&self) -> u64 {
        self.cache_evictions.load(Ordering::Relaxed)
    }

    pub fn record_cache_eviction(&self) {
        self.cache_evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_session_created(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_session_expired(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn get_expired_session_count(&self) -> u64 {
        self.gc_count.load(Ordering::Relaxed)
    }

    fn calculate_request_rate(&self) -> f64 {
        let window = self.latency_window.read();
        if window.data.is_empty() {
            return 0.0;
        }

        let window_duration = window.data.back()
            .and_then(|(t, _)| window.data.front().map(|(front_t, _)| t.duration_since(*front_t)))
            .unwrap_or_default();
        
        if window_duration.as_secs_f64() > 0.0 {
            window.data.len() as f64 / window_duration.as_secs_f64()
        } else {
            0.0
        }
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
        
        let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
        sorted[index]
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

    // Update get_current_metrics to use these functions
    pub fn get_current_metrics(&self) -> MetricsSnapshot {
        let latencies: Vec<f64> = self.request_latencies.read()
            .iter()
            .copied()
            .collect();

        MetricsSnapshot {
            timestamp: std::time::SystemTime::now(),
            uptime: self.start_time.elapsed(),
            total_requests: self.requests_total.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            error_rate: if self.requests_total.load(Ordering::Relaxed) > 0 {
                self.failed_requests.load(Ordering::Relaxed) as f64 
                    / self.requests_total.load(Ordering::Relaxed) as f64
            } else {
                0.0
            },
            requests_per_second: self.calculate_request_rate(),
            average_latency: self.calculate_average_latency(&latencies),
            p95_response_time: self.calculate_percentile(&latencies, 0.95),
            p99_response_time: self.calculate_percentile(&latencies, 0.99),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            memory_usage: self.memory_usage.load(Ordering::Relaxed),
            peak_memory_usage: self.peak_memory_usage.load(Ordering::Relaxed),
            cache_memory_usage: self.cache_memory_usage.load(Ordering::Relaxed),
            cache_hit_rate: self.calculate_cache_hit_rate(),
            cache_entries: self.request_latencies.read().len(),
            cache_evictions: self.cache_evictions.load(Ordering::Relaxed),
            gc_count: self.gc_count.load(Ordering::Relaxed),
            last_error: self.last_error.read().clone(),
            last_error_time: self.last_error_time.read().map(|t| t.elapsed()),
            average_response_time: self.calculate_average_latency(&latencies),  // Added this line
        }
    }
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

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: std::time::SystemTime,
    pub uptime: Duration,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub error_rate: f64,
    pub requests_per_second: f64,
    pub average_latency: f64,
    pub average_response_time: f64,  // Added this field
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
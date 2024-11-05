use crate::archiver::{
    error::{ArchiverError, ErrorContext, Result},
    metrics::ApiMetrics,
};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};
use tracing::{debug, error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemStatus {
    Healthy,
    Degraded {
        reason: String,
        since: DateTime<Utc>,
    },
    Unhealthy {
        reason: String,
        since: DateTime<Utc>,
        error_count: u64,
    },
    Initializing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: SystemStatus,
    #[serde(with = "duration_serde")]
    pub uptime: Duration,
    pub last_check: DateTime<Utc>,
    pub memory_metrics: MemoryMetrics,
    pub connection_metrics: ConnectionMetrics,
    pub cache_metrics: CacheMetrics,
    pub performance_metrics: PerformanceMetrics,
}

mod duration_serde {
    use super::*;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(duration.num_seconds())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<Duration, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let seconds = i64::deserialize(deserializer)
            .map_err(|e| D::Error::custom(format!("Failed to deserialize duration: {}", e)))?;
        Ok(Duration::seconds(seconds))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_allocated: u64,
    pub cache_usage: u64,
    pub peak_usage: u64,
    pub gc_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetrics {
    pub active_connections: usize,
    pub failed_requests: u64,
    pub average_latency: f64,
    pub error_rate: f64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hit_rate: f64,
    pub memory_usage: u64,
    pub entry_count: usize,
    pub eviction_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub requests_per_second: f64,
    pub average_response_time: f64,
    pub p95_response_time: f64,
    pub p99_response_time: f64,
}

#[derive(Debug)]
pub struct HealthMonitor {
    start_time: DateTime<Utc>,
    metrics: Arc<ApiMetrics>,
    status_history: Arc<RwLock<VecDeque<HealthStatus>>>,
    check_interval: Duration,
    max_history: usize,
    thresholds: HealthThresholds,
    shutdown_tx: Arc<RwLock<Option<watch::Sender<bool>>>>,
    is_running: Arc<RwLock<bool>>,
}

#[derive(Debug, Clone)]
pub struct HealthThresholds {
    max_error_rate: f64,
    max_response_time: f64,
    max_memory_usage: u64,
    degraded_cache_hit_rate: f64,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            max_error_rate: 0.05,                 // 5%
            max_response_time: 1000.0,            // 1 second
            max_memory_usage: 1024 * 1024 * 1024, // 1GB
            degraded_cache_hit_rate: 0.5,         // 50%
        }
    }
}

impl HealthMonitor {
    pub fn new(check_interval: Duration, max_history: usize) -> Self {
        Self {
            start_time: Utc::now(),
            metrics: Arc::new(ApiMetrics::new()),
            status_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            check_interval,
            max_history,
            thresholds: HealthThresholds::default(),
            shutdown_tx: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_thresholds(
        check_interval: Duration,
        max_history: usize,
        thresholds: HealthThresholds,
    ) -> Self {
        Self {
            start_time: Utc::now(),
            metrics: Arc::new(ApiMetrics::new()),
            status_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            check_interval,
            max_history,
            thresholds,
            shutdown_tx: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        // Check if already running
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }

        // Create shutdown channel
        let (tx, mut rx) = watch::channel(false);
        *self.shutdown_tx.write().await = Some(tx);
        *is_running = true;
        drop(is_running);

        // Create initial status with Initializing state
        let initial_status = HealthStatus {
            status: SystemStatus::Initializing,
            uptime: Duration::zero(),
            last_check: Utc::now(),
            memory_metrics: MemoryMetrics {
                total_allocated: 0,
                cache_usage: 0,
                peak_usage: 0,
                gc_count: 0,
            },
            connection_metrics: ConnectionMetrics {
                active_connections: 0,
                failed_requests: 0,
                average_latency: 0.0,
                error_rate: 0.0,
                last_error: None,
            },
            cache_metrics: CacheMetrics {
                hit_rate: 0.0,
                memory_usage: 0,
                entry_count: 0,
                eviction_count: 0,
            },
            performance_metrics: PerformanceMetrics {
                requests_per_second: 0.0,
                average_response_time: 0.0,
                p95_response_time: 0.0,
                p99_response_time: 0.0,
            },
        };

        // Set initial status
        self.status_history.write().await.push_back(initial_status);

        let monitor = self.clone();

        #[cfg(not(test))]
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                monitor.check_interval.num_seconds() as u64,
            ));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = monitor.perform_health_check().await {
                            error!("Health check failed: {}", e);
                        }
                    }
                    Ok(_) = rx.changed() => {
                        debug!("Health monitor shutdown signal received");
                        break;
                    }
                }
            }

            *monitor.is_running.write().await = false;
            debug!("Health monitor stopped");
        });

        // Perform initial health check
        self.perform_health_check()
            .await
            .map_err(|e| e.with_context("HealthMonitor", "perform_initial_health_check"))?;

        Ok(())
    }
    pub async fn stop(&self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.read().await.as_ref() {
            tx.send(true).map_err(|e| ArchiverError::HealthCheckError {
                message: "Failed to send shutdown signal".into(),
                context: "Shutdown channel".into(),
                source: Some(Box::new(e)),
                error_context: Some(ErrorContext::new("HealthMonitor", "stop")),
            })?;
        }
        Ok(())
    }

    async fn perform_health_check(&self) -> Result<()> {
        let status = self
            .collect_health_status()
            .await
            .map_err(|e| e.with_context("HealthMonitor", "collect_health_status"))?;

        // Update history
        let mut history = self.status_history.write().await;

        // Log status changes if there's previous status
        if let Some(prev_status) = history.back() {
            match (&prev_status.status, &status.status) {
                (SystemStatus::Healthy, SystemStatus::Degraded { reason, .. }) => {
                    warn!("System health degraded: {}", reason);
                }
                (_, SystemStatus::Unhealthy { reason, .. }) => {
                    error!("System unhealthy: {}", reason);
                }
                (
                    SystemStatus::Degraded { .. } | SystemStatus::Unhealthy { .. },
                    SystemStatus::Healthy,
                ) => {
                    debug!("System recovered to healthy state");
                }
                _ => {}
            }
        }

        history.push_back(status);

        // Maintain history size
        while history.len() > self.max_history {
            history.pop_front();
        }

        Ok(())
    }
    async fn collect_health_status(&self) -> Result<HealthStatus> {
        let error_context = ErrorContext::new("HealthMonitor", "collect_health_status");
        let metrics = self.metrics.get_current_metrics();

        let memory_metrics = MemoryMetrics {
            total_allocated: metrics.memory_usage,
            cache_usage: metrics.cache_memory_usage,
            peak_usage: metrics.peak_memory_usage,
            gc_count: metrics.gc_count,
        };

        let connection_metrics = ConnectionMetrics {
            active_connections: metrics.active_connections,
            failed_requests: metrics.failed_requests,
            average_latency: metrics.average_latency,
            error_rate: metrics.error_rate,
            last_error: metrics.last_error.clone(),
        };

        let cache_metrics = CacheMetrics {
            hit_rate: metrics.cache_hit_rate,
            memory_usage: metrics.cache_memory_usage,
            entry_count: metrics.cache_entries,
            eviction_count: metrics.cache_evictions,
        };

        let performance_metrics = PerformanceMetrics {
            requests_per_second: metrics.requests_per_second,
            average_response_time: metrics.average_response_time,
            p95_response_time: metrics.p95_response_time,
            p99_response_time: metrics.p99_response_time,
        };

        let status = if connection_metrics.error_rate > self.thresholds.max_error_rate
            || performance_metrics.average_response_time > self.thresholds.max_response_time
        {
            SystemStatus::Unhealthy {
                reason: "High error rate or slow response time".to_string(),
                since: Utc::now(),
                error_count: metrics.failed_requests,
            }
        } else if memory_metrics.total_allocated > self.thresholds.max_memory_usage
            || cache_metrics.hit_rate < self.thresholds.degraded_cache_hit_rate
        {
            SystemStatus::Degraded {
                reason: "High memory usage or low cache hit rate".to_string(),
                since: Utc::now(),
            }
        } else {
            SystemStatus::Healthy
        };

        Ok(HealthStatus {
            status,
            uptime: Utc::now()
                .signed_duration_since(self.start_time)
                .max(Duration::zero()),
            last_check: Utc::now(),
            memory_metrics,
            connection_metrics,
            cache_metrics,
            performance_metrics,
        })
    }

    pub async fn get_current_status(&self) -> Result<HealthStatus> {
        let history = self.status_history.read().await;
        history
            .back()
            .cloned()
            .ok_or_else(|| ArchiverError::HealthCheckError {
                message: "No health status available".into(),
                context: "Health check not yet performed".into(),
                source: None,
                error_context: Some(ErrorContext::new("HealthMonitor", "get_current_status")),
            })
    }

    pub async fn get_status_history(&self) -> Result<Vec<HealthStatus>> {
        let history = self.status_history.read().await;
        Ok(history.iter().cloned().collect())
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_health_monitor_lifecycle() {
        let monitor = Arc::new(HealthMonitor::new(Duration::seconds(1), 10));

        // Start the monitor
        assert!(!monitor.is_running().await);
        monitor.clone().start().await.unwrap();
        assert!(monitor.is_running().await);

        // Wait for initial health check
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Verify status
        let status = monitor.get_current_status().await.unwrap();
        assert!(matches!(status.status, SystemStatus::Healthy));

        // Stop the monitor
        monitor.stop().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(!monitor.is_running().await);
    }

    #[test]
    async fn test_health_monitor_thresholds() {
        let thresholds = HealthThresholds {
            max_error_rate: 0.01,          // 1%
            max_response_time: 100.0,      // 100ms
            max_memory_usage: 1024 * 1024, // 1MB
            degraded_cache_hit_rate: 0.8,  // 80%
        };

        let monitor = Arc::new(HealthMonitor::with_thresholds(
            Duration::seconds(1),
            10,
            thresholds,
        ));

        monitor.clone().start().await.unwrap();

        // Simulate high error rate
        monitor
            .metrics
            .record_error(Some("Test error 1".to_string()));
        monitor
            .metrics
            .record_error(Some("Test error 2".to_string()));
        monitor.metrics.record_request(); // 66% error rate

        let status = monitor.collect_health_status().await.unwrap();
        assert!(matches!(status.status, SystemStatus::Unhealthy { .. }));

        // Test recovery
        monitor.metrics.reset_errors();
        let status = monitor.collect_health_status().await.unwrap();
        assert!(matches!(status.status, SystemStatus::Healthy));

        // Cleanup
        monitor.stop().await.unwrap();
    }

    #[test]
    async fn test_health_monitor_history() {
        let monitor = Arc::new(HealthMonitor::new(Duration::seconds(1), 3));
        monitor.clone().start().await.unwrap();

        // Generate multiple status updates
        for i in 0..5 {
            if i % 2 == 0 {
                monitor
                    .metrics
                    .record_error(Some(format!("Test error {}", i)));
            }
            monitor.metrics.record_request();
            monitor.perform_health_check().await.unwrap();
        }

        // Check history size is limited
        let history = monitor.get_status_history().await.unwrap();
        assert_eq!(history.len(), 3);

        // Cleanup
        monitor.stop().await.unwrap();
    }

    #[test]
    async fn test_health_monitor_degraded_state() {
        let monitor = Arc::new(HealthMonitor::new(Duration::seconds(1), 10));
        monitor.clone().start().await.unwrap();

        // Simulate high memory usage
        monitor.metrics.update_memory_usage(2 * 1024 * 1024 * 1024); // 2GB

        let status = monitor.collect_health_status().await.unwrap();
        assert!(matches!(status.status, SystemStatus::Degraded { .. }));

        // Cleanup
        monitor.stop().await.unwrap();
    }

    #[test]
    async fn test_health_monitor_initialization() {
        let monitor = Arc::new(HealthMonitor::new(Duration::seconds(1), 10));

        // Check initial state before start
        let status = monitor.get_current_status().await;
        assert!(status.is_err());

        // Start monitor
        monitor.clone().start().await.unwrap();

        // Check initial status is set
        let status = monitor.get_current_status().await.unwrap();
        assert!(matches!(status.status, SystemStatus::Initializing));

        // Wait for first health check
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let status = monitor.get_current_status().await.unwrap();
        assert!(!matches!(status.status, SystemStatus::Initializing));

        // Cleanup
        monitor.stop().await.unwrap();
    }

    #[test]
    async fn test_health_monitor_concurrent_access() {
        let monitor = Arc::new(HealthMonitor::new(Duration::seconds(1), 10));
        monitor.clone().start().await.unwrap();

        // Spawn multiple tasks to access health monitor
        let mut handles = vec![];
        for _ in 0..5 {
            let monitor_clone = monitor.clone();
            handles.push(tokio::spawn(async move {
                let status = monitor_clone.get_current_status().await.unwrap();
                assert!(matches!(
                    status.status,
                    SystemStatus::Healthy | SystemStatus::Initializing
                ));
            }));
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Cleanup
        monitor.stop().await.unwrap();
    }

    #[test]
    async fn test_health_monitor_metrics_accuracy() {
        let monitor = Arc::new(HealthMonitor::new(Duration::seconds(1), 10));
        monitor.clone().start().await.unwrap();

        // Record some metrics
        monitor.metrics.record_request();
        monitor
            .metrics
            .record_latency(std::time::Duration::from_millis(100));
        monitor.metrics.record_cache_hit();
        monitor.metrics.record_cache_miss();

        let status = monitor.collect_health_status().await.unwrap();

        assert_eq!(status.connection_metrics.active_connections, 0);
        assert_eq!(status.connection_metrics.failed_requests, 0);
        assert!(status.connection_metrics.average_latency > 0.0);
        assert_eq!(status.cache_metrics.hit_rate, 0.5); // 1 hit, 1 miss = 50%

        // Cleanup
        monitor.stop().await.unwrap();
    }

    #[test]
    async fn test_health_monitor_uptime() {
        let monitor = Arc::new(HealthMonitor::new(Duration::seconds(1), 10));
        monitor.clone().start().await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let status = monitor.get_current_status().await.unwrap();
        assert!(status.uptime >= Duration::seconds(2));

        // Cleanup
        monitor.stop().await.unwrap();
    }
}

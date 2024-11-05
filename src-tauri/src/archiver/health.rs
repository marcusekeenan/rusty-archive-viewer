// health.rs

use crate::archiver::{
    error::{ArchiverError, Result},
    metrics::ApiMetrics,
};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::VecDeque;
use tracing::{debug, warn, error};

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
fn deserialize_duration<'de, D>(deserializer: D) -> std::result::Result<Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let seconds = i64::deserialize(deserializer)
        .map_err(|e| D::Error::custom(format!("Failed to deserialize duration: {}", e)))?;
    Ok(Duration::seconds(seconds))
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
            max_error_rate: 0.05, // 5%
            max_response_time: 1000.0, // 1 second
            max_memory_usage: 1024 * 1024 * 1024, // 1GB
            degraded_cache_hit_rate: 0.5, // 50%
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
        }
    }

    pub fn with_thresholds(check_interval: Duration, max_history: usize, thresholds: HealthThresholds) -> Self {
        Self {
            start_time: Utc::now(),
            metrics: Arc::new(ApiMetrics::new()),
            status_history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            check_interval,
            max_history,
            thresholds,
        }
    }

    pub fn start(self: Arc<Self>) {
        let monitor = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(monitor.check_interval.num_seconds() as u64)
            );

            loop {
                interval.tick().await;
                if let Err(e) = monitor.perform_health_check().await {
                    error!("Health check failed: {}", e);
                }
            }
        });
    }

    async fn perform_health_check(&self) -> Result<()> {
        let status = self.collect_health_status().await?;
        
        // Update history
        let mut history = self.status_history.write().await;
        history.push_back(status.clone());
        
        // Maintain history size
        while history.len() > self.max_history {
            history.pop_front();
        }

        // Log status changes
        if let Some(prev_status) = history.get(history.len() - 2) {
            match (&prev_status.status, &status.status) {
                (SystemStatus::Healthy, SystemStatus::Degraded { reason, .. }) => {
                    warn!("System health degraded: {}", reason);
                }
                (_, SystemStatus::Unhealthy { reason, .. }) => {
                    error!("System unhealthy: {}", reason);
                }
                (SystemStatus::Degraded { .. }, SystemStatus::Healthy) |
                (SystemStatus::Unhealthy { .. }, SystemStatus::Healthy) => {
                    debug!("System recovered to healthy state");
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn collect_health_status(&self) -> Result<HealthStatus> {
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

        let status = if connection_metrics.error_rate > self.thresholds.max_error_rate ||
           performance_metrics.average_response_time > self.thresholds.max_response_time {
            SystemStatus::Unhealthy {
                reason: "High error rate or slow response time".to_string(),
                since: Utc::now(),
                error_count: metrics.failed_requests,
            }
        } else if memory_metrics.total_allocated > self.thresholds.max_memory_usage ||
                  cache_metrics.hit_rate < self.thresholds.degraded_cache_hit_rate {
            SystemStatus::Degraded {
                reason: "High memory usage or low cache hit rate".to_string(),
                since: Utc::now(),
            }
        } else {
            SystemStatus::Healthy
        };

        Ok(HealthStatus {
            status,
            uptime: Utc::now() - self.start_time,
            last_check: Utc::now(),
            memory_metrics,
            connection_metrics,
            cache_metrics,
            performance_metrics,
        })
    }

    pub async fn get_current_status(&self) -> Result<HealthStatus> {
        let history = self.status_history.read().await;
        history.back().cloned().ok_or_else(|| ArchiverError::HealthCheckError {
            message: "No health status available".to_string(),
            context: "Health check not yet performed".to_string(),
            source: None,
        })
    }

    pub async fn get_status_history(&self) -> Result<Vec<HealthStatus>> {
        let history = self.status_history.read().await;
        Ok(history.iter().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_health_monitor_basic() {
        let monitor = Arc::new(HealthMonitor::new(
            Duration::seconds(1),
            10
        ));
        
        monitor.clone().start();

        // Wait for first check
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let status = monitor.get_current_status().await.unwrap();
        assert!(matches!(status.status, SystemStatus::Healthy));
    }

    #[test]
async fn test_health_monitor_thresholds() {
    let thresholds = HealthThresholds {
        max_error_rate: 0.01, // 1%
        max_response_time: 100.0, // 100ms
        max_memory_usage: 1024 * 1024, // 1MB
        degraded_cache_hit_rate: 0.8, // 80%
    };

    let monitor = Arc::new(HealthMonitor::with_thresholds(
        Duration::seconds(1),
        10,
        thresholds
    ));

    // Simulate high error rate
    monitor.metrics.record_error(Some("Test error 1".to_string()));
    monitor.metrics.record_error(Some("Test error 2".to_string()));
    monitor.metrics.record_request(); // 66% error rate

    let status = monitor.collect_health_status().await.unwrap();
    assert!(matches!(status.status, SystemStatus::Unhealthy { .. }));
}
}
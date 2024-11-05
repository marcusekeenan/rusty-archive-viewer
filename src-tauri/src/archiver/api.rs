// api.rs

use crate::archiver::{
    cache::{CacheManager, CacheKey},
    error::{ArchiverError, Result},
    health::{HealthMonitor, HealthStatus},
    metrics::ApiMetrics,
    session::{SessionManager, Session},
    types::*,
    validation::{RequestValidator, Validator},
    constants::API_CONFIG,
};

use chrono::{DateTime, Duration, Utc};
// Update futures imports
use futures::{
    future::{join_all, BoxFuture},
    FutureExt,
};
use reqwest::Client;
use serde_json::Value;
use std::{sync::Arc, collections::HashMap, time::{Duration as StdDuration, Instant}};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use url::Url;
use uuid::Uuid;
/// Main API client for interacting with the EPICS Archiver Appliance
pub struct ArchiveViewerApi {
    client: Client,
    base_url: Url,
    cache: Arc<CacheManager>,
    sessions: Arc<SessionManager>,
    health: Arc<HealthMonitor>,
    metrics: Arc<ApiMetrics>,
    validator: Arc<RequestValidator>,
    config: Arc<RwLock<Config>>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub connection: ConnectionConfig,
    pub cache: CacheConfig,
    pub session: SessionConfig,
    pub metrics: MetricsConfig,
}

#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    pub timeout: StdDuration,
    pub pool_size: usize,
}

#[derive(Clone, Debug)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub max_memory_mb: u64,
}

#[derive(Clone, Debug)]
pub struct SessionConfig {
    pub max_sessions: usize,
    pub timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct MetricsConfig {
    pub collection_interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            connection: ConnectionConfig {
                timeout: StdDuration::from_secs(30),
                pool_size: 10,
            },
            cache: CacheConfig {
                max_entries: 10000,
                max_memory_mb: 1024,
            },
            session: SessionConfig {
                max_sessions: 1000,
                timeout: Duration::hours(24),
            },
            metrics: MetricsConfig {
                collection_interval: Duration::seconds(60),
            },
        }
    }
}

impl ArchiveViewerApi {
    pub async fn new(base_url: String) -> Result<Arc<Self>> {
        let config = Config::default();
        Self::with_config(config, base_url).await
    }

    pub async fn with_config(config: Config, base_url_str: String) -> Result<Arc<Self>> {
        let client = Client::builder()
            .timeout(config.connection.timeout)
            .pool_max_idle_per_host(config.connection.pool_size)
            .build()
            .map_err(|e| ArchiverError::ConnectionError {
                message: "Failed to create HTTP client".into(),
                context: e.to_string(),
                source: Some(Box::new(e)),
                retry_after: None,
            })?;

        let metrics = Arc::new(ApiMetrics::new());
        let cache = Arc::new(CacheManager::new(
            config.cache.max_entries,
            config.cache.max_memory_mb,
            metrics.clone(),
        ));
        let sessions = Arc::new(SessionManager::new(
            config.session.max_sessions,
            config.session.timeout,
        ));
        let health = Arc::new(HealthMonitor::new(
            config.metrics.collection_interval,
            100,
        ));
        let validator = Arc::new(RequestValidator::new());

        let base_url = Url::parse(&base_url_str).map_err(|e| ArchiverError::ConnectionError {
            message: "Invalid base URL".into(),
            context: e.to_string(),
            source: Some(Box::new(e)),
            retry_after: None,
        })?;

        let api = Arc::new(Self {
            client,
            base_url,
            cache,
            sessions,
            health: health.clone(),
            metrics,
            validator,
            config: Arc::new(RwLock::new(config)),
        });

        health.clone().start();

        Ok(api)
    }
    
    pub async fn fetch_data(
        &self,
        session_id: Uuid,
        pvs: Vec<String>,
        time_range: TimeRange,
        resolution: Option<String>,
    ) -> Result<HashMap<String, Vec<ProcessedPoint>>> {
        let session = self.sessions.get_session(session_id).await?;

        let start = Instant::now();
        self.metrics.record_request();

        let mut tasks: Vec<BoxFuture<'_, Result<(String, Vec<ProcessedPoint>)>>> = Vec::new();
        
        // Create Arc references to shared data
        let time_range = Arc::new(time_range);
        let resolution = Arc::new(resolution);

        for pv in pvs.into_iter() {
            let time_range = Arc::clone(&time_range);
            let resolution = Arc::clone(&resolution);
            
            let task = async move {
                let resolution_deref = resolution.as_deref();
                self.fetch_pv_data(&pv, &time_range, resolution_deref).await
            }.boxed();
            tasks.push(task);
        }

        let results = join_all(tasks).await;
        let mut data = HashMap::new();

        for result in results {
            match result {
                Ok((pv, points)) => {
                    data.insert(pv, points);
                }
                Err(e) => {
                    self.metrics.record_error(Some(e.to_string()));
                    error!("Error fetching data: {}", e);
                    return Err(e);
                }
            }
        }

        self.metrics.record_latency(start.elapsed());

        Ok(data)
    }

    async fn fetch_pv_data(
        &self,
        pv: &str,
        time_range: &TimeRange,
        resolution: Option<&str>,
    ) -> Result<(String, Vec<ProcessedPoint>)> {
        let cache_key = CacheKey::new(
            pv.to_string(),
            time_range.start,
            time_range.end,
            resolution.unwrap_or("raw").to_string(),
        );

        let data = self.cache.get_or_fetch_data(
            cache_key,
            || async {
                self.fetch_from_archiver(pv, time_range, resolution).await
            },
            resolution.unwrap_or("raw").to_string(),
        ).await?;

        Ok((pv.to_string(), data))
    }

    async fn fetch_from_archiver(
        &self,
        pv: &str,
        time_range: &TimeRange,
        resolution: Option<&str>,
    ) -> Result<Vec<ProcessedPoint>> {
        let mut url = self.base_url.join("retrieval/data/getData.json")
            .map_err(|e| ArchiverError::ConnectionError {
                message: "Failed to construct URL".into(),
                context: e.to_string(),
                source: Some(Box::new(e)),
                retry_after: None,
            })?;

        let processed_pv = if let Some(res) = resolution {
            format!("{}_{}", res, pv)
        } else {
            pv.to_string()
        };

        let query = [
            ("pv", processed_pv),
            ("from", time_range.start.to_string()),
            ("to", time_range.end.to_string()),
        ];
        url.query_pairs_mut().extend_pairs(query);

        let response = self.client.get(url.clone())
            .send()
            .await
            .map_err(|e| ArchiverError::ConnectionError {
                message: "Failed to fetch data".into(),
                context: format!("URL: {}", url),
                source: Some(Box::new(e)),
                retry_after: None,
            })?;

        if !response.status().is_success() {
            return Err(ArchiverError::ServerError {
                message: "Server returned error".into(),
                status: response.status().as_u16(),
                body: None,
                retry_after: None,
            });
        }

        let data: Value = response.json().await
            .map_err(|e| ArchiverError::DataError {
                message: "Failed to parse response".into(),
                context: format!("PV: {}", pv),
                source: Some(Box::new(e)),
                timestamp: Utc::now(),
                pv: Some(pv.to_string()),
            })?;

        // For now, return empty vec until response processing is implemented
        Ok(Vec::new())
    }

    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        self.health.get_current_status().await
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Archive Viewer API");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_api_initialization() {
        let api = ArchiveViewerApi::new("http://localhost:17665".to_string())
            .await
            .unwrap();
            
        assert!(Arc::strong_count(&api) == 1);
    }
}
// api.rs

use crate::{
    cache::CacheManager,
    error::{ArchiverError, Result},
    health::HealthMonitor,
    metrics::ApiMetrics,
    session::SessionManager,
    types::*,
    validation::{RequestValidator, Validator},
};

use chrono::{DateTime, Duration, Utc};
use futures::future::join_all;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
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
    config: Arc<RwLock<ApiConfig>>,
}

impl ArchiveViewerApi {
    /// Creates a new API instance with default configuration
    pub async fn new(base_url: String) -> Result<Arc<Self>> {
        let config = ApiConfig::default();
        Self::with_config(config).await
    }

    /// Creates a new API instance with custom configuration
    pub async fn with_config(config: ApiConfig) -> Result<Arc<Self>> {
        // Validate config
        config.validate()?;

        // Initialize client
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

        // Initialize components
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

        let api = Arc::new(Self {
            client,
            base_url: Url::parse(&base_url).map_err(|e| ArchiverError::ConnectionError {
                message: "Invalid base URL".into(),
                context: e.to_string(),
                source: Some(Box::new(e)),
                retry_after: None,
            })?,
            cache,
            sessions,
            health: health.clone(),
            metrics,
            validator,
            config: Arc::new(RwLock::new(config)),
        });

        // Start health monitoring
        health.clone().start();

        Ok(api)
    }

    /// Fetches data for a set of PVs
    pub async fn fetch_data(
        &self,
        session_id: Uuid,
        pvs: Vec<String>,
        time_range: TimeRange,
        resolution: Option<DataResolution>,
    ) -> Result<HashMap<String, Vec<ProcessedPoint>>> {
        // Validate session
        let session = self.sessions.get_session(session_id).await?;

        // Validate request parameters
        let params = RequestParameters {
            pvs: pvs.clone(),
            start_time: time_range.start,
            end_time: time_range.end,
            operator: resolution.as_ref().map(|r| r.to_string()),
            chart_width: None,
            options: None,
        };
        self.validator.validate_data_request(&params)?;

        // Start metrics collection
        let start = std::time::Instant::now();
        self.metrics.record_request();

        // Determine optimal resolution if not specified
        let resolution = resolution.unwrap_or_else(|| {
            let duration = time_range.end - time_range.start;
            DataResolution::get_optimal_resolution(
                duration,
                1000, // Default width
                None,
            )
        });

        // Create fetch tasks for each PV
        let mut tasks = Vec::new();
        for pv in pvs {
            let task = self.fetch_pv_data(pv, &time_range, &resolution).boxed();
            tasks.push(task);
        }

        // Execute all fetches concurrently
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

        // Record metrics
        self.metrics.record_latency(start.elapsed());

        Ok(data)
    }

    /// Fetches data for a single PV
    async fn fetch_pv_data(
        &self,
        pv: String,
        time_range: &TimeRange,
        resolution: &DataResolution,
    ) -> Result<(String, Vec<ProcessedPoint>)> {
        // Create cache key
        let cache_key = CacheKey::new(
            pv.clone(),
            time_range.start.timestamp(),
            time_range.end.timestamp(),
            resolution.clone(),
        );

        // Try to get from cache
        let data = self.cache.get_or_fetch_data(
            cache_key,
            || async {
                self.fetch_from_archiver(&pv, time_range, resolution).await
            },
            resolution.clone(),
        ).await?;

        Ok((pv, data))
    }

    /// Makes the actual request to the archiver
    async fn fetch_from_archiver(
        &self,
        pv: &str,
        time_range: &TimeRange,
        resolution: &DataResolution,
    ) -> Result<Vec<ProcessedPoint>> {
        let mut url = self.base_url.join("retrieval/data/getData.json")
            .map_err(|e| ArchiverError::ConnectionError {
                message: "Failed to construct URL".into(),
                context: e.to_string(),
                source: Some(Box::new(e)),
                retry_after: None,
            })?;

        // Construct PV name with operator if needed
        let processed_pv = match resolution {
            DataResolution::Raw => pv.to_string(),
            DataResolution::Optimized { display_width } => {
                format!("optimized({})", pv)
            },
            DataResolution::Binned { operator, bin_size } => {
                format!("{}_{}", operator, bin_size)
            }
        };

        // Add query parameters
        let query = [
            ("pv", processed_pv),
            ("from", time_range.start.to_rfc3339()),
            ("to", time_range.end.to_rfc3339()),
        ];
        url.query_pairs_mut().extend_pairs(query);

        // Make request
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

        // Parse response
        let data: Value = response.json().await
            .map_err(|e| ArchiverError::DataError {
                message: "Failed to parse response".into(),
                context: format!("PV: {}", pv),
                source: Some(Box::new(e)),
                timestamp: Utc::now(),
                pv: Some(pv.to_string()),
            })?;

        // Process and validate data
        self.process_response(data, resolution)
    }

    /// Processes the raw response into ProcessedPoints
    fn process_response(
        &self,
        data: Value,
        resolution: &DataResolution,
    ) -> Result<Vec<ProcessedPoint>> {
        // Implementation details for processing the response...
        // This would handle the various data formats and create ProcessedPoints
        todo!("Implement response processing")
    }

    /// Starts live updates for a set of PVs
    pub async fn start_live_updates(
        &self,
        session_id: Uuid,
        pvs: Vec<String>,
        update_interval: Duration,
        callback: impl Fn(HashMap<String, ProcessedPoint>) + Send + Sync + 'static,
    ) -> Result<()> {
        // Implementation for live updates...
        todo!("Implement live updates")
    }

    /// Stops live updates for a session
    pub async fn stop_live_updates(&self, session_id: Uuid) -> Result<()> {
        // Implementation for stopping live updates...
        todo!("Implement stop live updates")
    }

    /// Gets metadata for a PV
    pub async fn get_metadata(&self, pv: &str) -> Result<Meta> {
        // Implementation for getting metadata...
        todo!("Implement metadata retrieval")
    }

    /// Gets the current health status
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        self.health.get_current_status().await
    }

    /// Shuts down the API client
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Archive Viewer API");
        
        // Stop health monitoring
        // Clean up sessions
        // Stop any live updates
        // Flush caches if needed
        
        Ok(())
    }
}

impl Drop for ArchiveViewerApi {
    fn drop(&mut self) {
        // Ensure cleanup happens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;
    use mockito::mock;

    // Test constants
    const TEST_PV: &str = "TEST:PV1";
    const TEST_DATA: &str = r#"{
        "meta": {"name": "TEST:PV1", "EGU": "C"},
        "data": [
            {"secs": 1000, "nanos": 0, "val": 42.0},
            {"secs": 1001, "nanos": 0, "val": 43.0}
        ]
    }"#;

    #[test]
    async fn test_api_initialization() {
        let api = ArchiveViewerApi::new("http://localhost:17665".to_string())
            .await
            .unwrap();
            
        assert!(Arc::strong_count(&api) == 1);
    }

    #[test]
    async fn test_data_fetch() {
        let mut server = mockito::Server::new();
        
        let mock = server.mock("GET", "/retrieval/data/getData.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(TEST_DATA)
            .create();

        let api = ArchiveViewerApi::new(server.url())
            .await
            .unwrap();

        // Create a session
        let session = api.sessions.create_session(None).await.unwrap();

        // Fetch data
        let result = api.fetch_data(
            session.id,
            vec![TEST_PV.to_string()],
            TimeRange {
                start: Utc::now() - Duration::hours(1),
                end: Utc::now(),
            },
            None,
        ).await;

        assert!(result.is_ok());
        mock.assert();
    }

    // Add more tests...
}
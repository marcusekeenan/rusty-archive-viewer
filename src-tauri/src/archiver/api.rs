use crate::archiver::{
    cache::{CacheManager, CacheKey},
    error::{ArchiverError, ErrorContext, Result},
    health::{HealthMonitor, HealthStatus},
    metrics::ApiMetrics,
    types::*,
    validation::{RequestValidator, Validator},
    constants::API_CONFIG,
};

use chrono::{DateTime, Duration, Utc};
use futures::{
    future::{join_all, BoxFuture},
    FutureExt, StreamExt,
};
use reqwest::{Client, ClientBuilder, header};
use serde_json::Value;
use std::{
    sync::Arc,
    collections::HashMap,
    time::{Duration as StdDuration, Instant},
    num::NonZeroU32,
};
use tokio::{
    sync::{RwLock, Semaphore},
    time::sleep,
};
use tracing::{debug, error, info, warn, instrument};
use url::Url;
use governor::{
    Quota, RateLimiter, 
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
};

// Constants
const DEFAULT_ARCHIVER_URL: &str = "http://lcls-archapp.slac.stanford.edu";
const RETRIEVAL_BASE_PATH: &str = "retrieval/data";
const RETRIEVAL_ENDPOINT: &str = "getData.json";
const MANAGEMENT_PATH: &str = "mgmt/bpl";
const MAX_CONCURRENT_REQUESTS: usize = 10;
const MAX_RETRIES: u32 = 3;
const BASE_RETRY_DELAY: StdDuration = StdDuration::from_millis(100);
const DEFAULT_TIMEOUT: StdDuration = StdDuration::from_secs(30);

// Configuration
#[derive(Clone, Debug)]
pub struct Config {
    pub connection: ConnectionConfig,
    pub cache: CacheConfig,
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
pub struct MetricsConfig {
    pub collection_interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            connection: ConnectionConfig {
                timeout: DEFAULT_TIMEOUT,
                pool_size: 20,
            },
            cache: CacheConfig {
                max_entries: 10000,
                max_memory_mb: 1024,
            },
            metrics: MetricsConfig {
                collection_interval: Duration::seconds(60),
            },
        }
    }
}

// Main API struct
pub struct ArchiveViewerApi {
    client: Client,
    base_url: Url,
    cache: Arc<CacheManager>,
    health: Arc<HealthMonitor>,
    metrics: Arc<ApiMetrics>,
    validator: Arc<RequestValidator>,
    config: Arc<RwLock<Config>>,
    request_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    request_semaphore: Arc<Semaphore>,
}

impl ArchiveViewerApi {
    #[instrument(skip_all)]
    pub async fn new(base_url: Option<String>) -> Result<Arc<Self>> {
        let config = Config::default();
        let url = base_url.unwrap_or_else(|| DEFAULT_ARCHIVER_URL.to_string());
        Self::with_config(config, url).await
    }

    #[instrument(skip_all)]
    pub async fn with_config(config: Config, base_url_str: String) -> Result<Arc<Self>> {
        let error_context = ErrorContext::new("API", "initialization");

        debug!("Initializing API with base URL: {}", base_url_str);

        // Build HTTP client with default headers
        let mut headers = header::HeaderMap::new();
        headers.insert(header::ACCEPT, header::HeaderValue::from_static("application/json"));

        let client = ClientBuilder::new()
            .timeout(config.connection.timeout)
            .pool_max_idle_per_host(config.connection.pool_size)
            .default_headers(headers)
            .build()
            .map_err(|e| ArchiverError::InitializationError {
                message: "Failed to create HTTP client".into(),
                context: "Client initialization".into(),
                source: Some(Box::new(e)),
                error_context: Some(error_context.clone()),
            })?;

        // Parse and validate base URL
        let base_url = if !base_url_str.ends_with('/') {
            format!("{}/", base_url_str)
        } else {
            base_url_str
        };

        let base_url = Url::parse(&base_url).map_err(|e| ArchiverError::InitializationError {
            message: format!("Invalid base URL: {}", base_url),
            context: "URL parsing".into(),
            source: Some(Box::new(e)),
            error_context: Some(error_context.clone()),
        })?;

        // Initialize components
        let metrics = Arc::new(ApiMetrics::new());
        let cache = Arc::new(CacheManager::new(
            config.cache.max_entries,
            config.cache.max_memory_mb,
            metrics.clone(),
        ));
        let health = Arc::new(HealthMonitor::new(
            config.metrics.collection_interval,
            100,
        ));
        let validator = Arc::new(RequestValidator::new());

        // Configure rate limiting
        let quota = Quota::per_second(NonZeroU32::new(100).unwrap())
            .allow_burst(NonZeroU32::new(20).unwrap());
        let request_limiter = Arc::new(RateLimiter::direct(quota));
        let request_semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

        let api = Arc::new(Self {
            client,
            base_url,
            cache,
            health: health.clone(),
            metrics,
            validator,
            config: Arc::new(RwLock::new(config)),
            request_limiter,
            request_semaphore,
        });

        // Test connection
        if let Err(e) = api.verify_connection().await {
            error!("Failed to verify connection: {}", e);
            return Err(ArchiverError::InitializationError {
                message: "Failed to verify connection to archiver".into(),
                context: format!("Error: {}", e),
                source: Some(Box::new(e)),
                error_context: Some(error_context),
            });
        }

        health.clone().start();
        debug!("API initialization completed successfully");
        Ok(api)
    }

    // Connection verification
    async fn verify_connection(&self) -> Result<()> {
        let error_context = ErrorContext::new("API", "verify_connection");
        
        // First try a simple endpoint status check
        let status_url = self.base_url.join("retrieval/bpl/getVersion")
            .map_err(|e| {
                error!("Failed to construct status URL: {}", e);
                ArchiverError::ConnectionError {
                    message: "Failed to construct status URL".into(),
                    context: e.to_string(),
                    source: Some(Box::new(e)),
                    retry_after: None,
                    error_context: Some(error_context.clone()),
                }
            })?;

        debug!("Testing connection to: {}", status_url);

        let response = self.client.get(status_url)
            .send()
            .await
            .map_err(|e| {
                error!("Connection test failed: {}", e);
                ArchiverError::ConnectionError {
                    message: "Failed to connect to LCLS archiver".into(),
                    context: e.to_string(),
                    source: Some(Box::new(e)),
                    retry_after: None,
                    error_context: Some(error_context.clone()),
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Connection test failed with status {}: {}", status, body);
            return Err(ArchiverError::ConnectionError {
                message: format!("LCLS archiver returned error status: {}", status),
                context: body,
                source: None,
                retry_after: None,
                error_context: Some(error_context),
            });
        }

        Ok(())
    }

    // Main data fetching method
    #[instrument(skip(self))]
    pub async fn fetch_data(
        &self,
        pvs: Vec<String>,
        time_range: TimeRange,
        resolution: Option<String>,
    ) -> Result<HashMap<String, Vec<ProcessedPoint>>> {
        let error_context = ErrorContext::new("API", "fetch_data")
            .with_info(format!("pvs: {:?}", pvs));

        if pvs.is_empty() {
            return Err(ArchiverError::InvalidRequest {
                message: "No PVs specified".into(),
                context: "Fetch data".into(),
                validation_errors: vec!["PV list cannot be empty".into()],
                error_context: Some(error_context),
            });
        }

        debug!("Fetching data for {} PVs", pvs.len());
        let start = Instant::now();
        self.metrics.record_request();

        // Wait for rate limiter
        self.request_limiter.until_ready().await;

        // Create fetch tasks
        let mut tasks: Vec<BoxFuture<'_, Result<(String, Vec<ProcessedPoint>)>>> = Vec::new();
        for pv in pvs {
            let time_range = time_range.clone();
            let resolution = resolution.clone();
            
            let task = async move {
                self.fetch_pv_data(&pv, &time_range, resolution.as_deref()).await
            }.boxed();
            
            tasks.push(task);
        }

        // Execute tasks with concurrency limit
        let mut results = Vec::new();
        for task in tasks {
            let permit = self.request_semaphore.acquire().await.map_err(|e| {
                ArchiverError::ConnectionError {
                    message: "Failed to acquire request permit".into(),
                    context: "Semaphore acquisition".into(),
                    source: Some(Box::new(e)),
                    retry_after: Some(StdDuration::from_secs(1)),
                    error_context: Some(error_context.clone()),
                }
            })?;

            let result = task.await;
            drop(permit);
            results.push(result);
        }

        // Process results
        let mut data = HashMap::new();
        for result in results {
            match result {
                Ok((pv, points)) => {
                    data.insert(pv, points);
                }
                Err(e) => {
                    self.metrics.record_error(Some(e.to_string()));
                    error!("Error fetching data: {}", e);
                    return Err(e.add_context(error_context));
                }
            }
        }

        self.metrics.record_latency(start.elapsed());
        debug!("Fetch completed in {:?}", start.elapsed());

        Ok(data)
    }

    // Helper methods
    async fn fetch_pv_data(
        &self,
        pv: &str,
        time_range: &TimeRange,
        resolution: Option<&str>,
    ) -> Result<(String, Vec<ProcessedPoint>)> {
        let error_context = ErrorContext::new("API", "fetch_pv_data")
            .with_info(format!("pv: {}", pv));

        debug!("Fetching data for PV: {}", pv);

        let cache_key = CacheKey::new(
            pv.to_string(),
            time_range.start,
            time_range.end,
            resolution.unwrap_or("raw").to_string(),
        );

        let data = self.cache.get_or_fetch_data(
            cache_key.clone(),
            || async {
                self.fetch_from_archiver(pv, time_range, resolution).await
            },
            resolution.unwrap_or("raw").to_string(),
        )
        .await
        .map_err(|e| e.add_context(error_context.clone()))?;

        Ok((pv.to_string(), data))
    }

    fn format_timestamp(timestamp: i64) -> String {
        // Convert Unix timestamp to ISO8601
        let dt = chrono::DateTime::<Utc>::from_timestamp(timestamp, 0)
            .unwrap_or_else(|| Utc::now());
        dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    async fn fetch_from_archiver(
        &self,
        pv: &str,
        time_range: &TimeRange,
        resolution: Option<&str>,
    ) -> Result<Vec<ProcessedPoint>> {
        let error_context = ErrorContext::new("API", "fetch_from_archiver")
            .with_info(format!("pv: {}", pv));

        // Construct full URL with correct path structure
        let retrieval_path = format!("{}/{}", RETRIEVAL_BASE_PATH, RETRIEVAL_ENDPOINT);
        let mut url = self.base_url.join(&retrieval_path).map_err(|e| {
            error!("Failed to construct LCLS URL: {}", e);
            ArchiverError::ConnectionError {
                message: "Failed to construct LCLS URL".into(),
                context: format!("Base URL: {}, Path: {}", self.base_url, retrieval_path),
                source: Some(Box::new(e)),
                retry_after: None,
                error_context: Some(error_context.clone()),
            }
        })?;

        // Format timestamps as ISO8601
        let from_time = Self::format_timestamp(time_range.start);
        let to_time = Self::format_timestamp(time_range.end);

        debug!("Formatted time range: {} to {}", from_time, to_time);

        // Add query parameters
        url.query_pairs_mut()
            .append_pair("pv", pv)
            .append_pair("from", &from_time)
            .append_pair("to", &to_time);

        if let Some(res) = resolution {
            url.query_pairs_mut().append_pair("mean", res);
        }

        debug!("Making request to LCLS URL: {}", url);

        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count < MAX_RETRIES {
            match self.make_request(&url).await {
                Ok(data) => return Ok(data),
                Err(e) if e.is_retryable() => {
                    warn!("Retryable error encountered: {}, attempt {}/{}", 
                          e, retry_count + 1, MAX_RETRIES);
                    retry_count += 1;
                    last_error = Some(e);
                    let delay = self.calculate_retry_delay(retry_count);
                    sleep(delay).await;
                    continue;
                }
                Err(e) => return Err(e.add_context(error_context)),
            }
        }

        Err(last_error.unwrap_or_else(|| ArchiverError::ConnectionError {
            message: "Maximum retries exceeded".into(),
            context: "Archiver request".into(),
            source: None,
            retry_after: None,
            error_context: Some(error_context),
        }))
    }
    async fn make_request(&self, url: &Url) -> Result<Vec<ProcessedPoint>> {
        let response = self.client.get(url.clone())
            .send()
            .await
            .map_err(|e| ArchiverError::ConnectionError {
                message: "Failed to fetch data".into(),
                context: format!("URL: {}", url),
                source: Some(Box::new(e)),
                retry_after: None,
                error_context: None,
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            error!("Server returned error status: {}, body: {}", status, body);
            
            return Err(ArchiverError::ServerError {
                message: format!("Server returned error status: {} Not Found", status),
                status: status.as_u16(),
                body: Some(body),
                retry_after: None,
                error_context: None,
            });
        }

        let text = response.text().await.map_err(|e| ArchiverError::DataError {
            message: "Failed to get response text".into(),
            context: "Response reading".into(),
            source: Some(Box::new(e)),
            timestamp: Utc::now(),
            pv: None,
            error_context: None,
        })?;

        debug!("Received response: {}", text);

        let data: Value = serde_json::from_str(&text).map_err(|e| ArchiverError::DataError {
            message: "Failed to parse JSON response".into(),
            context: format!("Invalid JSON: {}", text),
            source: Some(Box::new(e)),
            timestamp: Utc::now(),
            pv: None,
            error_context: None,
        })?;

        self.process_response_data(data)
    }

    fn process_response_data(&self, data: Value) -> Result<Vec<ProcessedPoint>> {
        let values = match data {
            Value::Object(obj) => {
                if let Some(Value::Array(values)) = obj.get("values") {
                    values.clone() // Clone the array to own the values
                } else {
                    return Err(ArchiverError::DataError {
                        message: "Missing 'values' array in response".into(),
                        context: "Data processing".into(),
                        source: None,
                        timestamp: Utc::now(),
                        pv: None,
                        error_context: None,
                    });
                }
            }
            _ => {
                return Err(ArchiverError::DataError {
                    message: "Unexpected response format".into(),
                    context: format!("Received: {:?}", data),
                    source: None,
                    timestamp: Utc::now(),
                    pv: None,
                    error_context: None,
                });
            }
        };

        let mut points = Vec::with_capacity(values.len());
        for value in values {
            match self.convert_to_point(value) {
                Ok(point) => points.push(point),
                Err(e) => {
                    warn!("Failed to convert data point: {}", e);
                    continue;
                }
            }
        }

        Ok(points)
    }
   
    fn convert_to_point(&self, value: Value) -> Result<ProcessedPoint> {
        let timestamp = value.get("secs")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| ArchiverError::DataError {
                message: "Missing or invalid timestamp".into(),
                context: format!("Value: {:?}", value),
                source: None,
                timestamp: Utc::now(),
                pv: None,
                error_context: None,
            })?;

        let val = value.get("val")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| ArchiverError::DataError {
                message: "Missing or invalid value".into(),
                context: format!("Value: {:?}", value),
                source: None,
                timestamp: Utc::now(),
                pv: None,
                error_context: None,
            })?;

        Ok(ProcessedPoint {
            timestamp,
            severity: value.get("severity").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            status: value.get("status").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            value: val,
            min: value.get("min").and_then(|v| v.as_f64()).unwrap_or(val),
            max: value.get("max").and_then(|v| v.as_f64()).unwrap_or(val),
            stddev: value.get("stddev").and_then(|v| v.as_f64()).unwrap_or(0.0),
            count: value.get("count").and_then(|v| v.as_i64()).unwrap_or(1),
        })
    }

    fn calculate_retry_delay(&self, retry_count: u32) -> StdDuration {
        let base = BASE_RETRY_DELAY.as_millis() as u64;
        let max = StdDuration::from_secs(30).as_millis() as u64;
        let delay = base * (2_u64.pow(retry_count - 1));
        StdDuration::from_millis(delay.min(max))
    }

    #[instrument(skip(self))]
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        self.health.get_current_status().await
    }

    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Archive Viewer API");
        self.health.clone().start();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::test;
    use std::time::Duration;

    async fn setup_test_api() -> Arc<ArchiveViewerApi> {
        ArchiveViewerApi::new(Some("http://localhost:17665".to_string()))
            .await
            .expect("Failed to create test API")
    }

    #[test]
    async fn test_api_initialization() {
        let api = setup_test_api().await;
        assert!(Arc::strong_count(&api) == 1);
    }

    #[test]
    async fn test_fetch_data() {
        let api = setup_test_api().await;
        let time_range = TimeRange {
            start: 0,
            end: 100,
        };

        let result = api.fetch_data(
            vec!["TEST:PV1".to_string()],
            time_range,
            None,
        ).await;

        assert!(result.is_ok());
    }

    // #[test]
    // async fn test_rate_limiting() {
    //     let api = setup_test_api().await;
    //     let time_range = TimeRange {
    //         start: 0,
    //         end: 100,
    //     };

    //     let mut handles = vec![];
    //     for i in 0..5 {
    //         let api_clone = api.clone();
    //         let handle = tokio::spawn(async move {
    //             api_clone.fetch_data(
    //                 vec![format!("TEST:PV{}", i)],
    //                 time_range.clone(),
    //                 None,
    //             ).await
    //         });
    //         handles.push(handle);
    //     }

    //     for handle in handles {
    //         let result = handle.await.unwrap();
    //         assert!(result.is_ok());
    //     }
    // }

    #[test]
    async fn test_error_handling() {
        let api = setup_test_api().await;
        
        let result = api.fetch_data(
            vec!["INVALID:PV".to_string()],
            TimeRange {
                start: 100,
                end: 0,  // Invalid range
            },
            None,
        ).await;

        assert!(result.is_err());
    }

    #[test]
    async fn test_response_parsing() {
        let sample_data = serde_json::json!({
            "values": [
                {
                    "secs": 1234567890,
                    "val": 42.5,
                    "severity": 0,
                    "status": 0
                }
            ]
        });

        let api = setup_test_api().await;
        let result = api.process_response_data(sample_data);
        assert!(result.is_ok());
        let points = result.unwrap();
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].value, 42.5);
    }
}
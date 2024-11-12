//! EPICS Archiver Appliance API Interface
//! Handles data retrieval, optimization, and live updates for EPICS PVs

use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use futures::future::join_all;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::sync::{broadcast, Mutex, RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Interval};
use url::Url;

use crate::archiver::constants::{API_CONFIG, ERRORS};
use crate::archiver::types::*;

/// Defines how time ranges are specified for data retrieval
#[derive(Debug, Clone)]
pub enum TimeRangeMode {
    Fixed {
        start: i64,
        end: i64,
    },
    Rolling {
        duration: Duration,
        end: Option<i64>, // None means "now"
    },
    Live {
        base_mode: Box<TimeRangeMode>,
        last_update: i64,
    },
}

impl TimeRangeMode {
    pub fn get_range(&self) -> (i64, i64) {
        match self {
            TimeRangeMode::Fixed { start, end } => (*start, *end),
            TimeRangeMode::Rolling { duration, end } => {
                let end_time = end.unwrap_or_else(|| Utc::now().timestamp());
                let start_time = end_time - duration.as_secs() as i64;
                (start_time, end_time)
            }
            TimeRangeMode::Live { base_mode, .. } => base_mode.get_range(),
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self, TimeRangeMode::Live { .. })
    }
}

/// Data optimization configuration
#[derive(Debug, Clone, Copy)]
pub enum OptimizationLevel {
    Raw,
    Optimized(i32), // number of points
    Auto,           // decides based on time range
}

impl OptimizationLevel {
    pub fn get_operator(&self, duration: i64, chart_width: Option<i32>) -> DataOperator {
        match self {
            OptimizationLevel::Raw => DataOperator::Raw,
            OptimizationLevel::Optimized(points) => DataOperator::Optimized(*points),
            OptimizationLevel::Auto => {
                // Calculate actual data density (points per pixel)
                let points_per_pixel = match chart_width {
                    Some(width) if width > 0 => duration as f64 / width as f64,
                    _ => duration as f64 / 1000.0  // fallback to assuming 1000px width
                };

                // For time ranges <= 1 hour or when data would be sparse, use raw data
                if duration <= 3600 || points_per_pixel <= 1.0 {
                    DataOperator::Raw
                } else {
                    // For longer ranges, use optimized operator with number of points 
                    // based on chart width or a reasonable default
                    let target_points = match chart_width {
                        Some(width) if width > 0 => width * 2, // 2 points per pixel for good resolution
                        _ => 2000 // reasonable default for unknown width
                    };
                    
                    DataOperator::Optimized(target_points)
                }
            }
        }
    }
}

/// Request configuration for data fetching
#[derive(Debug, Clone)]
pub struct DataRequest {
    pub pv: String,
    pub range: TimeRange,
    pub operator: DataOperator,
    pub format: DataFormat,
    pub timezone: Option<String>,
}

/// Main client for interacting with the EPICS Archiver Appliance
pub struct ArchiverClient {
    client: Client,
    semaphore: Arc<Semaphore>,
    base_url: String,
    data_cache: Arc<RwLock<HashMap<String, NormalizedPVData>>>,
    live_update_tx: Arc<Mutex<Option<broadcast::Sender<HashMap<String, PointValue>>>>>,
    live_interval: Arc<Mutex<Option<Interval>>>,
    live_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    running: Arc<AtomicBool>,
    shutdown_signal: Arc<Notify>,
}

#[derive(Debug, Clone, Default)]
pub struct DataProcessor;

impl DataProcessor {
    pub fn calculate_chunks(&self, from: i64, to: i64, _width: Option<i32>) -> Vec<DataChunk> {
        let duration = to - from;
        let chunk_size = match duration {
            d if d <= 86400 => 3600,   // 1 hour chunks for <= 1 day
            d if d <= 604800 => 86400, // 1 day chunks for <= 1 week
            _ => 604800,               // 1 week chunks for > 1 week
        };

        let mut chunks = Vec::new();
        let mut current = from;

        while current < to {
            let chunk_end = (current + chunk_size).min(to);
            chunks.push(DataChunk {
                start: current,
                end: chunk_end,
            });
            current = chunk_end;
        }

        chunks
    }

    pub fn process_chunks(&self, chunks: Vec<PVData>) -> Result<NormalizedPVData, String> {
        if chunks.is_empty() {
            return Err(ERRORS.no_data.to_string());
        }

        let mut all_points = Vec::new();
        let meta = chunks[0].meta.clone();

        for chunk in chunks {
            for point in chunk.data {
                if let Some(value) = point.value_as_f64() {
                    // The timestamp is already in the correct timezone from the API
                    let timestamp = point.secs * 1000 + point.nanos.unwrap_or(0) / 1_000_000;

                    all_points.push(ProcessedPoint {
                        timestamp,
                        severity: point.severity.unwrap_or(0),
                        status: point.status.unwrap_or(0),
                        value,
                        min: value,
                        max: value,
                        stddev: 0.0,
                        count: 1,
                    });
                }
            }
        }

        all_points.sort_by_key(|p| p.timestamp);
        all_points.dedup_by_key(|p| p.timestamp);

        Ok(NormalizedPVData {
            meta,
            data: all_points.clone(),
            statistics: calculate_statistics(&all_points),
        })
    }
}

impl ArchiverClient {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(API_CONFIG.timeouts.default)
            .danger_accept_invalid_certs(true)
            .pool_max_idle_per_host(API_CONFIG.request_limits.max_concurrent)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let semaphore = Arc::new(Semaphore::new(API_CONFIG.request_limits.max_concurrent));

        Ok(Self {
            client,
            semaphore,
            base_url: API_CONFIG.base_url.to_string(),
            data_cache: Arc::new(RwLock::new(HashMap::new())),
            live_update_tx: Arc::new(Mutex::new(None)),
            live_interval: Arc::new(Mutex::new(None)),
            live_task: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_signal: Arc::new(Notify::new()),
        })
    }

    pub async fn is_live_task_running(&self) -> bool {
        let task_lock = self.live_task.lock().await;
        task_lock.is_some()
    }

    fn format_date(&self, timestamp_ms: i64, timezone: Option<&str>) -> Option<String> {
        let dt = Utc.timestamp_millis_opt(timestamp_ms).single()?;

        if let Some(tz_name) = timezone {
            if let Ok(tz) = tz_name.parse::<Tz>() {
                let localized_dt = dt.with_timezone(&tz);
                return Some(localized_dt.to_rfc3339());
            }
        }

        Some(dt.to_rfc3339())
    }

    pub fn build_url(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Url, String> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, endpoint))
            .map_err(|e| format!("Invalid URL: {}", e))?;

        {
            let mut query_pairs = url.query_pairs_mut();
            for &(key, value) in params {
                query_pairs.append_pair(key, value);
            }
        }

        Ok(url)
    }

    async fn get<T>(&self, url: Url) -> Result<T, String> 
where 
    T: DeserializeOwned,
{
    let _permit = self
        .semaphore
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

    println!("Making GET request to: {}", url);

    let response = self
        .client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    println!("Response status: {}", response.status());

    if !response.status().is_success() {
        return Err(format!(
            "{}: {} ({})",
            ERRORS.server_error,
            response.status(),
            url.as_str()
        ));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to get response text: {}", e))?;

    println!("Raw response text: {}", text);

    serde_json::from_str::<T>(&text)
        .map_err(|e| format!("Failed to parse JSON response: {} - Raw text: {}", e, text))
}


    pub async fn fetch_historical_data(
        &self,
        pv: &str,
        mode: &TimeRangeMode,
        optimization: OptimizationLevel,
        chart_width: Option<i32>,
        timezone: Option<&str>,
    ) -> Result<NormalizedPVData, String> {
        let (start, end) = mode.get_range();
        let duration = end - start;
        let operator = optimization.get_operator(duration, chart_width);

        let request = DataRequest {
            pv: pv.to_string(),
            range: TimeRange { start, end },
            operator,
            format: DataFormat::Json,
            timezone: timezone.map(String::from),
        };

        let processor = DataProcessor::default();
        let chunks = processor.calculate_chunks(request.range.start, request.range.end, None);

        let chunk_futures: Vec<_> = chunks
            .iter()
            .map(|chunk| {
                self.fetch_chunk_data(
                    &request.pv,
                    chunk,
                    &request.operator,
                    &request.format,
                    request.timezone.as_deref(),
                )
            })
            .collect();

        let results = join_all(chunk_futures).await;
        let mut chunk_data = Vec::new();

        for result in results {
            match result {
                Ok(data) => chunk_data.push(data),
                Err(e) => eprintln!("Warning: Failed to fetch chunk: {}", e),
            }
        }

        if chunk_data.is_empty() {
            return Err(ERRORS.no_data.to_string());
        }

        let result = processor.process_chunks(chunk_data)?;
        // Update cache
        let mut cache = self.data_cache.write().await;
        cache.insert(pv.to_string(), result.clone());

        Ok(result)
    }

    async fn fetch_chunk_data(
        &self,
        pv: &str,
        chunk: &DataChunk,
        operator: &DataOperator,
        format: &DataFormat,
        timezone: Option<&str>,
    ) -> Result<PVData, String> {
        if pv.is_empty() {
            return Err("PV name cannot be empty".to_string());
        }
        if chunk.end <= chunk.start {
            return Err("Invalid time range: end must be after start".to_string());
        }

        let from_formatted = self
            .format_date(chunk.start * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
        let to_formatted = self
            .format_date(chunk.end * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

        let pv_query = if operator.supports_binning() {
            format!("{}({})", operator.to_string(), pv)
        } else {
            pv.to_string()
        };

        let mut params = vec![
            ("pv", pv_query.as_str()),
            ("from", &from_formatted),
            ("to", &to_formatted),
        ];

        if let Some(tz) = timezone {
            params.push(("timeZone", tz));
        }

        let url = self.build_url(&format!("getData.{}", format.as_str()), &params)?;
        self.get(url).await.and_then(|data: Vec<PVData>| {
            data.into_iter()
                .next()
                .ok_or_else(|| ERRORS.no_data.to_string())
        })
    }

    pub async fn start_live_updates(
        &self,
        pvs: Vec<String>,
        update_interval: Duration,
        timezone: Option<String>,
    ) -> Result<broadcast::Receiver<HashMap<String, PointValue>>, String> {
        println!("Starting live updates with interval {:?}", update_interval);

        // Reset state
        self.running.store(true, Ordering::SeqCst);

        // Create new channel with larger buffer
        let (tx, rx) = broadcast::channel(100);

        {
            let mut tx_lock = self.live_update_tx.lock().await;
            *tx_lock = Some(tx.clone());
        }

        let client = self.clone();
        let pvs = Arc::new(pvs);
        let timezone = Arc::new(timezone);
        let shutdown = self.shutdown_signal.clone();
        let running = self.running.clone();

        println!("Creating live update task for PVs: {:?}", pvs);

        let task = tokio::spawn(async move {
            println!("Live update task started");
            let mut interval = tokio::time::interval(update_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            while running.load(Ordering::SeqCst) {
                tokio::select! {
                    _ = shutdown.notified() => {
                        println!("Shutdown signal received in live update task");
                        break;
                    }
                    _ = interval.tick() => {
                        // Check if there are any receivers
                        if tx.receiver_count() == 0 {
                            println!("No receivers left, stopping updates");
                            break;
                        }

                        println!("Tick received, fetching live data");
                        match client.fetch_live_data(&pvs, timezone.as_deref()).await {
                            Ok(data) => {
                                if !data.is_empty() {
                                    println!("Fetched {} points of live data", data.len());
                                    match tx.send(data) {
                                        Ok(_) => println!("Live data sent successfully"),
                                        Err(e) => {
                                            println!("Error sending live data: {}", e);
                                            break;
                                        }
                                    }
                                } else {
                                    println!("No live data received");
                                }
                            }
                            Err(e) => {
                                println!("Error fetching live data: {}", e);
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                        }
                    }
                }
            }
            println!("Live update task ending gracefully");
        });

        {
            let mut task_lock = self.live_task.lock().await;
            *task_lock = Some(task);
            println!("Live update task stored");
        }

        println!("Live updates started successfully");
        Ok(rx)
    }

    pub async fn stop_live_updates(&self) -> Result<(), String> {
        println!("Stopping live updates...");

        // Set running to false first
        self.running.store(false, Ordering::SeqCst);

        // Signal shutdown
        self.shutdown_signal.notify_waiters();
        println!("Shutdown signal sent");

        // Clear transmitter
        {
            let mut tx_lock = self.live_update_tx.lock().await;
            *tx_lock = None;
            println!("Transmitter cleared");
        }

        // Wait for task to complete
        {
            let mut task_lock = self.live_task.lock().await;
            if let Some(mut task) = task_lock.take() {
                println!("Waiting for task to complete...");
                if !task.is_finished() {
                    match tokio::time::timeout(Duration::from_secs(2), &mut task).await {
                        Ok(join_result) => {
                            if let Err(e) = join_result {
                                println!("Task ended with error: {}", e);
                            } else {
                                println!("Task completed successfully");
                            }
                        }
                        Err(_) => {
                            println!("Task cleanup timed out, aborting");
                            task.abort();
                        }
                    }
                }
                println!("Task completed");
            }
        }

        // Clear interval
        {
            let mut interval_lock = self.live_interval.lock().await;
            *interval_lock = None;
            println!("Interval cleared");
        }

        println!("Live updates stopped successfully");
        Ok(())
    }

    pub async fn fetch_live_data(
        &self,
        pvs: &[String],
        timezone: Option<&str>,
    ) -> Result<HashMap<String, PointValue>, String> {
        if pvs.is_empty() {
            return Ok(HashMap::new());
        }
    
        println!("=== Starting live data fetch ===");
        println!("Requested PVs: {:?}", pvs);
    
        let now = Utc::now().timestamp();
        let five_seconds_ago = now - 5;
    
        let from_formatted = self
            .format_date(five_seconds_ago * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
        let to_formatted = self
            .format_date(now * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
    
        // Create base parameters
        let base_params = [
            ("from", from_formatted.as_str()),
            ("to", to_formatted.as_str())
        ];
    
        let mut results = HashMap::new();
        let mut futures = Vec::new();
    
        // Create a request for each PV
        for pv in pvs {
            let mut url = self.build_url("getData.json", &base_params)?;
            {
                let mut query_pairs = url.query_pairs_mut();
                query_pairs.append_pair("pv", pv);
            }
            
            println!("Created request URL for {}: {}", pv, url);
            let future = self.get::<Vec<PVData>>(url);
            futures.push((pv.clone(), future));
        }
    
        // Execute all requests concurrently
        for (pv_name, future) in futures {
            match future.await {
                Ok(mut response) => {
                    println!("Received data for PV {}", pv_name);
                    if let Some(pv_data) = response.pop() {
                        if let Some(last_point) = pv_data.data.last() {
                            println!("Found last point for {}: {:?}", pv_name, last_point);
    
                            let value = match &last_point.val {
                                Value::Single(v) => {
                                    println!("Single value for {}: {}", pv_name, v);
                                    *v
                                },
                                Value::Array(arr) if !arr.is_empty() => {
                                    println!("Array value for {}: {:?}", pv_name, arr);
                                    arr[0]
                                },
                                _ => {
                                    println!("Invalid value format for {}", pv_name);
                                    continue;
                                }
                            };
    
                            let point = PointValue {
                                secs: last_point.secs,
                                nanos: last_point.nanos,
                                val: Value::Single(value),
                                severity: last_point.severity,
                                status: last_point.status,
                            };
    
                            results.insert(pv_name.clone(), point);
                            println!("Successfully added point for {}", pv_name);
                        } else {
                            println!("No data points found for {}", pv_name);
                        }
                    }
                }
                Err(e) => {
                    println!("Error fetching data for {}: {}", pv_name, e);
                }
            }
        }
    
        // Verify all requested PVs are in results
        for pv in pvs {
            if !results.contains_key(pv) {
                println!("WARNING: Missing data for requested PV: {}", pv);
            }
        }
    
        println!("=== Completed live data fetch ===");
        println!("Returning data for {} PVs: {:?}", results.len(), results.keys().collect::<Vec<_>>());
    
        Ok(results)
    }

    pub async fn ensure_tasks_stopped(&self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Live updates still running".to_string());
        }

        let task_exists = {
            let task_lock = self.live_task.lock().await;
            task_lock.is_some()
        };

        if task_exists {
            return Err("Task still exists".to_string());
        }

        Ok(())
    }

    pub async fn fetch_data_at_time(
        &self,
        pvs: &[String],
        timestamp: Option<i64>,
        timezone: Option<&str>,
    ) -> Result<HashMap<String, PointValue>, String> {
        if pvs.is_empty() {
            return Ok(HashMap::new());
        }

        let timestamp = timestamp.unwrap_or_else(|| Utc::now().timestamp());
        let timestamp_formatted = self
            .format_date(timestamp * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

        let mut params = vec![("at", timestamp_formatted.as_str())];

        if let Some(tz) = timezone {
            params.push(("timeZone", tz));
        }

        let mut url = self.build_url("getData.json", &params)?;

        {
            let mut query_pairs = url.query_pairs_mut();
            for pv in pvs {
                query_pairs.append_pair("pv", pv);
            }
        }

        let response: Vec<PVData> = self.get(url).await?;

        Ok(response
            .into_iter()
            .filter_map(|pv_data| {
                pv_data.data.first().map(|point| {
                    (
                        pv_data.meta.name.clone(),
                        PointValue {
                            secs: point.secs,
                            nanos: point.nanos,
                            val: point.val.clone(),
                            severity: point.severity,
                            status: point.status,
                        },
                    )
                })
            })
            .collect())
    }

    pub async fn fetch_metadata(&self, pv: &str) -> Result<Meta, String> {
        let url = self.build_url("bpl/getMetadata", &[("pv", pv)])?;
        self.get(url).await
    }

    pub async fn fetch_multiple_metadata(
        &self,
        pvs: &[String],
    ) -> HashMap<String, Result<Meta, String>> {
        let futures: Vec<_> = pvs
            .iter()
            .map(|pv| async {
                let result = self.fetch_metadata(pv).await;
                (pv.clone(), result)
            })
            .collect();

        join_all(futures).await.into_iter().collect()
    }
}

impl Clone for ArchiverClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            semaphore: self.semaphore.clone(),
            base_url: self.base_url.clone(),
            data_cache: self.data_cache.clone(),
            live_update_tx: self.live_update_tx.clone(),
            live_interval: self.live_interval.clone(),
            live_task: self.live_task.clone(),
            running: self.running.clone(),
            shutdown_signal: self.shutdown_signal.clone(),
        }
    }
}

fn calculate_statistics(points: &[ProcessedPoint]) -> Option<Statistics> {
    if points.is_empty() {
        return None;
    }

    let values: Vec<f64> = points.iter().map(|p| p.value).collect();
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;

    Some(Statistics {
        mean,
        std_dev: variance.sqrt(),
        min: values.iter().copied().fold(f64::INFINITY, f64::min),
        max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        count: values.len() as i64,
        first_timestamp: points.first().map(|p| p.timestamp).unwrap_or(0),
        last_timestamp: points.last().map(|p| p.timestamp).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_historical_data_fetch() {
        let client = ArchiverClient::new().unwrap();
        let mode = TimeRangeMode::Fixed {
            start: Utc::now().timestamp() - 3600,
            end: Utc::now().timestamp(),
        };

        let result = client
            .fetch_historical_data(
                "ROOM:LI30:1:OUTSIDE_TEMP",
                &mode,
                OptimizationLevel::Auto,
                Some(1000),
                Some("UTC"),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_live_updates() {
        let client = ArchiverClient::new().unwrap();
        let pvs = vec!["ROOM:LI30:1:OUTSIDE_TEMP".to_string()];

        let mut rx = client
            .start_live_updates(pvs.clone(), Duration::from_secs(1), Some("UTC".to_string()))
            .await
            .unwrap();

        // Wait for some updates
        let mut update_count = 0;
        while update_count < 3 {
            if let Ok(data) = rx.recv().await {
                assert!(!data.is_empty());
                update_count += 1;
            }
            sleep(Duration::from_secs(1)).await;
        }

        let stop_result = client.stop_live_updates().await;
        assert!(stop_result.is_ok());
    }

    #[tokio::test]
    async fn test_rolling_window() {
        let client = ArchiverClient::new().unwrap();

        // Set the rolling window duration to 1 hour (3600 seconds)
        let mode = TimeRangeMode::Rolling {
            duration: Duration::from_secs(3600),
            end: None,
        };

        // Calculate the expected start and end times before fetching data
        let now = Utc::now().timestamp() * 1000;
        let start_time = now - 3600000; // 1 hour ago in milliseconds
        let buffer = 10000; // Allow a 10-second buffer on either end

        // Fetch historical data for the rolling window
        let result = client
            .fetch_historical_data(
                "ROOM:LI30:1:OUTSIDE_TEMP",
                &mode,
                OptimizationLevel::Auto,
                Some(1000),
                Some("UTC"),
            )
            .await;

        assert!(result.is_ok(), "Failed to fetch historical data");

        if let Ok(data) = result {
            for point in data.data {
                // Assert that each point falls within the buffered time range
                assert!(
                    (start_time - buffer) <= point.timestamp && point.timestamp <= (now + buffer),
                    "Point timestamp out of expected range: {} not within {} - {}",
                    point.timestamp,
                    start_time - buffer,
                    now + buffer
                );
            }
        }
    }

    #[tokio::test]
    async fn test_concurrent_requests() {
        let client = ArchiverClient::new().unwrap();
        let mode = TimeRangeMode::Fixed {
            start: Utc::now().timestamp() - 3600,
            end: Utc::now().timestamp(),
        };

        let pvs = vec![
            "ROOM:LI30:1:OUTSIDE_TEMP",
            "CTE:CM33:2502:B1:TEMP",
            "CTE:CM34:2502:B1:TEMP",
            "CTE:CM35:2502:B1:TEMP",
        ];

        let futures: Vec<_> = pvs
            .iter()
            .map(|&pv| {
                client.fetch_historical_data(
                    pv,
                    &mode,
                    OptimizationLevel::Auto,
                    Some(1000),
                    Some("UTC"),
                )
            })
            .collect();

        let results = join_all(futures).await;
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn test_cache_behavior() {
        let client = ArchiverClient::new().unwrap();
        let pv = "ROOM:LI30:1:OUTSIDE_TEMP";

        // Initial fetch
        let mode = TimeRangeMode::Fixed {
            start: Utc::now().timestamp() - 3600,
            end: Utc::now().timestamp(),
        };

        let result = client
            .fetch_historical_data(pv, &mode, OptimizationLevel::Auto, Some(1000), Some("UTC"))
            .await;

        assert!(result.is_ok());

        // Verify cache
        let cache = client.data_cache.read().await;
        assert!(cache.contains_key(pv));
    }
}

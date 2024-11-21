//! EPICS Archiver Appliance API Interface
//! Provides efficient data retrieval and processing for EPICS PVs

use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use dashmap::DashMap;
use futures::future::join_all;
use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration as StdDuration, SystemTime};
use tokio::sync::Notify;
use tokio::sync::{broadcast, Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Duration;
use url::Url;

use super::{
    DataFormat, DataOperator, NormalizedPVData, PVData, PointValue, ProcessedPoint, Statistics,
    TimeRange,
};
use crate::archiver::constants::{API_CONFIG, ERRORS};
use crate::archiver::types::{DebugEvent, Meta, Value};

// Constants
const CACHE_TTL: StdDuration = StdDuration::from_secs(300); // 5 minutes
const MAX_CONCURRENT_REQUESTS: usize = 50;

const MAX_PVS_PER_REQUEST: usize = 100; // Adjust based on server capabilities

// Debug print macro definition
macro_rules! debug_print {
    ($($arg:tt)*) => {{
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    }};
}

// Type definitions needed from the module
#[derive(Debug, Clone)]
pub enum TimeRangeMode {
    Fixed {
        start: i64,
        end: i64,
    },
    Rolling {
        duration: Duration,
        end: Option<i64>,
    },
    Live {
        base_mode: Box<TimeRangeMode>,
        last_update: i64,
    },
}

impl TimeRangeMode {
    pub fn get_range(&self) -> (i64, i64) {
        match self {
            Self::Fixed { start, end } => (*start, *end),
            Self::Rolling { duration, end } => {
                let end_time = end.unwrap_or_else(|| Utc::now().timestamp());
                (end_time - duration.as_secs() as i64, end_time)
            }
            Self::Live { base_mode, .. } => base_mode.get_range(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OptimizationLevel {
    Raw,                    // Raw data, no processing
    Optimized(i32),        // Default optimized mode with point count
    Auto,                  // Automatic optimization with default points
    Mean(i32),            // Mean with bin size
    FirstSample(i32),     // First sample in bin
    LastSample(i32),      // Last sample in bin
    Min(i32),             // Minimum value in bin
    Max(i32),             // Maximum value in bin
}

impl OptimizationLevel {
    pub fn get_operator(&self, _duration: i64, target_points: Option<i32>) -> DataOperator {
        match self {
            OptimizationLevel::Raw => DataOperator::Raw,
            // Optimized mode handles both raw and binned data automatically
            OptimizationLevel::Optimized(points) => DataOperator::Optimized(*points),
            // Auto mode uses optimized with default or specified points
            OptimizationLevel::Auto => DataOperator::Optimized(target_points.unwrap_or(1000)),
            // Direct operator mappings for specific processing needs
            OptimizationLevel::Mean(interval) => DataOperator::Mean(Some(*interval)),
            OptimizationLevel::FirstSample(interval) => DataOperator::FirstSample(Some(*interval)),
            OptimizationLevel::LastSample(interval) => DataOperator::LastSample(Some(*interval)),
            OptimizationLevel::Min(interval) => DataOperator::Min(Some(*interval)),
            OptimizationLevel::Max(interval) => DataOperator::Max(Some(*interval)),
        }
    }
}


// Helper trait for value extraction
trait ValueExt {
    fn value_as_f64(&self) -> Option<f64>;
}

impl ValueExt for Value {
    fn value_as_f64(&self) -> Option<f64> {
        match self {
            Value::Single(v) => Some(*v),
            Value::Array(arr) if !arr.is_empty() => Some(arr[0]),
            _ => None,
        }
    }
}

impl ValueExt for PointValue {
    fn value_as_f64(&self) -> Option<f64> {
        self.val.value_as_f64()
    }
}

// Cache entry with expiration
#[derive(Clone)]
struct CacheEntry {
    data: NormalizedPVData,
    expires_at: SystemTime,
}

impl CacheEntry {
    fn new(data: NormalizedPVData, ttl: StdDuration) -> Self {
        Self {
            data,
            expires_at: SystemTime::now() + ttl,
        }
    }

    fn is_valid(&self) -> bool {
        SystemTime::now() < self.expires_at
    }
}

#[derive(Debug, Clone)]
pub struct DataRequest {
    pub pv: String,
    pub range: TimeRange,
    pub operator: DataOperator,
    pub format: DataFormat,
    pub timezone: Option<String>,
}

// Data processor definition
#[derive(Debug, Clone)]
pub struct DataProcessor {
    debug_sender: Option<broadcast::Sender<DebugEvent>>,
}

impl DataProcessor {
    pub fn new() -> Self {
        Self { debug_sender: None }
    }

    pub fn with_debug(debug_sender: broadcast::Sender<DebugEvent>) -> Self {
        Self {
            debug_sender: Some(debug_sender),
        }
    }

    pub fn process_data(&self, data: PVData) -> Result<NormalizedPVData, String> {
        let mut points = Vec::with_capacity(data.data.len());
        let meta = data.meta.clone();

        let display_range = meta.get_display_range();

        for point in data.data {
            if let Some(value) = point.value_as_f64() {
                if let Some((low, high)) = display_range {
                    if value < low || value > high {
                        if let Some(ref sender) = self.debug_sender {
                            let _ = sender.send(DebugEvent {
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                level: "warn".to_string(),
                                message: format!(
                                    "Value {} outside range [{}, {}] for {}",
                                    value, low, high, meta.name
                                ),
                                details: None,
                            });
                        }
                    }
                }

                let timestamp = point.secs * 1000 + point.nanos.unwrap_or(0) / 1_000_000;
                points.push(ProcessedPoint {
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

        points.sort_by_key(|p| p.timestamp);
        points.dedup_by_key(|p| p.timestamp);

        Ok(NormalizedPVData {
            meta,
            data: points.clone(),
            statistics: calculate_statistics(&points),
        })
    }
}

impl Default for DataProcessor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ArchiverClient {
    client: Client,
    semaphore: Arc<Semaphore>,
    base_url: String,
    data_cache: Arc<DashMap<String, CacheEntry>>,
    live_update_tx: Arc<Mutex<Option<broadcast::Sender<HashMap<String, PointValue>>>>>,
    live_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    running: Arc<AtomicBool>,
    shutdown_signal: Arc<Notify>,
    processor: DataProcessor,
}

impl ArchiverClient {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(API_CONFIG.timeouts.default)
            .danger_accept_invalid_certs(true)
            .pool_max_idle_per_host(MAX_CONCURRENT_REQUESTS)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

        Ok(Self {
            client,
            semaphore,
            base_url: API_CONFIG.base_url.to_string(),
            data_cache: Arc::new(DashMap::new()),
            live_update_tx: Arc::new(Mutex::new(None)),
            live_task: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_signal: Arc::new(Notify::new()),
            processor: DataProcessor::default(),
        })
    }

    // Cache handling
    async fn get_cached_or_fetch<F>(
        &self,
        cache_key: &str,
        fetch_fn: F,
    ) -> Result<NormalizedPVData, String>
    where
        F: std::future::Future<Output = Result<NormalizedPVData, String>>,
    {
        // Check cache first
        if let Some(entry) = self.data_cache.get(cache_key) {
            if entry.is_valid() {
                return Ok(entry.data.clone());
            }
            self.data_cache.remove(cache_key);
        }

        // Fetch new data
        let data = fetch_fn.await?;

        // Cache the result
        self.data_cache.insert(
            cache_key.to_string(),
            CacheEntry::new(data.clone(), CACHE_TTL),
        );

        Ok(data)
    }

    fn format_date(&self, timestamp_ms: i64, timezone: Option<&str>) -> Option<String> {
        let dt = Utc.timestamp_millis_opt(timestamp_ms).single()?;
        if let Some(tz_name) = timezone {
            if let Ok(tz) = tz_name.parse::<Tz>() {
                return Some(dt.with_timezone(&tz).to_rfc3339());
            }
        }
        Some(dt.to_rfc3339())
    }

    pub fn build_url(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Url, String> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, endpoint))
            .map_err(|e| format!("Invalid URL: {}", e))?;
        url.query_pairs_mut().extend_pairs(params);
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

        debug_print!("Request URL: {}", url);

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

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

        serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {}", e))
    }

    pub async fn fetch_historical_data(
        &self,
        pv: &str,
        mode: &TimeRangeMode,
        optimization: OptimizationLevel,
        target_points: Option<i32>,
        timezone: Option<&str>,
    ) -> Result<NormalizedPVData, String> {
        let (start, end) = mode.get_range();
        let cache_key = format!("{}:{}:{}:{:?}", pv, start, end, optimization);
    
        self.get_cached_or_fetch(&cache_key, async {
            let duration = end - start;
            let operator = optimization.get_operator(duration, target_points);
    
            let data = self
                .fetch_data_with_operator(pv, start, end, &operator, timezone)
                .await?;
    
            self.processor.process_data(data)
        })
        .await
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

        let mut url = self.build_url(
            "data/getDataAtTime",
            &[
                ("at", timestamp_formatted.as_str()),
                ("includeProxies", "true"),
            ],
        )?;

        if let Some(tz) = timezone {
            url.query_pairs_mut().append_pair("timeZone", tz);
        }

        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

        debug_print!("Request URL: {}", url);

        let response = self
            .client
            .post(url.clone())
            .json(pvs)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "{}: {} ({})",
                ERRORS.server_error,
                response.status(),
                url.as_str()
            ));
        }

        let data: HashMap<String, PointValue> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        Ok(data)
    }

    async fn fetch_data_with_operator(
        &self,
        pv: &str,
        start: i64,
        end: i64,
        operator: &DataOperator,
        timezone: Option<&str>,
    ) -> Result<PVData, String> {
        let from_formatted = self
            .format_date(start * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
        let to_formatted = self
            .format_date(end * 1000, timezone)
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
            ("fetchLatestMetadata", "true"),
        ];

        if let Some(tz) = timezone {
            params.push(("timeZone", tz));
        }

        let url = self.build_url("getData.json", &params)?;

        self.get::<Vec<PVData>>(url)
            .await
            .and_then(|mut data| data.pop().ok_or_else(|| ERRORS.no_data.to_string()))
    }

    pub async fn fetch_multiple_pvs(
        &self,
        pvs: &[String],
        mode: &TimeRangeMode,
        optimization: Option<OptimizationLevel>,
        target_points: Option<i32>,
        timezone: Option<&str>,
    ) -> Result<Vec<NormalizedPVData>, String> {
        if pvs.is_empty() {
            return Ok(Vec::new());
        }
    
        let (start, end) = mode.get_range();
        let duration = end - start;
        let optimization = optimization.unwrap_or(OptimizationLevel::Auto);
        let operator = optimization.get_operator(duration, target_points);
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));
    
        let results = stream::iter(pvs.chunks(MAX_PVS_PER_REQUEST))
            .map(|chunk| {
                let chunk = chunk.to_vec();
                let semaphore = semaphore.clone();
                let client = self.clone();
                let operator = operator.clone();
                let timezone = timezone.map(String::from);

                async move {
                    let _permit = semaphore
                        .acquire()
                        .await
                        .expect("Semaphore should not be closed");
                    client
                        .fetch_chunk_data(&chunk, start, end, &operator, timezone.as_deref())
                        .await
                }
            })
            .buffer_unordered(MAX_CONCURRENT_REQUESTS)
            .collect::<Vec<_>>()
            .await;

        let mut normalized_data = Vec::new();
        for result in results {
            match result {
                Ok(mut chunk_data) => normalized_data.append(&mut chunk_data),
                Err(e) => debug_print!("Failed to process data chunk: {}", e),
            }
        }

        if normalized_data.is_empty() {
            return Err("No data available for any PV".to_string());
        }

        Ok(normalized_data)
    }

    async fn fetch_chunk_data(
        &self,
        pvs: &[String],
        start: i64,
        end: i64,
        operator: &DataOperator,
        timezone: Option<&str>,
    ) -> Result<Vec<NormalizedPVData>, String> {
        let futures = pvs.iter().map(|pv| {
            let cache_key = format!("{}:{}:{}:{:?}", pv, start, end, operator);
            let pv = pv.to_string();
            let operator = operator.clone();

            async move {
                self.get_cached_or_fetch(&cache_key, async move {
                    let mut url = self.build_url(
                        "data/getData.json",
                        &[
                            ("pv", &pv),
                            ("from", &self.format_date(start * 1000, timezone).unwrap()),
                            ("to", &self.format_date(end * 1000, timezone).unwrap()),
                            ("fetchLatestMetadata", "true"),
                        ],
                    )?;

                    let op_str = operator.to_string();
                    if !op_str.is_empty() {
                        url.query_pairs_mut().append_pair("donotchunk", "true");
                        url.query_pairs_mut().append_pair("op", &op_str);
                    }

                    let data: Vec<PVData> = self.get(url).await?;

                    // Assume we only get one PVData per request
                    if let Some(pv_data) = data.into_iter().next() {
                        self.processor.process_data(pv_data)
                    } else {
                        Err("No data returned for PV".to_string())
                    }
                })
                .await
            }
        });

        let results: Vec<Result<NormalizedPVData, String>> = stream::iter(futures)
            .buffer_unordered(MAX_CONCURRENT_REQUESTS)
            .collect()
            .await;

        // Collect successful results, return error if all failed
        let (successes, errors): (Vec<_>, Vec<_>) = results.into_iter().partition(Result::is_ok);

        if successes.is_empty() {
            Err(errors
                .into_iter()
                .map(Result::unwrap_err)
                .collect::<Vec<_>>()
                .join(", "))
        } else {
            Ok(successes.into_iter().map(Result::unwrap).collect())
        }
    }

    async fn fetch_live_data(
        &self,
        pvs: &[String],
        timezone: Option<&str>,
    ) -> Result<HashMap<String, PointValue>, String> {
        if pvs.is_empty() {
            return Ok(HashMap::new());
        }

        let now = Utc::now().timestamp();
        let five_seconds_ago = now - 5;

        let from_formatted = self
            .format_date(five_seconds_ago * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
        let to_formatted = self
            .format_date(now * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

        let base_params = [
            ("from", from_formatted.as_str()),
            ("to", to_formatted.as_str()),
            ("fetchLatestMetadata", "true"),
        ];

        let futures: Vec<_> = pvs
            .chunks(MAX_CONCURRENT_REQUESTS)
            .map(|chunk| {
                let requests = chunk.iter().map(|pv| {
                    let url = self.build_url("getData.json", &base_params).map(|mut u| {
                        u.query_pairs_mut().append_pair("pv", pv);
                        u
                    });
                    let pv = pv.clone();

                    async move {
                        match url {
                            Ok(u) => match self.get::<Vec<PVData>>(u).await {
                                Ok(mut data) => data.pop().and_then(|pv_data| {
                                    pv_data.data.last().map(|point| {
                                        (
                                            pv,
                                            PointValue {
                                                secs: point.secs,
                                                nanos: point.nanos,
                                                val: point.val.clone(),
                                                severity: point.severity,
                                                status: point.status,
                                            },
                                        )
                                    })
                                }),
                                Err(_) => None,
                            },
                            Err(_) => None,
                        }
                    }
                });
                join_all(requests)
            })
            .collect();

        let all_results = join_all(futures).await;
        Ok(all_results.into_iter().flatten().flatten().collect())
    }

    pub async fn start_live_updates(
        &self,
        pvs: Vec<String>,
        update_interval: Duration,
        timezone: Option<String>,
    ) -> Result<broadcast::Receiver<HashMap<String, PointValue>>, String> {
        debug_print!("Starting live updates for {} PVs", pvs.len());

        self.running.store(true, Ordering::SeqCst);
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
        let cache = self.data_cache.clone();

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            let mut last_cache_cleanup = SystemTime::now();
            let cleanup_interval = StdDuration::from_secs(60);

            while running.load(Ordering::SeqCst) {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = interval.tick() => {
                        if last_cache_cleanup.elapsed().unwrap_or_default() > cleanup_interval {
                            cache.retain(|_, entry| entry.is_valid());
                            last_cache_cleanup = SystemTime::now();
                        }

                        if tx.receiver_count() == 0 {
                            break;
                        }

                        match client.fetch_live_data(&pvs, timezone.as_deref()).await {
                            Ok(data) if !data.is_empty() => {
                                for (pv, point) in data.iter() {
                                    if let Some(mut entry) = cache.get_mut(pv) {
                                        if let Some(last_point) = entry.data.data.last_mut() {
                                            last_point.timestamp = point.secs * 1000 + point.nanos.unwrap_or(0) / 1_000_000;
                                            if let Some(value) = point.value_as_f64() {
                                                last_point.value = value;
                                            }
                                            entry.expires_at = SystemTime::now() + CACHE_TTL;
                                        }
                                    }
                                }

                                if tx.send(data).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                debug_print!("Live data error: {}", e);
                                tokio::time::sleep(Duration::from_secs(1)).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        {
            let mut task_lock = self.live_task.lock().await;
            *task_lock = Some(task);
        }

        Ok(rx)
    }

    pub async fn stop_live_updates(&self) -> Result<(), String> {
        debug_print!("Stopping live updates");
        self.running.store(false, Ordering::SeqCst);
        self.shutdown_signal.notify_waiters();

        {
            let mut tx_lock = self.live_update_tx.lock().await;
            *tx_lock = None;
        }

        {
            let mut task_lock = self.live_task.lock().await;
            if let Some(mut task) = task_lock.take() {
                if !task.is_finished() {
                    match tokio::time::timeout(Duration::from_secs(2), &mut task).await {
                        Ok(join_result) => {
                            if let Err(e) = join_result {
                                debug_print!("Task ended with error: {}", e);
                            }
                        }
                        Err(_) => {
                            debug_print!("Task cleanup timed out, aborting");
                            task.abort();
                        }
                    }
                }
            }
        }

        self.ensure_tasks_stopped().await?;
        Ok(())
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
            return Err("Task exists but should be stopped".to_string());
        }

        let tx_exists = {
            let tx_lock = self.live_update_tx.lock().await;
            tx_lock.is_some()
        };

        if tx_exists {
            return Err("Transmitter exists but should be cleared".to_string());
        }

        Ok(())
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
            .chunks(MAX_CONCURRENT_REQUESTS)
            .map(|chunk| {
                let requests = chunk.iter().map(|pv| {
                    let pv = pv.clone();
                    async move { (pv.clone(), self.fetch_metadata(&pv).await) }
                });
                join_all(requests)
            })
            .collect();

        let all_results = join_all(futures).await;
        all_results.into_iter().flatten().collect()
    }

    pub async fn cleanup_cache(&self) {
        self.data_cache.retain(|_, entry| entry.is_valid());
    }

    pub fn get_cache_stats(&self) -> (usize, usize) {
        let total = self.data_cache.len();
        let valid = self
            .data_cache
            .iter()
            .filter(|entry| entry.is_valid())
            .count();
        (valid, total)
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
            live_task: self.live_task.clone(),
            running: self.running.clone(),
            shutdown_signal: self.shutdown_signal.clone(),
            processor: self.processor.clone(),
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

    #[tokio::test]
    async fn test_cache_behavior() {
        let client = ArchiverClient::new().unwrap();

        let pv = "TEST:PV";
        let start = Utc::now().timestamp() - 3600;
        let end = Utc::now().timestamp();

        let mode = TimeRangeMode::Fixed { start, end };
        let result = client
            .fetch_historical_data(pv, &mode, OptimizationLevel::Raw, Some(1000), None)
            .await;

        assert!(result.is_ok(), "Failed to fetch data");

        let (valid_entries, total_entries) = client.get_cache_stats();
        assert!(valid_entries > 0, "Cache should contain valid entries");
        assert_eq!(
            valid_entries, total_entries,
            "All cache entries should be valid"
        );
    }
}

//! EPICS Archiver Appliance API Interface
//! Provides efficient data retrieval and processing for EPICS PVs

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
use crate::archiver::types::{DebugEvent, Meta};
use super::{DataFormat, DataOperator, NormalizedPVData, PVData, PointValue, ProcessedPoint, Statistics, TimeRange};

// Debug logging macro
macro_rules! debug_print {
    ($($arg:tt)*) => {{
        if cfg!(debug_assertions) {
            println!($($arg)*);
        }
    }};
}

#[derive(Debug, Clone)]
pub struct DataRequest {
    pub pv: String,
    pub range: TimeRange,
    pub operator: DataOperator,
    pub format: DataFormat,
    pub timezone: Option<String>,
}

// Data processor with enhanced metadata handling
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

    pub fn process_data(&self, mut data: PVData) -> Result<NormalizedPVData, String> {
        let mut points = Vec::with_capacity(data.data.len());
        let meta = data.meta.clone();
        
        // Get display limits if available
        let display_range = match meta.get_display_range() {
            Some((low, high)) => Some((low, high)),
            None => None  
         };

        for point in data.data {
            if let Some(value) = point.value_as_f64() {
                // Validate value against display range if available
                if let Some((low, high)) = display_range {
                    if value < low || value > high {
                        if let Some(ref sender) = self.debug_sender {
                            let _ = sender.send(DebugEvent {
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                level: "warn".to_string(),
                                message: format!("Value {} outside range [{}, {}] for {}", 
                                               value, low, high, meta.name),
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

#[derive(Debug, Clone)]
pub enum TimeRangeMode {
    Fixed { start: i64, end: i64 },
    Rolling { duration: Duration, end: Option<i64> },
    Live { base_mode: Box<TimeRangeMode>, last_update: i64 },
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

    pub fn is_live(&self) -> bool {
        matches!(self, Self::Live { .. })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OptimizationLevel {
    Raw,
    Optimized(i32),
    Auto,
}

impl OptimizationLevel {
    pub fn get_operator(&self, duration: i64, _chart_width: Option<i32>) -> DataOperator {
        match self {
            OptimizationLevel::Raw => DataOperator::Raw,
            OptimizationLevel::Optimized(points) => DataOperator::Mean(Some(*points)),
            OptimizationLevel::Auto => {
                match duration {
                    d if d <= 86400 => DataOperator::Raw,
                    d if d <= 604800 => DataOperator::Mean(Some(60)),
                    d if d <= 2592000 => DataOperator::Mean(Some(900)),
                    _ => DataOperator::Mean(Some(3600))
                }
            }
        }
    }
}

// ArchiverClient implementation with processor
pub struct ArchiverClient {
    client: Client,
    semaphore: Arc<Semaphore>,
    base_url: String,
    data_cache: Arc<RwLock<HashMap<String, NormalizedPVData>>>,
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
            live_task: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
            shutdown_signal: Arc::new(Notify::new()),
            processor: DataProcessor::default(),
        })
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
    where T: DeserializeOwned {
        let _permit = self.semaphore.clone().acquire_owned().await
            .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

        debug_print!("Request URL: {}", url);

        let response = self.client.get(url.clone()).send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("{}: {} ({})", ERRORS.server_error, response.status(), url.as_str()));
        }

        let text = response.text().await
            .map_err(|e| format!("Failed to get response text: {}", e))?;

        serde_json::from_str(&text)
            .map_err(|e| format!("JSON parse error: {}", e))
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

        let data = self.fetch_data_with_operator(
            pv,
            start,
            end,
            &operator,
            timezone,
        ).await?;

        let result = self.processor.process_data(data)?;
        self.data_cache.write().await.insert(pv.to_string(), result.clone());

        Ok(result)
    }

    async fn fetch_data_with_operator(
        &self,
        pv: &str,
        start: i64,
        end: i64,
        operator: &DataOperator,
        timezone: Option<&str>,
    ) -> Result<PVData, String> {
        let from_formatted = self.format_date(start * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
        let to_formatted = self.format_date(end * 1000, timezone)
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
        optimization: OptimizationLevel,
        chart_width: Option<i32>,
        timezone: Option<&str>,
    ) -> Result<Vec<NormalizedPVData>, String> {
        if pvs.is_empty() {
            return Ok(Vec::new());
        }

        let (start, end) = mode.get_range();
        let duration = end - start;
        let operator = optimization.get_operator(duration, chart_width);

        let futures: Vec<_> = pvs.iter().map(|pv| {
            self.fetch_data_with_operator(pv, start, end, &operator, timezone)
        }).collect();

        let results = join_all(futures).await;
        let mut normalized_data = Vec::with_capacity(pvs.len());

        for (result, pv) in results.into_iter().zip(pvs.iter()) {
            match result.and_then(|data| self.processor.process_data(data)) {
                Ok(processed) => {
                    self.data_cache.write().await.insert(pv.to_string(), processed.clone());
                    normalized_data.push(processed);
                }
                Err(e) => debug_print!("Failed to process data for {}: {}", pv, e),
            }
        }

        if normalized_data.is_empty() {
            return Err("No data available for any PV".to_string());
        }

        Ok(normalized_data)
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

        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(update_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            while running.load(Ordering::SeqCst) {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = interval.tick() => {
                        if tx.receiver_count() == 0 {
                            break;
                        }

                        match client.fetch_live_data(&pvs, timezone.as_deref()).await {
                            Ok(data) if !data.is_empty() => {
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
        let timestamp_formatted = self.format_date(timestamp * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

        let mut params = vec![
            ("at", timestamp_formatted.as_str()),
            ("fetchLatestMetadata", "true"),
        ];

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
                pv_data.data.first().map(|point| (
                    pv_data.meta.name.clone(),
                    PointValue {
                        secs: point.secs,
                        nanos: point.nanos,
                        val: point.val.clone(),
                        severity: point.severity,
                        status: point.status,
                    },
                ))
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

        let from_formatted = self.format_date(five_seconds_ago * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
        let to_formatted = self.format_date(now * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

        let base_params = [
            ("from", from_formatted.as_str()),
            ("to", to_formatted.as_str()),
            ("fetchLatestMetadata", "true"),
        ];

        let futures: Vec<_> = pvs.iter().map(|pv| {
            let mut url = self.build_url("getData.json", &base_params)
                .map(|mut u| {
                    u.query_pairs_mut().append_pair("pv", pv);
                    u
                });
            let pv = pv.clone();
            
            async move {
                match url {
                    Ok(u) => match self.get::<Vec<PVData>>(u).await {
                        Ok(mut data) => data.pop()
                            .and_then(|pv_data| pv_data.data.last().map(|point| {
                                (pv, PointValue {
                                    secs: point.secs,
                                    nanos: point.nanos,
                                    val: point.val.clone(),
                                    severity: point.severity,
                                    status: point.status,
                                })
                            })),
                        Err(_) => None,
                    },
                    Err(_) => None,
                }
            }
        }).collect();

        let results = join_all(futures).await;
        Ok(results.into_iter().flatten().collect())
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
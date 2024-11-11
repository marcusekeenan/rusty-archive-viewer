//! EPICS Archiver Appliance API Interface
//! Provides stateless data retrieval and processing capabilities

use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use futures::future::join_all;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::Semaphore;
use url::Url;

use crate::archiver::constants::{API_CONFIG, ERRORS};
use crate::archiver::types::*;

/// Defines how time ranges are specified for data retrieval
#[derive(Debug, Clone)]
pub enum TimeRangeMode {
    Fixed { start: i64, end: i64 },
    Rolling { duration: Duration },
}

impl TimeRangeMode {
    pub fn get_range(&self) -> (i64, i64) {
        match self {
            TimeRangeMode::Fixed { start, end } => (*start, *end),
            TimeRangeMode::Rolling { duration } => {
                let end_time = Utc::now().timestamp();
                let start_time = end_time - duration.as_secs() as i64;
                (start_time, end_time)
            }
        }
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
                let points_per_pixel =
                    chart_width.map_or(duration as f64 / 1000.0, |w| duration as f64 / w as f64);

                if points_per_pixel <= 1.0 {
                    DataOperator::Raw
                } else if duration <= 3600 {
                    DataOperator::Mean(Some(10)) // 10 second bins for <= 1 hour
                } else if duration <= 86400 {
                    DataOperator::Mean(Some(60)) // 1 minute bins for <= 1 day
                } else if duration <= 604800 {
                    DataOperator::Mean(Some(300)) // 5 minute bins for <= 1 week
                } else {
                    DataOperator::Mean(Some(900)) // 15 minute bins for > 1 week
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
    rate_limiter: Semaphore,
    base_url: String,
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

        let rate_limiter = Semaphore::new(API_CONFIG.request_limits.max_concurrent);

        Ok(Self {
            client,
            rate_limiter,
            base_url: API_CONFIG.base_url.to_string(),
        })
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
            .rate_limiter
            .acquire()
            .await
            .map_err(|e| format!("Rate limiter error: {}", e))?;

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

        response
            .json::<T>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    pub async fn fetch_data(
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

        processor.process_chunks(chunk_data)
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

    pub async fn fetch_current_values(
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
        ];

        let mut results = HashMap::new();

        for pv in pvs {
            let mut url = self.build_url("getData.json", &base_params)?;
            {
                let mut query_pairs = url.query_pairs_mut();
                query_pairs.append_pair("pv", pv);
            }

            match self.get::<Vec<PVData>>(url).await {
                Ok(mut response) => {
                    if let Some(pv_data) = response.pop() {
                        if let Some(last_point) = pv_data.data.last() {
                            if let Some(value) = last_point.value_as_f64() {
                                results.insert(
                                    pv.clone(),
                                    PointValue {
                                        secs: last_point.secs,
                                        nanos: last_point.nanos,
                                        val: Value::Single(value),
                                        severity: last_point.severity,
                                        status: last_point.status,
                                    },
                                );
                            }
                        }
                    }
                }
                Err(e) => eprintln!("Error fetching data for {}: {}", pv, e),
            }
        }

        Ok(results)
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
    async fn test_fetch_data() {
        let client = ArchiverClient::new().unwrap();
        let mode = TimeRangeMode::Fixed {
            start: Utc::now().timestamp() - 3600,
            end: Utc::now().timestamp(),
        };

        let result = client
            .fetch_data(
                "ROOM:LI30:1:OUTSIDE_TEMP",
                &mode,
                OptimizationLevel::Auto,
                Some(1000),
                Some("UTC"),
            )
            .await;

        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(!data.data.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_current_values() {
        let client = ArchiverClient::new().unwrap();
        let pvs = vec!["ROOM:LI30:1:OUTSIDE_TEMP".to_string()];

        let result = client.fetch_current_values(&pvs, Some("UTC")).await;
        assert!(result.is_ok());
        let values = result.unwrap();
        assert!(!values.is_empty());
    }

    #[tokio::test]
    async fn test_rolling_window() {
        let client = ArchiverClient::new().unwrap();
        let mode = TimeRangeMode::Rolling {
            duration: Duration::from_secs(3600),
        };

        let result = client
            .fetch_data(
                "ROOM:LI30:1:OUTSIDE_TEMP",
                &mode,
                OptimizationLevel::Auto,
                Some(1000),
                Some("UTC"),
            )
            .await;

        assert!(result.is_ok());
        let data = result.unwrap();

        // Verify data points are within the rolling window
        let now = Utc::now().timestamp() * 1000;
        let window_start = now - 3600000;
        let buffer = 10000; // 10 second buffer

        for point in data.data {
            assert!(
                point.timestamp >= (window_start - buffer) && point.timestamp <= (now + buffer),
                "Point timestamp {} outside window range {} to {}",
                point.timestamp,
                window_start - buffer,
                now + buffer
            );
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
        ];

        let futures: Vec<_> = pvs
            .iter()
            .map(|&pv| {
                let client = client.clone();
                let mode = mode.clone();
                async move {
                    client
                        .fetch_data(pv, &mode, OptimizationLevel::Auto, Some(1000), Some("UTC"))
                        .await
                }
            })
            .collect();

        let results = join_all(futures).await;
        assert!(results.iter().all(|r| r.is_ok()));
    }

    #[tokio::test]
    async fn test_optimization_levels() {
        let client = ArchiverClient::new().unwrap();
        let pv = "ROOM:LI30:1:OUTSIDE_TEMP";

        // Test different optimization levels
        for optimization in [
            OptimizationLevel::Raw,
            OptimizationLevel::Optimized(100),
            OptimizationLevel::Auto,
        ] {
            let mode = TimeRangeMode::Fixed {
                start: Utc::now().timestamp() - 3600,
                end: Utc::now().timestamp(),
            };

            let result = client
                .fetch_data(pv, &mode, optimization, Some(1000), Some("UTC"))
                .await;

            assert!(result.is_ok());
            let data = result.unwrap();

            match optimization {
                OptimizationLevel::Raw => {
                    // Raw data should have more points
                    assert!(data.data.len() > 10);
                }
                OptimizationLevel::Optimized(n) => {
                    // Should be close to the requested number of points
                    assert!(data.data.len() as i32 <= n * 2);
                }
                OptimizationLevel::Auto => {
                    // Auto should have a reasonable number of points
                    assert!(data.data.len() > 0 && data.data.len() < 10000);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_error_handling() {
        let client = ArchiverClient::new().unwrap();

        // Test invalid PV
        let mode = TimeRangeMode::Fixed {
            start: Utc::now().timestamp() - 3600,
            end: Utc::now().timestamp(),
        };

        let result = client
            .fetch_data(
                "INVALID:PV:NAME",
                &mode,
                OptimizationLevel::Auto,
                Some(1000),
                Some("UTC"),
            )
            .await;

        assert!(result.is_err());

        // Test invalid time range
        let mode = TimeRangeMode::Fixed {
            start: Utc::now().timestamp(),
            end: Utc::now().timestamp() - 3600, // End before start
        };

        let result = client
            .fetch_data(
                "ROOM:LI30:1:OUTSIDE_TEMP",
                &mode,
                OptimizationLevel::Auto,
                Some(1000),
                Some("UTC"),
            )
            .await;

        assert!(result.is_err());
    }
}

impl Clone for ArchiverClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            rate_limiter: Semaphore::new(API_CONFIG.request_limits.max_concurrent),
            base_url: self.base_url.clone(),
        }
    }
}

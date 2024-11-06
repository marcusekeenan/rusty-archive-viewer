//! API module for interacting with the EPICS Archiver Appliance
//! Handles all HTTP communication and data processing

use crate::archiver::constants::{API_CONFIG, ERRORS};
use crate::archiver::types::*;
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use futures::future::join_all;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use url::Url;

#[async_trait::async_trait]
pub trait DataFetch {
    async fn fetch_data(&self, request: &DataRequest) -> Result<NormalizedPVData, String>;
    async fn fetch_metadata(&self, pv: &str) -> Result<Meta, String>;
    async fn fetch_live_data(
        &self,
        pvs: &[String],
        timestamp: i64,
        timezone: Option<&str>, // Add timezone parameter
    ) -> Result<HashMap<String, PointValue>, String>;
}

pub trait DataProcess {
    fn process_chunks(&self, chunks: Vec<PVData>) -> Result<NormalizedPVData, String>;
    fn calculate_chunks(&self, from: i64, to: i64, width: Option<i32>) -> Vec<DataChunk>;
    fn get_optimal_operator(&self, duration: i64, width: Option<i32>) -> DataOperator;
}

#[derive(Debug, Clone)]
pub struct DataRequest {
    pub pv: String,
    pub range: TimeRange,
    pub operator: DataOperator,
    pub format: DataFormat,
    pub timezone: Option<String>, // Add timezone field
}

pub struct ArchiverClient {
    client: Client,
    semaphore: Arc<Semaphore>,
    base_url: String,
}

#[derive(Debug, Clone, Default)]
pub struct DataProcessor;

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

        Some(dt.to_rfc3339()) // Fallback to UTC if no timezone is provided
    }

    fn build_url(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Url, String> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, endpoint))
            .map_err(|e| format!("Invalid URL: {}", e))?;

        url.query_pairs_mut().extend_pairs(params);
        Ok(url)
    }

    async fn get<T: DeserializeOwned>(&self, url: Url) -> Result<T, String> {
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| format!("Failed to acquire rate limit permit: {}", e))?;

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
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
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

        println!("The start time is: {:?}", from_formatted);
        println!("THe end time is: {:?}", to_formatted);

        let pv_query = if operator.supports_binning() {
            format!("{}({})", operator.to_string(), pv)
        } else {
            pv.to_string()
        };

        // Build parameters vector
        let mut params = vec![
            ("pv", pv_query.as_str()),
            ("from", &from_formatted),
            ("to", &to_formatted),
        ];

        // Add timezone if provided
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
}

#[async_trait::async_trait]
impl DataFetch for ArchiverClient {
    async fn fetch_data(&self, request: &DataRequest) -> Result<NormalizedPVData, String> {
        let processor = DataProcessor::default();
        let chunks = processor.calculate_chunks(request.range.start, request.range.end, None);

        // Create futures for each chunk with proper error handling
        let chunk_futures: Vec<_> = chunks
            .iter()
            .map(|chunk| {
                let pv = request.pv.clone();
                let operator = request.operator.clone();
                let format = request.format.clone();
                let timezone = request.timezone.clone();
                println!("the timezoni in api is: {:?}", timezone);

                async move {
                    self.fetch_chunk_data(&pv, chunk, &operator, &format, timezone.as_deref())
                        .await
                }
            })
            .collect();

        // Execute all futures
        let results = join_all(chunk_futures).await;
        let mut chunk_data = Vec::new();

        // Process results
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

    async fn fetch_metadata(&self, pv: &str) -> Result<Meta, String> {
        let url = self.build_url("bpl/getMetadata", &[("pv", pv)])?;
        self.get(url).await
    }

    async fn fetch_live_data(
        &self,
        pvs: &[String],
        timestamp: i64,
        timezone: Option<&str>,
    ) -> Result<HashMap<String, PointValue>, String> {
        if pvs.is_empty() {
            return Ok(HashMap::new());
        }

        // Format the timestamp with timezone
        let timestamp_formatted = self
            .format_date(timestamp * 1000, timezone)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

        let mut url = self.build_url("getData.json", &[("at", &timestamp_formatted)])?;

        {
            let mut query_pairs = url.query_pairs_mut();
            for pv in pvs {
                query_pairs.append_pair("pv", pv);
            }
        }

        let response: Vec<PVData> = self.get(url).await?;
        let result = response
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
            .collect();

        Ok(result)
    }
}

impl DataProcess for DataProcessor {
    fn process_chunks(&self, chunks: Vec<PVData>) -> Result<NormalizedPVData, String> {
        if chunks.is_empty() {
            return Err(ERRORS.no_data.to_string());
        }

        let mut all_points = Vec::new();
        let meta = chunks[0].meta.clone();

        for chunk in chunks {
            for point in chunk.data {
                if let Some(value) = point.value_as_f64() {
                    all_points.push(ProcessedPoint {
                        timestamp: point.secs * 1000 + point.nanos.unwrap_or(0) / 1_000_000,
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

    fn calculate_chunks(&self, from: i64, to: i64, width: Option<i32>) -> Vec<DataChunk> {
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

    fn get_optimal_operator(&self, duration: i64, width: Option<i32>) -> DataOperator {
        let points_per_pixel = width
            .map(|w| duration / w as i64)
            .unwrap_or(duration / 1000);

        match duration {
            d if d <= 3600 || points_per_pixel <= 1 => DataOperator::Raw,
            d if d <= 86400 => DataOperator::Mean(Some(10)), // 10 second bins for <= 1 day
            d if d <= 604800 => DataOperator::Mean(Some(60)), // 1 minute bins for <= 1 week
            _ => DataOperator::Mean(Some(300)),              // 5 minute bins for > 1 week
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
    async fn test_fetch_data() {
        // Add tests...
    }

    #[tokio::test]
    async fn test_fetch_metadata() {
        // Add tests...
    }

    #[tokio::test]
    async fn test_fetch_live_data() {
        // Add tests...
    }
}

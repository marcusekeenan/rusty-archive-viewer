//! API module for interacting with the EPICS Archiver Appliance
//! Handles all HTTP communication and data processing

use crate::archiver::constants::{API_CONFIG, ERRORS};
use crate::archiver::types::*;
use chrono::{TimeZone, Utc};
use futures::future::join_all;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;
use url::Url;

/// Main client for interacting with the EPICS Archiver Appliance
pub struct ArchiverClient {
    client: Client,
    semaphore: Arc<Semaphore>,
    base_url: String,
}

impl ArchiverClient {
    /// Creates a new ArchiverClient instance
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

    /// Formats a timestamp for the archiver API in ISO 8601 format
    fn format_date(&self, timestamp_ms: i64) -> Option<String> {
        Utc.timestamp_millis_opt(timestamp_ms).single().map(|dt| {
            dt.to_rfc3339()
                .replace("+00:00", "-00:00")
                .replace("Z", "-00:00")
        })
    }

    /// Builds a URL for the API request with proper encoding
    fn build_url(&self, endpoint: &str, params: &[(&str, &str)]) -> Result<Url, String> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, endpoint))
            .map_err(|e| format!("Invalid URL: {}", e))?;

        url.query_pairs_mut().extend_pairs(params);
        Ok(url)
    }

    /// Makes a GET request to the API with rate limiting
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

    /// Fetches data for a single PV and time chunk with specified operator
    pub async fn fetch_chunk_data(
        &self,
        pv: &str,
        chunk: &DataChunk,
        operator: &DataOperator,
        format: &DataFormat,  // Changed to take a reference
    ) -> Result<PVData, String> {
        // Check inputs
        if pv.is_empty() {
            return Err("PV name cannot be empty".to_string());
        }
        if chunk.end <= chunk.start {
            return Err("Invalid time range: end must be after start".to_string());
        }

        let from_formatted = self
            .format_date(chunk.start * 1000)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
        let to_formatted = self
            .format_date(chunk.end * 1000)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

        // Apply operator if supported
        let pv_query = if operator.supports_binning() {
            format!("{}({})", operator.to_string(), pv)
        } else {
            pv.to_string()
        };

        let url = self.build_url(
            &format!("getData.{}", format.as_str()),
            &[
                ("pv", &pv_query),
                ("from", &from_formatted),
                ("to", &to_formatted),
            ],
        )?;

        let data: Vec<PVData> = self.get(url).await?;
        data.into_iter()
            .next()
            .ok_or_else(|| ERRORS.no_data.to_string())
    }

    /// Fetches metadata for a PV
    pub async fn fetch_metadata(&self, pv: &str) -> Result<Meta, String> {
        if pv.is_empty() {
            return Err("PV name cannot be empty".to_string());
        }

        let url = self.build_url("bpl/getMetadata", &[("pv", pv)])?;

        self.get(url).await
    }

    /// Gets data at a specific point in time for multiple PVs
    pub async fn get_data_at_time(
        &self,
        pvs: &[String],
        timestamp: i64,
        options: &ExtendedFetchOptions,
    ) -> Result<HashMap<String, PointValue>, String> {
        if pvs.is_empty() {
            return Err("No PVs specified".to_string());
        }
    
        let timestamp_formatted = self
            .format_date(timestamp)
            .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
    
        // Create a vector to hold our strings so they live long enough
        let mut param_strings = Vec::new();
        let mut params = Vec::new();
    
        // Add timestamp parameter
        params.push(("at", timestamp_formatted.as_str()));
    
        // Add optional parameters
        if let Some(include_proxies) = options.fetch_latest_metadata {
            param_strings.push(include_proxies.to_string());
            params.push(("includeProxies", param_strings.last().unwrap().as_str()));
        }
    
        if let Some(template) = &options.retired_pv_template {
            params.push(("retiredPVTemplate", template.as_str()));
        }
    
        let url = self.build_url("getDataAtTime", &params)?;
    
        let response = self
            .client
            .post(url)
            .json(pvs)
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
    
        if !response.status().is_success() {
            return Err(format!("{}: {}", ERRORS.server_error, response.status()));
        }
    
        response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {}", e))
    }

    /// Fetches binned data for multiple PVs with options
    pub async fn fetch_binned_data(
        &self,
        pvs: &[String],
        from: i64,
        to: i64,
        options: &ExtendedFetchOptions,
    ) -> Result<Vec<NormalizedPVData>, String> {
        if pvs.is_empty() {
            return Err("No PVs specified".to_string());
        }
        if to <= from {
            return Err("Invalid time range: end must be after start".to_string());
        }
    
        let chunks = self.calculate_chunks(from, to, options.do_not_chunk);
        let mut all_results = Vec::new();
    
        let operator = if let Some(op_str) = &options.operator {
            self.parse_operator(op_str, to - from)?
        } else {
            self.get_optimal_operator(to - from, options.chart_width)
        };
    
        let format = DataFormat::Json;
    
        for pv in pvs {
            // Clone operator and format outside the closure
            let operator = operator.clone();
            let format = format.clone();
            
            // Create futures for each chunk
            let chunk_futures = chunks.iter().map(|chunk| {
                let pv = pv.clone();
                let operator = operator.clone(); // Clone again for each chunk
                let format = format.clone();     // Clone again for each chunk
                
                async move {
                    self.fetch_chunk_data(&pv, chunk, &operator, &format).await
                }
            });
    
            // Collect and execute futures
            let results = join_all(chunk_futures).await;
            
            let mut chunk_results = Vec::new();
            for result in results {
                match result {
                    Ok(data) => chunk_results.push(data),
                    Err(e) => println!("Warning: Failed to fetch chunk for {}: {}", pv, e),
                }
            }
    
            if !chunk_results.is_empty() {
                all_results.push(self.merge_chunks(chunk_results)?);
            }
        }
    
        if all_results.is_empty() {
            Err(ERRORS.no_data.to_string())
        } else {
            Ok(all_results)
        }
    }
   
    /// Calculates time chunks for data retrieval
    pub fn calculate_chunks(&self, from: i64, to: i64, do_not_chunk: Option<bool>) -> Vec<DataChunk> {
        if do_not_chunk.unwrap_or(false) {
            return vec![DataChunk {
                start: from,
                end: to,
            }];
        }

        let mut chunks = Vec::new();
        let mut current = from;
        let duration = to - from;

        // Determine optimal chunk size based on duration
        let chunk_size = match duration {
            d if d <= 86400 => 3600,   // 1 hour chunks for <= 1 day
            d if d <= 604800 => 86400, // 1 day chunks for <= 1 week
            _ => 604800,               // 1 week chunks for > 1 week
        };

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

    /// Determines optimal operator based on time range and display width
    fn get_optimal_operator(
        &self,
        duration_seconds: i64,
        chart_width: Option<i32>,
    ) -> DataOperator {
        let points_per_pixel = if let Some(width) = chart_width {
            duration_seconds / width as i64
        } else {
            duration_seconds / 1000 // Default assumption: 1000px width
        };

        match duration_seconds {
            // Use raw data for short time spans or when enough pixels
            d if d <= 3600 || points_per_pixel <= 1 => DataOperator::Raw,

            // Use optimized binning for longer spans
            d if d <= 86400 => DataOperator::Mean(Some(60)), // 1-minute bins for <= 1 day
            d if d <= 604800 => DataOperator::Mean(Some(300)), // 5-minute bins for <= 1 week
            d if d <= 2592000 => DataOperator::Mean(Some(900)), // 15-minute bins for <= 1 month
            _ => DataOperator::Mean(Some(3600)),             // 1-hour bins for > 1 month
        }
    }

    /// Parses operator string into DataOperator enum with support for all operators
    fn parse_operator(&self, operator: &str, duration: i64) -> Result<DataOperator, String> {
        let parts: Vec<&str> = operator.split('_').collect();
        let base_op = parts[0];
        let bin_size = parts.get(1).and_then(|s| s.parse::<i32>().ok());

        // Parse any additional parameters
        let extra_param = parts.get(2).and_then(|s| s.parse::<f64>().ok());

        match base_op {
            // Basic operators
            "raw" => Ok(DataOperator::Raw),
            "mean" => Ok(DataOperator::Mean(bin_size)),
            "firstSample" => Ok(DataOperator::FirstSample(bin_size)),
            "lastSample" => Ok(DataOperator::LastSample(bin_size)),

            // Fill operators
            "firstFill" => Ok(DataOperator::FirstFill(bin_size)),
            "lastFill" => Ok(DataOperator::LastFill(bin_size)),

            // Statistical operators
            "min" => Ok(DataOperator::Min(bin_size)),
            "max" => Ok(DataOperator::Max(bin_size)),
            "count" => Ok(DataOperator::Count(bin_size)),
            "median" => Ok(DataOperator::Median(bin_size)),
            "std" => Ok(DataOperator::Std(bin_size)),
            "variance" => Ok(DataOperator::Variance(bin_size)),
            "popvariance" => Ok(DataOperator::PopVariance(bin_size)),
            "jitter" => Ok(DataOperator::Jitter(bin_size)),
            "kurtosis" => Ok(DataOperator::Kurtosis(bin_size)),
            "skewness" => Ok(DataOperator::Skewness(bin_size)),

            // Special operators
            "nth" => {
                let n = bin_size.ok_or("nth operator requires a numeric parameter")?;
                Ok(DataOperator::Nth(n))
            }

            // Flyer detection operators
            "ignoreflyers" => {
                let deviations = extra_param.unwrap_or(3.0);
                Ok(DataOperator::IgnoreFlyers {
                    bin_size,
                    deviations,
                })
            }
            "flyers" => {
                let deviations = extra_param.unwrap_or(3.0);
                Ok(DataOperator::Flyers {
                    bin_size,
                    deviations,
                })
            }

            _ => Err(format!("Unknown operator: {}", operator)),
        }
    }

    /// Merges multiple PVData chunks into a single NormalizedPVData
    fn merge_chunks(&self, chunks: Vec<PVData>) -> Result<NormalizedPVData, String> {
        if chunks.is_empty() {
            return Err(ERRORS.no_data.to_string());
        }

        let mut all_points = Vec::new();
        let meta = chunks[0].meta.clone();

        // Process and sort all points from all chunks
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

        // Sort points by timestamp and remove duplicates
        all_points.sort_by_key(|p| p.timestamp);
        all_points.dedup_by_key(|p| p.timestamp);

        // Calculate statistics if we have points
        let statistics = if !all_points.is_empty() {
            let values: Vec<f64> = all_points.iter().map(|p| p.value).collect();
            let n = values.len() as f64;
            let mean = values.iter().sum::<f64>() / n;

            // Calculate variance and other statistics
            let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;

            Some(Statistics {
                mean,
                std_dev: variance.sqrt(),
                min: values.iter().copied().fold(f64::INFINITY, f64::min),
                max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                count: values.len() as i64,
                first_timestamp: all_points.first().map(|p| p.timestamp).unwrap_or(0),
                last_timestamp: all_points.last().map(|p| p.timestamp).unwrap_or(0),
            })
        } else {
            None
        };

        Ok(NormalizedPVData {
            meta,
            data: all_points,
            statistics,
        })
    }

    /// Validates a PV name
    fn validate_pv_name(&self, pv: &str) -> Result<(), String> {
        if pv.is_empty() {
            return Err("PV name cannot be empty".to_string());
        }
        if pv.contains(char::is_whitespace) {
            return Err("PV name cannot contain whitespace".to_string());
        }
        Ok(())
    }

    /// Validates a time range
    fn validate_time_range(&self, start: i64, end: i64) -> Result<(), String> {
        if end <= start {
            return Err("End time must be after start time".to_string());
        }
        if end - start > 31536000 {
            // 1 year in seconds
            return Err("Time range cannot exceed 1 year".to_string());
        }
        Ok(())
    }
}

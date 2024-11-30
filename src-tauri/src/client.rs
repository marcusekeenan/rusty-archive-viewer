use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Utc};
use futures::future::join_all;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use tokio::task;

use crate::decode::decode_response;
use crate::decode_helpers::format_date_for_archiver;
use crate::types::{Config, Error, Meta, PVData};
use reqwest::Client;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UPlotData {
    pub timestamps: Vec<f64>,  // milliseconds since epoch
    pub series: Vec<Vec<f64>>, // array of value arrays, one per PV
    pub meta: Vec<Meta>,       // metadata for each series
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingMode {
    Raw,
    Optimized(usize), // number of points
    Binning {
        bin_size: u32,
        operation: BinningOperation,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinningOperation {
    Mean,
    Max,
    Min,
    Jitter,
    StdDev,
    Count,
    FirstSample,
    LastSample,
    FirstFill,
    LastFill,
    Median,
    Variance,
    PopVariance,
    Kurtosis,
    Skewness,
    Linear,
    Loess,
    CAPlotBinning,
}

impl ProcessingMode {
    pub fn determine_optimal(start: i64, end: i64) -> Self {
        let duration = end - start;

        match duration {
            // For intervals less than 5 minutes, use raw data
            d if d < 3600 => ProcessingMode::Raw,

            // For 1 hour to 6 hours
            d if d < 21600 => ProcessingMode::Optimized(1000),

            // For 6 hours to 24 hours
            d if d < 86400 => ProcessingMode::Optimized(1500),

            // For 1 day to 7 days
            d if d < 604800 => ProcessingMode::Optimized(2000),

            // For 1 week to 1 month
            d if d < 2592000 => ProcessingMode::Optimized(3000),

            // For anything longer
            _ => ProcessingMode::Optimized(4000),
        }
    }
}

impl fmt::Display for BinningOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinningOperation::Mean => write!(f, "mean"),
            BinningOperation::Max => write!(f, "max"),
            BinningOperation::Min => write!(f, "min"),
            BinningOperation::Jitter => write!(f, "jitter"),
            BinningOperation::StdDev => write!(f, "std"),
            BinningOperation::Count => write!(f, "count"),
            BinningOperation::FirstSample => write!(f, "firstSample"),
            BinningOperation::LastSample => write!(f, "lastSample"),
            BinningOperation::FirstFill => write!(f, "firstFill"),
            BinningOperation::LastFill => write!(f, "lastFill"),
            BinningOperation::Median => write!(f, "median"),
            BinningOperation::Variance => write!(f, "variance"),
            BinningOperation::PopVariance => write!(f, "popvariance"),
            BinningOperation::Kurtosis => write!(f, "kurtosis"),
            BinningOperation::Skewness => write!(f, "skewness"),
            BinningOperation::Linear => write!(f, "linear"),
            BinningOperation::Loess => write!(f, "loess"),
            BinningOperation::CAPlotBinning => write!(f, "caplotbinning"),
        }
    }
}

impl fmt::Display for ProcessingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessingMode::Raw => write!(f, ""),
            ProcessingMode::Optimized(points) => write!(f, "optimized({})", points),
            ProcessingMode::Binning {
                bin_size,
                operation,
            } => match operation {
                BinningOperation::CAPlotBinning => write!(f, "caplot"),
                _ => write!(f, "{}", operation.to_string().to_lowercase()),
            },
        }
    }
}

#[derive(Clone)]
pub struct ArchiverClient {
    client: Client,
    base_url: String,
}

impl ArchiverClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: config.url,
        }
    }

    fn convert_to_uplot(pv_data: Vec<PVData>) -> UPlotData {
        println!("Number of PVs: {}", pv_data.len());
    
        if let Some(first_pv) = pv_data.first() {
            first_pv
                .data
                .iter()
                .take(5)
                .enumerate()
                .for_each(|(i, point)| {
                    println!(
                        "Raw data point {}: secs = {}, nanos = {}",
                        i, point.secs, point.nanos
                    );
                });
        }
    
        let mut timestamp_value_pairs: Vec<(f64, usize, f64)> = pv_data
            .iter()
            .enumerate()
            .flat_map(|(series_idx, pv)| {
                pv.data
                    .iter()
                    .filter_map(move |point| {
                        let unix_ms = point.secs as i64 + (point.nanos / 1_000_000) as i64;
    
                        if NaiveDateTime::from_timestamp_millis(unix_ms).is_none() {
                            println!(
                                "Failed to convert timestamp: secs = {}, nanos = {}",
                                point.secs, point.nanos
                            );
                        }
    
                        match &point.val {
                            Value::Number(n) => n.as_f64(),
                            Value::Object(obj) => obj
                                .get("mean")
                                .and_then(|v| v.as_f64())
                                .or_else(|| obj.get("value").and_then(|v| v.as_f64())),
                            Value::Array(arr) if !arr.is_empty() => arr[0].as_f64(),
                            _ => None,
                        }
                        .map(|val| (unix_ms as f64, series_idx, val))
                    })
            })
            .collect();
    
        // Sort data by timestamp
        timestamp_value_pairs.sort_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
        });
    
        let timestamps: Vec<f64> = timestamp_value_pairs.iter().map(|(ts, _, _)| *ts).collect();
    
        // Create a 2D array for the series
        let mut series: Vec<Vec<f64>> = vec![vec![f64::NAN; timestamps.len()]; pv_data.len()];
    
        // Fill the series data
        for (ts, series_idx, val) in timestamp_value_pairs {
            if let Ok(pos) = timestamps.binary_search_by(|probe| {
                probe.partial_cmp(&ts).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                series[series_idx][pos] = val;
            }
        }
    
        // Return the final UPlotData structure
        UPlotData {
            timestamps,
            series,
            meta: pv_data.into_iter().map(|pv| pv.meta).collect(),
        }
    }
    
    
    pub async fn fetch_data_uplot(
        &self,
        pvs: Vec<String>,
        start: i64,
        end: i64,
        mode: Option<ProcessingMode>,
    ) -> Result<UPlotData, Error> {
        // If mode is not specified, determine optimal mode based on time range
        let mode = mode.unwrap_or_else(|| ProcessingMode::determine_optimal(start, end));

        let pv_data = self.fetch_historical_data(pvs, start, end, mode).await?;

        let uplot_data = task::spawn_blocking(move || Self::convert_to_uplot(pv_data))
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?;

        Ok(uplot_data)
    }

    pub async fn fetch_historical_data(
        &self,
        pvs: Vec<String>,
        start: i64,
        end: i64,
        mode: ProcessingMode, // Removed &
    ) -> Result<Vec<PVData>, Error> {
        let fetch_tasks: Vec<_> = pvs
            .into_iter()
            .map(|pv| {
                let client = self.clone();
                let mode = mode.clone();

                task::spawn(async move {
                    client
                        .fetch_data_with_processing(&pv, start, end, mode) // Removed &
                        .await
                })
            })
            .collect();

        let results = join_all(fetch_tasks).await;

        let mut pv_data = Vec::new();
        for result in results {
            match result {
                Ok(Ok(data)) => pv_data.push(data),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(Error::Invalid(e.to_string())),
            }
        }

        Ok(pv_data)
    }

    pub async fn fetch_data_with_processing(
        &self,
        pv: &str,
        start: i64,
        end: i64,
        mode: ProcessingMode,
    ) -> Result<PVData, Error> {
        // For raw data, just use the PV name directly
        if matches!(mode, ProcessingMode::Raw) {
            return self.fetch_raw_data(pv, start, end).await;
        }

        // Build the operator string
        let processed_pv = match &mode {
            ProcessingMode::Raw => unreachable!(),
            ProcessingMode::Optimized(points) => format!("optimized_{}({})", points, pv),
            ProcessingMode::Binning {
                bin_size,
                operation,
            } => match operation {
                BinningOperation::CAPlotBinning => format!("caplotbinning({})", pv),
                _ => format!(
                    "{}_{}({})",
                    operation.to_string().to_lowercase(),
                    bin_size,
                    pv
                ),
            },
        };

        let start_date = format_date_for_archiver(start * 1000)
            .ok_or_else(|| Error::Invalid("Invalid start timestamp".to_string()))?;
        let end_date = format_date_for_archiver(end * 1000)
            .ok_or_else(|| Error::Invalid("Invalid end timestamp".to_string()))?;

        let url = self.build_url(
            "data/getData.raw",
            &[
                ("pv", &processed_pv),
                ("from", &start_date),
                ("to", &end_date),
                ("fetchLatestMetadata", "true"),
            ],
        )?;

        println!("Requesting URL: {}", url);

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(Error::Network)?;

        if !response.status().is_success() {
            return Err(Error::Invalid(format!(
                "Server returned {} for {}",
                response.status(),
                url
            )));
        }

        let bytes = response.bytes().await.map_err(Error::Network)?;
        let pv_data = decode_response(bytes)?;

        pv_data
            .into_iter()
            .next()
            .ok_or_else(|| Error::Invalid("No data returned".to_string()))
    }

    async fn fetch_raw_data(&self, pv: &str, start: i64, end: i64) -> Result<PVData, Error> {
        let start_date = format_date_for_archiver(start * 1000)
            .ok_or_else(|| Error::Invalid("Invalid start timestamp".to_string()))?;
        let end_date = format_date_for_archiver(end * 1000)
            .ok_or_else(|| Error::Invalid("Invalid end timestamp".to_string()))?;

        let url = self.build_url(
            "data/getData.raw",
            &[
                ("pv", pv),
                ("from", &start_date),
                ("to", &end_date),
                ("fetchLatestMetadata", "true"),
            ],
        )?;

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(Error::Network)?;

        if !response.status().is_success() {
            return Err(Error::Invalid(format!(
                "Server returned {} for {}",
                response.status(),
                url
            )));
        }

        let bytes = response.bytes().await.map_err(Error::Network)?;
        let pv_data = decode_response(bytes)?;

        pv_data
            .into_iter()
            .next()
            .ok_or_else(|| Error::Invalid("No data returned".to_string()))
    }

    fn build_url(&self, path: &str, params: &[(&str, &str)]) -> Result<Url, Error> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, path))
            .map_err(|e| Error::Invalid(format!("Invalid URL: {}", e)))?;
        url.query_pairs_mut().extend_pairs(params);
        Ok(url)
    }

    pub async fn get_metadata(&self, pv: &str) -> Result<Meta, Error> {
        let url = self.build_url("bpl/getMetadata", &[("pv", pv)])?;

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(Error::Network)?;

        if !response.status().is_success() {
            return Err(Error::Invalid(format!(
                "Server returned {} for {}",
                response.status(),
                url
            )));
        }

        let meta_data: HashMap<String, String> = response.json().await.map_err(Error::Network)?;
        Ok(Meta(meta_data))
    }
}

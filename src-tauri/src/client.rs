use chrono::Utc;
use futures::future::join_all;
use tokio::task;
use reqwest::{Client, Response};
use url::Url;

use crate::decode::DecoderContext;  use crate::decode_helpers::format_date_for_archiver;
// Update this import
use crate::types::{BinningOperation, Config, DataFormat, Error, Meta, PVData, Point, ProcessingMode, UPlotData};

const ESTIMATED_POINTS_CAPACITY: usize = 100;
#[derive(Clone)]
pub struct ArchiverClient {
    client: Client,
    base_url: String,
    decoder_context: std::sync::Arc<parking_lot::Mutex<DecoderContext>>, 
}

impl ArchiverClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(config.timeout_secs))
                .build()
                .expect("Failed to create HTTP client"),
            base_url: config.url,
            decoder_context: std::sync::Arc::new(parking_lot::Mutex::new(
                DecoderContext::new(ESTIMATED_POINTS_CAPACITY)
            )),
        }
    }

    fn convert_to_uplot(pv_data: Vec<PVData>) -> UPlotData {
        let mut timestamp_value_pairs: Vec<(f64, usize, f64)> = pv_data
            .iter()
            .enumerate()
            .flat_map(|(series_idx, pv)| {
                pv.data
                    .iter()
                    .filter_map(move |point| {
                        let unix_ms = point.secs * 1000 + (point.nanos / 1_000_000) as i64;
                        match &point.val {
                            serde_json::Value::Number(n) => n.as_f64(),
                            serde_json::Value::Object(obj) => obj
                                .get("mean")
                                .and_then(|v| v.as_f64())
                                .or_else(|| obj.get("value").and_then(|v| v.as_f64())),
                            serde_json::Value::Array(arr) if !arr.is_empty() => arr[0].as_f64(),
                            _ => None,
                        }
                        .map(|val| (unix_ms as f64, series_idx, val))
                    })
            })
            .collect();

        timestamp_value_pairs.sort_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
        });

        let timestamps: Vec<f64> = timestamp_value_pairs.iter().map(|(ts, _, _)| *ts).collect();
        let mut series: Vec<Vec<f64>> = vec![vec![f64::NAN; timestamps.len()]; pv_data.len()];

        for (ts, series_idx, val) in timestamp_value_pairs {
            if let Ok(pos) = timestamps.binary_search_by(|probe| {
                probe.partial_cmp(&ts).unwrap_or(std::cmp::Ordering::Equal)
            }) {
                series[series_idx][pos] = val;
            }
        }

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
        format: DataFormat,
    ) -> Result<(UPlotData, usize), Error> {
        let mode = mode.unwrap_or_else(|| ProcessingMode::determine_optimal(start, end));
        let (pv_data, total_size) = self.fetch_historical_data(pvs, start, end, mode, format).await?;

        let uplot_data = task::spawn_blocking(move || Self::convert_to_uplot(pv_data))
            .await
            .map_err(|e| Error::Invalid(e.to_string()))?;

        Ok((uplot_data, total_size))
    }

    pub async fn fetch_historical_data(
        &self,
        pvs: Vec<String>,
        start: i64,
        end: i64,
        mode: ProcessingMode,
        format: DataFormat,
    ) -> Result<(Vec<PVData>, usize), Error> {
        let fetch_tasks: Vec<_> = pvs
            .into_iter()
            .map(|pv| {
                let client = self.clone();
                let mode = mode.clone();
                task::spawn(async move {
                    client.fetch_data_with_processing(&pv, start, end, mode, format).await
                })
            })
            .collect();

        let results = join_all(fetch_tasks).await;
        let mut pv_data = Vec::new();
        let mut total_size = 0;
        
        for result in results {
            match result {
                Ok(Ok((data, size))) => {
                    pv_data.push(data);
                    total_size += size;
                }
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(Error::Invalid(e.to_string())),
            }
        }

        Ok((pv_data, total_size))
    }

    pub async fn fetch_data_with_processing(
        &self,
        pv: &str,
        start: i64,
        end: i64,
        mode: ProcessingMode,
        format: DataFormat,
    ) -> Result<(PVData, usize), Error> {
        match mode {
            ProcessingMode::Raw => self.fetch_data(pv, start, end, format).await,
            _ => {
                let processed_pv = match &mode {
                    ProcessingMode::Raw => unreachable!(),
                    ProcessingMode::Optimized(points) => {
                        format!("optimized_{}({})", points, pv)
                    }
                    ProcessingMode::Binning { bin_size, operation } => match operation {
                        BinningOperation::CAPlotBinning => format!("caplotbinning({})", pv),
                        _ => format!(
                            "{}_{}({})",
                            operation.to_string().to_lowercase(),
                            bin_size,
                            pv
                        ),
                    },
                };
                self.fetch_data(&processed_pv, start, end, format).await
            }
        }
    }

    async fn fetch_data(
        &self,
        pv: &str,
        start: i64,
        end: i64,
        format: DataFormat,
    ) -> Result<(PVData, usize), Error> {
        let start_date = format_date_for_archiver(start * 1000)
            .ok_or_else(|| Error::Invalid("Invalid start timestamp".to_string()))?;
        let end_date = format_date_for_archiver(end * 1000)
            .ok_or_else(|| Error::Invalid("Invalid end timestamp".to_string()))?;

        let ext = match format {
            DataFormat::Raw => "raw",
            DataFormat::Json => "json",
        };

        let url = self.build_url(
            &format!("data/getData.{}", ext),
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

        let content_length = get_content_length(&response);
        let bytes = response.bytes().await.map_err(Error::Network)?;
        let actual_size = bytes.len();

        let pv_data = match format {
            DataFormat::Raw => {
                let mut context = self.decoder_context.lock();
                context.decode_response(bytes)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::Invalid("No data returned".to_string()))?
            }
            DataFormat::Json => {
                let mut pv_data: Vec<PVData> = serde_json::from_slice(&bytes)
                    .map_err(|e| Error::Invalid(format!("Failed to parse JSON: {}", e)))?;
                
                if let Some(data) = pv_data.get_mut(0) {
                    for point in &mut data.data {
                        if point.nanos >= 1_000_000_000 {
                            point.secs += point.nanos as i64 / 1_000_000_000;
                            point.nanos %= 1_000_000_000;
                        }
                    }
                }

                pv_data
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::Invalid("No data returned".to_string()))?
            }
        };

        Ok((pv_data, content_length.unwrap_or(actual_size)))
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

        let meta_data = response.json().await.map_err(Error::Network)?;
        Ok(Meta(meta_data))
    }
}

fn get_content_length(response: &Response) -> Option<usize> {
    response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|cl| cl.to_str().ok())
        .and_then(|cl| cl.parse::<usize>().ok())
}
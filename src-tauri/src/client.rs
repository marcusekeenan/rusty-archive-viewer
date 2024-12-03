use crate::decode::DecoderContext;
use crate::decode_helpers::format_date_for_archiver;
use crate::types::{
    BinningOperation, Config, DataFormat, Error, Meta, PVData, PVDataJson, PointValue,
    ProcessingMode, UPlotData,
};
use crate::Point;
use futures::future::join_all;
use regex::Regex;
use reqwest::{Client, Response};
use tokio::task;
use url::Url;

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

    pub async fn fetch_data_uplot(
        &self,
        pvs: Vec<String>,
        start: i64,
        end: i64,
        mode: Option<ProcessingMode>,
        format: DataFormat,
    ) -> Result<(UPlotData, usize), Error> {
        let mode = mode.unwrap_or_else(|| ProcessingMode::determine_optimal(start, end));
        let (pv_data, total_size) = self
            .fetch_historical_data(pvs, start, end, mode, format)
            .await?;

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
                    client
                        .fetch_data_with_processing(&pv, start, end, mode, format)
                        .await
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
                self.fetch_data(&processed_pv, start, end, format).await
            }
        }
    }

    pub async fn fetch_data(
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

        let endpoint = match format {
            DataFormat::Raw => "data/getData.raw",
            DataFormat::Json => "data/getData.json",
        };

        let url = self.build_url(
            endpoint,
            &[
                ("pv", pv),
                ("from", &start_date),
                ("to", &end_date),
                ("fetchLatestMetadata", "true"),
            ],
        )?;

        //  println!("Requesting URL: {}", url);

        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .map_err(Error::Network)?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            return Err(Error::Invalid(format!(
                "Server returned {} for {}. Error: {}",
                status, url, error_body
            )));
        }

        let content_length = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|cl| cl.to_str().ok())
            .and_then(|cl| cl.parse::<usize>().ok())
            .unwrap_or(0);

        match format {
            DataFormat::Raw => {
                let bytes = response.bytes().await.map_err(Error::Network)?;
                
                let duration_seconds = end - start;
                let content_size = bytes.len();
                
                // println!("\nProcessing request:");
                // println!("Duration: {}s, Size: {} bytes", duration_seconds, content_size);
            
                let mut decoder_context = match duration_seconds {
                    d if d <= 5 => {
                        // Live data: ~6 points per 5 seconds
                        let estimated_points = d;
                        let batch_size = 10;  // Small batch since we only get ~6 points
                        let ctx = DecoderContext::new(estimated_points.try_into().unwrap());
                        // println!("Live data: using batch size: {}", batch_size);
                        ctx
                    },
                    d if d <= 60 => {
                        // 1 minute: ~58-59 points
                        let estimated_points = d;
                        let batch_size = 30;  // Half of expected points
                        let ctx = DecoderContext::new(estimated_points.try_into().unwrap());
                        // println!("1-min data: using batch size: {}", batch_size);
                        ctx
                    },
                    d if d <= 300 => {
                        // 5 minutes: ~295-296 points
                        let estimated_points = d;
                        let batch_size = 100;  // ~1/3 of expected points
                        let ctx = DecoderContext::new(estimated_points.try_into().unwrap());
                        // println!("5-min data: using batch size: {}", batch_size);
                        ctx
                    },
                    d => {
                        // 1 hour+: ~3500+ points
                        let estimated_points = d;
                        let batch_size = 500;  // Larger batch for better parallel processing
                        let ctx = DecoderContext::new(estimated_points.try_into().unwrap());
                        // println!("Long-term data: using batch size: {}", batch_size);
                        ctx
                    }
                };
            
                let pv_data = decoder_context.decode_response(&bytes)?;
                if let Some(first_pv) = &pv_data.first() {
                    // println!("Actual points decoded: {}", first_pv.data.len());
                }
            
                Ok((
                    pv_data
                        .into_iter()
                        .next()
                        .ok_or_else(|| Error::Invalid("No data returned".to_string()))?,
                    content_size,
                ))
            }
            DataFormat::Json => {
                // Read the response body as text
                let text = response.text().await.map_err(Error::Network)?;
                //  println!("Raw JSON response:\n{}", text);

                // Attempt to parse the JSON response
                let pv_data_json: Vec<PVDataJson> = match serde_json::from_str(&text) {
                    Ok(data) => data,
                    Err(e) => {
                        println!("[ERROR] JSON parsing error: {}", e);

                        // Extract error line and column if available
                        let line = e.line();
                        let column = e.column();

                        if line > 0 {
                            println!("[ERROR] Error occurred at line {}, column {}", line, column);
                            let lines: Vec<&str> = text.lines().collect();
                            if line <= lines.len() {
                                println!("[ERROR] Problematic line: {}", lines[line - 1]);
                                println!("[ERROR] {}^", " ".repeat(column.saturating_sub(1)));
                            }
                        } else {
                            println!(
                                "[ERROR] Could not determine the line or column of the error."
                            );
                        }

                        return Err(Error::Invalid(format!(
                            "Failed to parse JSON: {}. JSON snippet:\n{}",
                            e,
                            &text[..std::cmp::min(500, text.len())]
                        )));
                    }
                };

                // println!(
                //     "[DEBUG] Successfully parsed JSON data for {} PVs",
                //     pv_data_json.len()
                // );

                // Ensure we have at least one PVDataJson in the parsed JSON
                if pv_data_json.is_empty() {
                    return Err(Error::Invalid(
                        "No data returned in the JSON response".to_string(),
                    ));
                }

                // Convert PVDataJson into PVData
                let pv_data: Vec<PVData> = pv_data_json
                    .into_iter()
                    .map(|json| PVData {
                        meta: json.meta,
                        data: json
                            .data
                            .into_iter()
                            .map(|point_json| Point {
                                secs: point_json.secs,
                                nanos: point_json.nanos,
                                val: PointValue::from(point_json.val), // Convert serde_json::Value to PointValue
                                severity: point_json.severity.unwrap_or(0) as i32, // Provide default value
                                status: point_json.status.unwrap_or(0) as i32, // Provide default value
                            })
                            .collect(),
                    })
                    .collect();

                // Debugging output for the parsed data
                // println!(
                //     "[DEBUG] Successfully converted JSON data to {} PV(s)",
                //     pv_data.len()
                // );
                if let Some(first_pv) = pv_data.first() {
                    // println!("[DEBUG] First PV name: {}", first_pv.meta.name);
                    // println!("[DEBUG] Number of data points: {}", first_pv.data.len());
                    // if let Some(first_point) = first_pv.data.first() {
                    //     println!("[DEBUG] First data point: {:?}", first_point);
                    // }
                }

                // Return the first PVData and the content length (if available)
                Ok((pv_data.into_iter().next().unwrap(), content_length))
            }
        }
    }

    fn preprocess_json(&self, json: &str) -> String {
        let remove_spaces = Regex::new(r"\s*:\s*").unwrap();
        let remove_trailing_commas = Regex::new(r",\s*([}\]])").unwrap();

        let json = json.trim(); // This will remove leading and trailing whitespace
        let json = remove_spaces.replace_all(json, ":").to_string();
        let json = remove_trailing_commas.replace_all(&json, "$1").to_string();

        json.replace("\n", "\\n")
    }

    fn build_url(&self, path: &str, params: &[(&str, &str)]) -> Result<Url, Error> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, path))
            .map_err(|e| Error::Invalid(format!("Invalid URL: {}", e)))?;
        url.query_pairs_mut().extend_pairs(params);
        Ok(url)
    }

    pub async fn get_metadata(&self, pv: &str) -> Result<Meta, Error> {
        // Build the URL
        let url = self.build_url("bpl/getMetadata", &[("pv", pv)])?;
        //  println!("[DEBUG] Built URL: {}", url);

        // Send the request
        let response = match self.client.get(url.clone()).send().await {
            Ok(resp) => {
                //  println!("[DEBUG] Received response with status: {}", resp.status());
                resp
            }
            Err(err) => {
                println!("[ERROR] Failed to send request: {}", err);
                return Err(Error::Network(err));
            }
        };

        // Get the response status
        let status = response.status();
        //  println!("[DEBUG] Response status: {}", status);

        // Read the response body
        let bytes = match response.bytes().await {
            Ok(b) => {
                //  println!("[DEBUG] Successfully read response bytes");
                b
            }
            Err(err) => {
                println!("[ERROR] Failed to read response bytes: {}", err);
                return Err(Error::Network(err));
            }
        };

        // Check if the status is not successful
        if !status.is_success() {
            let response_text = String::from_utf8_lossy(&bytes);
            println!(
                "[ERROR] Server returned error status {} for URL {}\nResponse: {}",
                status, url, response_text
            );
            return Err(Error::Invalid(format!(
                "Server returned {} for {}\nResponse: {}",
                status, url, response_text
            )));
        }

        // Parse the metadata
        let meta_data: Meta = match serde_json::from_slice(&bytes) {
            Ok(data) => {
                //  println!("[DEBUG] Successfully parsed metadata JSON");
                data
            }
            Err(err) => {
                let snippet = String::from_utf8_lossy(&bytes[..bytes.len().min(200)]);
                println!(
                    "[ERROR] Failed to parse metadata JSON: {}\nResponse snippet: {}",
                    err, snippet
                );
                return Err(Error::Invalid(format!(
                    "Failed to parse metadata JSON: {}\nResponse snippet: {}",
                    err, snippet
                )));
            }
        };

        // Return the parsed metadata
        //  println!("[DEBUG] Metadata fetched successfully for PV: {}", pv);
        Ok(meta_data)
    }

    fn convert_to_uplot(pv_data: Vec<PVData>) -> UPlotData {
        use std::collections::{BTreeSet, HashMap};

        // Collect all unique timestamps
        let mut timestamps_set = BTreeSet::new();
        let mut series_data: Vec<HashMap<i64, f64>> = vec![];

        for pv in &pv_data {
            let mut series_map = HashMap::new();
            for point in &pv.data {
                let ts = point.secs * 1000 + (point.nanos as i64 / 1_000_000);
                if let Some(val) = match &point.val {
                    PointValue::Float(v) => Some(*v as f64),
                    PointValue::Double(v) => Some(*v),
                    PointValue::Int(v) => Some(*v as f64),
                    PointValue::Long(v) => Some(*v as f64),
                    PointValue::Short(v) => Some(*v as f64),
                    PointValue::Byte(v) => Some(*v as f64),
                    PointValue::Enum(v) => Some(*v as f64),
                    _ => None,
                } {
                    timestamps_set.insert(ts);
                    series_map.insert(ts, val);
                }
            }
            series_data.push(series_map);
        }

        let timestamps: Vec<i64> = timestamps_set.into_iter().collect();
        let mut series = Vec::with_capacity(series_data.len());

        for series_map in series_data {
            let mut data = Vec::with_capacity(timestamps.len());
            for &ts in &timestamps {
                data.push(*series_map.get(&ts).unwrap_or(&f64::NAN));
            }
            series.push(data);
        }

        // Convert timestamps to f64 for plotting
        let timestamps_f64: Vec<f64> = timestamps.iter().map(|&ts| ts as f64).collect();

        UPlotData {
            timestamps: timestamps_f64,
            series,
            meta: pv_data.iter().map(|pv| pv.meta.clone()).collect(),
        }
    }
}

fn get_content_length(response: &Response) -> Option<usize> {
    response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|cl| cl.to_str().ok())
        .and_then(|cl| cl.parse::<usize>().ok())
}

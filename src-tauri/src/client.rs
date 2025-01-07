use crate::decode::DecoderContext;
use crate::decode_helpers::format_date_for_archiver;
use crate::types::{
    DataFormat, Error, Meta, PVData, PVDataJson, Point, PointValue, ProcessingMode, UPlotData,
};
use crate::Config;
use futures::future::join_all;
use reqwest::Client;
use tokio::task;
use url::Url;

#[derive(Clone)]
pub struct ArchiverClient {
    pub client: Client,
    pub base_url: String,
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

    pub async fn fetch_data(
        &self,
        pvs: Vec<String>,
        start: i64,
        end: i64,
        mode: Option<ProcessingMode>,
        format: DataFormat,
    ) -> Result<UPlotData, Error> {
        println!("Starting fetch_data with {} PVs", pvs.len());
        let processing_mode = mode.unwrap_or(ProcessingMode::Raw);
        println!("Using processing mode: {:?}", processing_mode);
    
        let fetch_tasks: Vec<_> = pvs
            .into_iter()
            .map(|pv| {
                let client = self.clone();
                let mode = processing_mode.clone();
                task::spawn(
                    async move { 
                        println!("Fetching data for PV: {}", pv);
                        let result = client.fetch_single_pv(&pv, start, end, mode, format).await;
                        println!("Fetch result for {}: {:?}", pv, result.is_ok());
                        result
                    },
                )
            })
            .collect();
    
        println!("Created {} fetch tasks", fetch_tasks.len());
        let results = join_all(fetch_tasks).await;
        println!("Completed all fetch tasks");
    
        let mut pv_data = Vec::new();
    
        for result in results {
            match result {
                Ok(Ok(data)) => {
                    println!("Successfully processed PV data with {} points", data.data.len());
                    pv_data.push(data)
                },
                Ok(Err(e)) => {
                    println!("Error in PV data: {:?}", e);
                    return Err(e)
                },
                Err(e) => {
                    println!("Task error: {:?}", e);
                    return Err(Error::Invalid(e.to_string()))
                },
            }
        }
    
        println!("Converting {} PV datasets to uplot format", pv_data.len());
        let uplot_data = task::spawn_blocking(move || {
            let result = Self::convert_to_uplot(pv_data);
            println!("Conversion complete: {} timestamps, {} series", 
                result.timestamps.len(), 
                result.series.len()
            );
            result
        })
        .await
        .map_err(|e| {
            println!("Error in conversion task: {:?}", e);
            Error::Invalid(e.to_string())
        })?;
    
        println!("Returning uplot data");
        Ok(uplot_data)
    }

    async fn fetch_single_pv(
        &self,
        pv: &str,
        start: i64,
        end: i64,
        mode: ProcessingMode,
        format: DataFormat,
    ) -> Result<PVData, Error> {
        // Convert OptimizedAuto to concrete optimization if needed
        let start_date = format_date_for_archiver(start * 1000)
        .ok_or_else(|| Error::Invalid("Invalid start timestamp".to_string()))?;
    let end_date = format_date_for_archiver(end * 1000)
        .ok_or_else(|| Error::Invalid("Invalid end timestamp".to_string()))?;

    // Format the PV with the operator (e.g., mean or optimized)
    let processed_pv = mode.format_pv(pv);

    // Build the parameter list
    let params = vec![
        ("pv", processed_pv.as_str()),
        ("from", &start_date),
        ("to", &end_date),
        ("fetchLatestMetadata", "true"),
    ];

    // Build the URL based on the data format
    let url = self.build_url(
        match format {
            DataFormat::Raw => "data/getData.raw",
            DataFormat::Json => "data/getData.json",
        },
        &params,
    )?;

        println!("url: {}", url);
        let response = self.client.get(url).send().await.map_err(Error::Network)?;
        // println!("response: {:?}", response);
        match format {
            DataFormat::Raw => {
                println!("Raw format being called");
                let bytes = response.bytes().await.map_err(Error::Network)?;
                let mut decoder = DecoderContext::new(self.determine_batch_size(end - start));
                let pv_data = decoder.decode_response(&bytes)?;
                //println!("pv_data: {:?}", pv_data);
                pv_data
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::Invalid("No data returned".to_string()))
            }
            DataFormat::Json => {
                let bytes = response.bytes().await.map_err(Error::Network)?;

                let json_data: Vec<PVDataJson> = serde_json::from_slice(&bytes)
                    .map_err(|e| Error::Invalid(format!("Failed to parse JSON: {}", e)))?;

                let pv_data_json = json_data.into_iter().next().ok_or_else(|| {
                    Error::Invalid("No data returned in JSON response".to_string())
                })?;

                let data = pv_data_json
                    .data
                    .into_iter()
                    .map(|point| {
                        Ok(Point {
                            secs: point.secs,
                            nanos: point.nanos,
                            val: point.to_point_value()?,
                            severity: point.severity.unwrap_or(0) as i32,
                            status: point.status.unwrap_or(0) as i32,
                        })
                    })
                    .collect::<Result<Vec<Point>, Error>>()?;

                Ok(PVData {
                    meta: pv_data_json.meta,
                    data,
                })
            }
        }
    }

    pub fn build_url(&self, path: &str, params: &[(&str, &str)]) -> Result<Url, Error> {
        let mut url = Url::parse(&format!("{}/{}", self.base_url, path))
            .map_err(|e| Error::Invalid(format!("Invalid URL: {}", e)))?;
        url.query_pairs_mut().extend_pairs(params);
        Ok(url)
    }

    fn determine_batch_size(&self, duration_seconds: i64) -> usize {
        match duration_seconds {
            d if d <= 5 => 10,
            d if d <= 60 => 30,
            d if d <= 300 => 100,
            _ => 500,
        }
    }

    pub async fn get_metadata(&self, pv: &str) -> Result<Meta, Error> {
        let url = self.build_url("bpl/getMetadata", &[("pv", pv)])?;
    
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
                "Server returned {} for {}\nResponse: {}",
                status, url, error_body
            )));
        }
    
        let bytes = response.bytes().await.map_err(Error::Network)?;
        
        // Print the raw response for debugging
        println!("Metadata response: {}", String::from_utf8_lossy(&bytes));
    
        let meta: Meta = serde_json::from_slice(&bytes)
            .map_err(|e| Error::Invalid(format!("Failed to parse metadata: {} - Raw response: {}", 
                e, String::from_utf8_lossy(&bytes))))?;
    
        // Ensure required fields are present
        if meta.name.is_empty() {
            return Err(Error::Invalid("Missing name in metadata response".to_string()));
        }
    
        Ok(meta)
    }
    

    // In convert_to_uplot
    fn convert_to_uplot(pv_data: Vec<PVData>) -> UPlotData {
        let mut timestamps = Vec::new();
        let mut series = Vec::with_capacity(pv_data.len());
    
        for pv in &pv_data {
            let mut pv_values = Vec::new();
            
            // Add timestamps and values for this PV
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
                    // Only add timestamp if first PV
                    if timestamps.len() < pv.data.len() {
                        timestamps.push(ts as f64);
                    }
                    pv_values.push(val);
                }
            }
            series.push(pv_values);
        }
    
        UPlotData {
            timestamps,
            series,
            meta: pv_data.iter().map(|pv| pv.meta.clone()).collect(),
        }
    }


    }

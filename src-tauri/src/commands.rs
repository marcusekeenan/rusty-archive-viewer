use chrono::Utc;
use serde::{Serialize, Deserialize};
use tauri::State;

use crate::client::ArchiverClient;
use crate::types::{Config, PVData, Meta, Point, ProcessingMode, UPlotData, BinningOperation, DataFormat};

pub struct AppState {
    client: ArchiverClient,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            client: ArchiverClient::new(config),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FetchDataOptions {
    pub pvs: Vec<String>,
    pub from: i64,
    pub to: i64,
    pub target_points: Option<usize>,
    pub format: Option<DataFormat>,
}

#[tauri::command]
pub async fn fetch_data(
    state: State<'_, AppState>,
    options: FetchDataOptions,
) -> Result<UPlotData, String> {
    let mode = options.target_points.map(ProcessingMode::Optimized);
    let format = options.format.unwrap_or_default();
    
    state.client
        .fetch_data_uplot(options.pvs, options.from, options.to, mode, format)
        .await
        .map(|(data, _size)| data)
        .map_err(|e| format!("Error fetching data: {}", e))
}

#[derive(Debug, Deserialize)]
pub struct LiveDataOptions {
    pub pvs: Vec<String>,
    pub format: Option<DataFormat>,
}

#[tauri::command]
pub async fn fetch_live_data(
    state: State<'_, AppState>,
    options: LiveDataOptions,
) -> Result<UPlotData, String> {
    let end = Utc::now().timestamp();
    let start = end - 300; // Last 5 minutes
    let format = options.format.unwrap_or_default();
    
    state.client
        .fetch_data_uplot(options.pvs, start, end, Some(ProcessingMode::Raw), format)
        .await
        .map(|(data, _size)| data)
        .map_err(|e| format!("Error fetching live data: {}", e))
}

#[derive(Debug, Deserialize)]
pub struct FetchLatestOptions {
    pub pv: String,
    pub format: Option<DataFormat>,
}

#[tauri::command]
pub async fn fetch_latest(
    state: State<'_, AppState>,
    options: FetchLatestOptions,
) -> Result<Point, String> {
    let end = Utc::now().timestamp();
    let start = end - 5; // Last 5 seconds
    let format = options.format.unwrap_or_default();

    let (data, _size) = state.client
        .fetch_data_with_processing(&options.pv, start, end, ProcessingMode::Raw, format)
        .await
        .map_err(|e| e.to_string())?;

    data.data.last()
        .cloned()
        .ok_or_else(|| "No data available".to_string())
}

#[derive(Debug, Deserialize)]
pub struct MetadataOptions {
    pub pv: String,
}

#[tauri::command]
pub async fn get_pv_metadata(
    state: State<'_, AppState>,
    options: MetadataOptions,
) -> Result<Meta, String> {
    state.client
        .get_metadata(&options.pv)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    format: Option<DataFormat>,
) -> Result<bool, String> {
    let now = Utc::now().timestamp();
    let format = format.unwrap_or_default();

    match state.client
        .fetch_data_with_processing(
            "ROOM:LI30:1:OUTSIDE_TEMP",
            now - 60,
            now,
            ProcessingMode::Raw,
            format
        )
        .await 
    {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[derive(Debug, Deserialize)]
pub struct BinnedDataOptions {
    pub pvs: Vec<String>,
    pub from: i64,
    pub to: i64,
    pub bin_size: u32,
    pub operation: String,
    pub format: Option<DataFormat>,
}

#[tauri::command]
pub async fn fetch_binned_data(
    state: State<'_, AppState>,
    options: BinnedDataOptions,
) -> Result<UPlotData, String> {
    let operation = match options.operation.to_lowercase().as_str() {
        "mean" => BinningOperation::Mean,
        "max" => BinningOperation::Max,
        "min" => BinningOperation::Min,
        "jitter" => BinningOperation::Jitter,
        "stddev" => BinningOperation::StdDev,
        "count" => BinningOperation::Count,
        "firstsample" => BinningOperation::FirstSample,
        "lastsample" => BinningOperation::LastSample,
        "firstfill" => BinningOperation::FirstFill,
        "lastfill" => BinningOperation::LastFill,
        "median" => BinningOperation::Median,
        "variance" => BinningOperation::Variance,
        "popvariance" => BinningOperation::PopVariance,
        "kurtosis" => BinningOperation::Kurtosis,
        "skewness" => BinningOperation::Skewness,
        "linear" => BinningOperation::Linear,
        "loess" => BinningOperation::Loess,
        "caplotbinning" => BinningOperation::CAPlotBinning,
        _ => return Err("Invalid binning operation".to_string()),
    };

    let mode = ProcessingMode::Binning {
        bin_size: options.bin_size,
        operation,
    };

    let format = options.format.unwrap_or_default();

    state.client
        .fetch_data_uplot(options.pvs, options.from, options.to, Some(mode), format)
        .await
        .map(|(data, _size)| data)
        .map_err(|e| format!("Error fetching binned data: {}", e))
}

#[derive(Debug, Deserialize)]
pub struct ChartDataOptions {
    pub pvs: Vec<String>,
    pub from: i64,
    pub to: i64,
    pub target_points: Option<usize>,
    pub format: Option<DataFormat>,
}

#[tauri::command]
pub async fn fetch_chart_data(
    state: State<'_, AppState>,
    options: ChartDataOptions,
) -> Result<UPlotData, String> {
    let mode = options.target_points.map(ProcessingMode::Optimized);
    let format = options.format.unwrap_or_default();
    
    state.client
        .fetch_data_uplot(options.pvs, options.from, options.to, mode, format)
        .await
        .map(|(data, _size)| data)
        .map_err(|e| format!("Error fetching chart data: {}", e))
}

#[derive(Debug, Deserialize)]
pub struct LiveChartDataOptions {
    pub pvs: Vec<String>,
    pub target_points: Option<usize>,
    pub format: Option<DataFormat>,
}

#[tauri::command]
pub async fn fetch_live_chart_data(
    state: State<'_, AppState>,
    options: LiveChartDataOptions,
) -> Result<UPlotData, String> {
    let end = Utc::now().timestamp();
    let start = end - 300; // Last 5 minutes
    let format = options.format.unwrap_or_default();

    let mode = options.target_points.map(ProcessingMode::Optimized);
    
    state.client
        .fetch_data_uplot(options.pvs, start, end, mode, format)
        .await
        .map(|(data, _size)| data)
        .map_err(|e| format!("Error fetching live chart data: {}", e))
}
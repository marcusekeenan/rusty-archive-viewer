use chrono::Utc;
use serde::{Serialize, Deserialize};
use tauri::State;

use crate::client::{ArchiverClient, ProcessingMode, UPlotData, BinningOperation};
use crate::types::{Config, PVData, Meta, Point};

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

#[tauri::command]
pub async fn fetch_data(
    state: State<'_, AppState>,
    pvs: Vec<String>,
    from: i64,
    to: i64,
    target_points: Option<usize>,
) -> Result<UPlotData, String> {
    let mode = target_points.map(ProcessingMode::Optimized);
    
    state.client
        .fetch_data_uplot(pvs, from, to, mode)
        .await
        .map_err(|e| format!("Error fetching data: {}", e))
}

#[tauri::command]
pub async fn fetch_live_data(
    state: State<'_, AppState>,
    pvs: Vec<String>,
) -> Result<UPlotData, String> {
    let end = Utc::now().timestamp();
    let start = end - 300; // Last 5 minutes
    
    state.client
        .fetch_data_uplot(pvs, start, end, Some(ProcessingMode::Raw))
        .await
        .map_err(|e| format!("Error fetching live data: {}", e))
}

#[tauri::command]
pub async fn fetch_latest(
    state: State<'_, AppState>,
    pv: String,
) -> Result<Point, String> {
    let end = Utc::now().timestamp();
    let start = end - 5; // Last 5 seconds

    let data = state.client
        .fetch_data_with_processing(&pv, start, end, ProcessingMode::Raw)
        .await
        .map_err(|e| e.to_string())?;

    data.data.last()
        .cloned()
        .ok_or_else(|| "No data available".to_string())
}

#[tauri::command]
pub async fn get_pv_metadata(
    state: State<'_, AppState>,
    pv: String,
) -> Result<Meta, String> {
    // Use direct metadata endpoint instead of data endpoint
    state.client
        .get_metadata(&pv)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn test_connection(state: State<'_, AppState>) -> Result<bool, String> {
    let now = Utc::now().timestamp();
    match state.client
        .fetch_data_with_processing(
            "ROOM:LI30:1:OUTSIDE_TEMP",
            now - 60,
            now,
            ProcessingMode::Raw
        )
        .await 
    {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn fetch_binned_data(
    state: State<'_, AppState>,
    pvs: Vec<String>,
    from: i64,
    to: i64,
    bin_size: u32,
    operation: String,
) -> Result<UPlotData, String> {
    let operation = match operation.to_lowercase().as_str() {
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
        bin_size,
        operation,
    };

    state.client
        .fetch_data_uplot(pvs, from, to, Some(mode))
        .await
        .map_err(|e| format!("Error fetching binned data: {}", e))
}

#[tauri::command]
pub async fn fetch_chart_data(
    state: State<'_, AppState>,
    pvs: Vec<String>,
    from: i64,
    to: i64,
    target_points: Option<usize>,
) -> Result<UPlotData, String> {
    let mode = target_points.map(ProcessingMode::Optimized);
    
    state.client
        .fetch_data_uplot(pvs, from, to, mode)
        .await
        .map_err(|e| format!("Error fetching chart data: {}", e))
}

#[tauri::command]
pub async fn fetch_live_chart_data(
    state: State<'_, AppState>,
    pvs: Vec<String>,
    target_points: Option<usize>,
) -> Result<UPlotData, String> {
    let end = Utc::now().timestamp();
    let start = end - 300; // Last 5 minutes

    let mode = match target_points {
        Some(points) => Some(ProcessingMode::Optimized(points)),
        None => None, // Let backend choose optimal mode
    };
    
    state.client
        .fetch_data_uplot(pvs, start, end, mode)
        .await
        .map_err(|e| format!("Error fetching live chart data: {}", e))
}
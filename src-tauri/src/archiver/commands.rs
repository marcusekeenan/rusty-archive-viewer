//! Tauri commands for interacting with the EPICS Archiver Appliance
//! Provides the interface between the frontend and the archiver API

use super::api::ArchiverClient;
use crate::archiver::{types::*, export::*};
use std::collections::HashMap;

/// Fetches binned data for multiple PVs with options
#[tauri::command]
pub async fn fetch_binned_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    options: Option<ExtendedFetchOptions>,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ArchiverClient::new()?;
    let options = options.unwrap_or_default();

    client.fetch_binned_data(&pvs, from, to, &options).await
}

/// Fetches metadata for a PV
#[tauri::command]
pub async fn get_pv_metadata(pv: String) -> Result<Meta, String> {
    let client = ArchiverClient::new()?;
    client.fetch_metadata(&pv).await
}

/// Gets data at a specific point in time for multiple PVs
#[tauri::command]
pub async fn get_data_at_time(
    pvs: Vec<String>,
    timestamp: i64,
    options: Option<ExtendedFetchOptions>,
) -> Result<HashMap<String, PointValue>, String> {
    let client = ArchiverClient::new()?;
    let options = options.unwrap_or_default();
    
    client.get_data_at_time(&pvs, timestamp, &options).await
}

/// Exports data in various formats
#[tauri::command]
pub async fn export_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    format: DataFormat,
    options: Option<ExtendedFetchOptions>,
) -> Result<String, String> {
    let client = ArchiverClient::new()?;
    let mut options = options.unwrap_or_default();
    
    // Ensure format is set in options
    options.format = Some(format.clone());

    // Fetch the data
    let data = client.fetch_binned_data(&pvs, from, to, &options).await?;
    
    // Export in requested format
    match format {
        DataFormat::Csv => export_to_csv(&data),
        DataFormat::Matlab => export_to_matlab(&data),
        DataFormat::Text => export_to_text(&data),
        DataFormat::Svg => export_to_svg(&data),
        _ => Err("Unsupported export format".to_string())
    }
}

/// Retrieves data with a specific operator
#[tauri::command]
pub async fn fetch_data_with_operator(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    operator: String,
    options: Option<ExtendedFetchOptions>,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ArchiverClient::new()?;
    let mut options = options.unwrap_or_default();
    
    // Set the specified operator
    options.operator = Some(operator);

    client.fetch_binned_data(&pvs, from, to, &options).await
}

/// Fetches raw data without any processing
#[tauri::command]
pub async fn fetch_raw_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ArchiverClient::new()?;
    let options = ExtendedFetchOptions {
        operator: Some("raw".to_string()),
        do_not_chunk: Some(true),
        ..Default::default()
    };

    client.fetch_binned_data(&pvs, from, to, &options).await
}

/// Fetches optimized data based on time range and display width
#[tauri::command]
pub async fn fetch_optimized_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    chart_width: i32,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ArchiverClient::new()?;
    let options = ExtendedFetchOptions {
        chart_width: Some(chart_width),
        ..Default::default()
    };

    client.fetch_binned_data(&pvs, from, to, &options).await
}

/// Validates PV names
#[tauri::command]
pub async fn validate_pvs(pvs: Vec<String>) -> Result<Vec<bool>, String> {
    let client = ArchiverClient::new()?;
    
    let mut results = Vec::with_capacity(pvs.len());
    for pv in pvs {
        match client.fetch_metadata(&pv).await {
            Ok(_) => results.push(true),
            Err(_) => results.push(false),
        }
    }
    
    Ok(results)
}

/// Gets status information for PVs
#[tauri::command]
pub async fn get_pv_status(pvs: Vec<String>) -> Result<Vec<PVStatus>, String> {
    let client = ArchiverClient::new()?;
    let mut statuses = Vec::with_capacity(pvs.len());
    
    for pv in pvs {
        let metadata = client.fetch_metadata(&pv).await;
        let cloned_metadata = metadata.clone();
        let status = PVStatus {
            name: pv.clone(),
            connected: metadata.is_ok(),
            last_event_time: None,
            last_status: cloned_metadata.map_err(|e| e).err(),
            archived: metadata.is_ok(),
            error_count: 0,
            last_error: None,
        };
        statuses.push(status);
    }
    
    Ok(statuses)
}

/// Tests connection to the archiver
#[tauri::command]
pub async fn test_connection() -> Result<bool, String> {
    let client = ArchiverClient::new()?;
    
    // Try to fetch metadata for a known PV
    match client.fetch_metadata("ROOM:LI30:1:OUTSIDE_TEMP").await {
        Ok(_) => Ok(true),
        Err(e) => Ok(false),
    }
}
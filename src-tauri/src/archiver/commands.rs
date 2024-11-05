//! Tauri commands for interacting with the EPICS Archiver Appliance

use crate::archiver::{
    types::*,
    api::ArchiveViewerApi,
    error::{ArchiverError, Result as ArchiverResult},
    export::{export_to_csv, export_to_matlab, export_to_text, export_to_svg},
}; 
use std::collections::HashMap;
use tokio::sync::OnceCell;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

// Global client instance - store Arc<ArchiveViewerApi> since that's what new() returns
static CLIENT: OnceCell<Arc<ArchiveViewerApi>> = OnceCell::const_new();

async fn get_client() -> ArchiverResult<Arc<ArchiveViewerApi>> {
    if let Some(client) = CLIENT.get() {
        Ok(client.clone())
    } else {
        let client = ArchiveViewerApi::new(
            "http://lcls-archapp.slac.stanford.edu/retrieval/data".to_string()
        ).await?;
        
        // Initialize global client
        if let Err(_) = CLIENT.set(client.clone()) {
            return Err(ArchiverError::ConnectionError {
                message: "Failed to initialize archiver client".to_string(),
                context: "Client initialization".to_string(),
                source: None,
                retry_after: None,
            });
        }
        
        Ok(client)
    }
}

// Helper function to convert ArchiverError to String
fn to_string_error(err: ArchiverError) -> String {
    format!("{}", err)
}

/// Creates session and fetches binned data for multiple PVs with options
#[tauri::command]
pub async fn fetch_binned_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    options: Option<ExtendedFetchOptions>,
) -> std::result::Result<Vec<NormalizedPVData>, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let session_id = Uuid::new_v4();
    let time_range = TimeRange { start: from, end: to };
    let resolution = options.and_then(|opt| opt.operator);

    client.fetch_data(session_id, pvs, time_range, resolution)
        .await
        .map_err(to_string_error)
        .map(|data| {
            data.into_iter()
                .map(|(name, points)| NormalizedPVData {
                    meta: Meta {
                        name,
                        egu: String::new(),
                        description: None,
                        precision: None,
                        archive_parameters: None,
                        display_limits: None,
                        alarm_limits: None,
                    },
                    data: points,
                    statistics: None,
                })
                .collect()
        })
}


/// Gets data at a specific point in time for multiple PVs
#[tauri::command]
pub async fn get_data_at_time(
    pvs: Vec<String>,
    timestamp: i64,
    options: Option<ExtendedFetchOptions>,
) -> std::result::Result<HashMap<String, PointValue>, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let session_id = Uuid::new_v4();
    let time_range = TimeRange {
        start: timestamp,
        end: timestamp + 1,
    };

    client.fetch_data(session_id, pvs, time_range, None)
        .await
        .map_err(to_string_error)
        .map(|data| {
            data.into_iter()
                .filter_map(|(name, points)| {
                    points.first().map(|point| {
                        (name, PointValue {
                            secs: point.timestamp / 1000,
                            nanos: Some((point.timestamp % 1000) * 1_000_000),
                            val: Value::Single(point.value),
                            severity: Some(point.severity),
                            status: Some(point.status),
                        })
                    })
                })
                .collect()
        })
}

/// Exports data in various formats
#[tauri::command]
pub async fn export_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    format: DataFormat,
    options: Option<ExtendedFetchOptions>,
) -> std::result::Result<String, String> {
    // First get the data using fetch_binned_data
    let data = fetch_binned_data(pvs, from, to, options).await?;
    
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
) -> std::result::Result<Vec<NormalizedPVData>, String> {
    let mut options = options.unwrap_or_default();
    options.operator = Some(operator);
    fetch_binned_data(pvs, from, to, Some(options)).await
}

/// Fetches raw data without any processing
#[tauri::command]
pub async fn fetch_raw_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
) -> std::result::Result<Vec<NormalizedPVData>, String> {
    let options = ExtendedFetchOptions {
        operator: Some("raw".to_string()),
        do_not_chunk: Some(true),
        ..Default::default()
    };

    fetch_binned_data(pvs, from, to, Some(options)).await
}

/// Fetches optimized data based on time range and display width
#[tauri::command]
pub async fn fetch_optimized_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    chart_width: i32,
) -> std::result::Result<Vec<NormalizedPVData>, String> {
    let options = ExtendedFetchOptions {
        chart_width: Some(chart_width),
        ..Default::default()
    };

    fetch_binned_data(pvs, from, to, Some(options)).await
}

/// Gets status information for PVs
#[tauri::command]
pub async fn get_pv_status(pvs: Vec<String>) -> std::result::Result<Vec<PVStatus>, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let session_id = Uuid::new_v4();

    let time_range = TimeRange {
        start: Utc::now().timestamp() - 60,
        end: Utc::now().timestamp(),
    };

    let mut statuses = Vec::with_capacity(pvs.len());
    
    for pv in pvs {
        let result = client.fetch_data(
            session_id,
            vec![pv.clone()],
            time_range.clone(),
            None
        ).await;

        let is_ok = result.is_ok();
        let error_msg = result.map_err(to_string_error).err();
        
        let status = PVStatus {
            name: pv,
            connected: is_ok,
            last_event_time: None,
            last_status: error_msg,
            archived: is_ok,
            error_count: 0,
            last_error: None,
        };
        statuses.push(status);
    }
    
    Ok(statuses)
}

/// Tests connection to the archiver
#[tauri::command]
pub async fn test_connection() -> std::result::Result<bool, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let session_id = Uuid::new_v4();
    let now = Utc::now().timestamp();
    
    match client.fetch_data(
        session_id,
        vec!["ROOM:LI30:1:OUTSIDE_TEMP".to_string()],
        TimeRange {
            start: now - 60,
            end: now,
        },
        None
    ).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Fetches metadata for a PV
#[tauri::command]
pub async fn get_pv_metadata(pv: String) -> std::result::Result<Meta, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let session_id = Uuid::new_v4();
    
    // We'll fetch a single point to get the metadata
    let time_range = TimeRange {
        start: Utc::now().timestamp() - 60,
        end: Utc::now().timestamp(),
    };

    client.fetch_data(
        session_id,
        vec![pv.clone()],
        time_range,
        None
    ).await
    .map_err(to_string_error)
    .and_then(|data| {
        data.get(&pv)
            .and_then(|points| points.first())
            .map(|point| Meta {
                name: pv,
                egu: String::new(), // We need to get this from the point
                description: None,
                precision: None,
                archive_parameters: None,
                display_limits: None,
                alarm_limits: None,
            })
            .ok_or("No data available for PV".to_string())
    })
}
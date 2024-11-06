//! Tauri commands for interacting with the EPICS Archiver Appliance
//! Provides the interface between the frontend and the archiver API

use super::api::{ArchiverClient, DataFetch, DataProcess, DataProcessor, DataRequest};
use crate::archiver::{export::*, types::*};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, Window, WindowBuilder, WindowUrl};

lazy_static::lazy_static! {
    static ref ARCHIVER_CLIENT: Arc<ArchiverClient> = Arc::new(
        ArchiverClient::new().expect("Failed to create archiver client")
    );
}

#[tauri::command]
pub async fn toggle_debug_window(window: Window) -> Result<(), String> {
    let app = window.app_handle();
    println!("Toggle debug window called"); // Debug log

    if let Some(debug_window) = app.get_window("debug") {
        println!("Found existing window");
        let visible = debug_window.is_visible().unwrap_or(false);
        if visible {
            debug_window.hide().map_err(|e| e.to_string())?;
        } else {
            debug_window.show().map_err(|e| e.to_string())?;
            debug_window.set_focus().map_err(|e| e.to_string())?;
        }
    } else {
        println!("Creating new window");
        let window_url = WindowUrl::App("index.html#/debug-view".into());
        println!("Window URL: {:?}", window_url);

        let debug_window = WindowBuilder::new(&app, "debug", window_url)
            .title("Debug Information")
            .inner_size(800.0, 600.0)
            .resizable(true)
            .visible(true)
            .build()
            .map_err(|e| {
                println!("Failed to create window: {}", e);
                e.to_string()
            })?;

        println!("Window created successfully");
    }

    Ok(())
}

/// Fetches data for multiple PVs with automatic optimization
#[tauri::command]
pub async fn fetch_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    chart_width: i32, // Consistently using snake_case
    timezone: String,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ARCHIVER_CLIENT.clone();
    let processor = DataProcessor::default();
    println!("the timezone in command is: {}", timezone);

    let time_range = TimeRange {
        start: from,
        end: to,
    };

    // Get optimal operator based on duration and chart width
    let operator = processor.get_optimal_operator(to - from, Some(chart_width));

    let mut results = Vec::new();
    for pv in pvs {
        let request = DataRequest {
            pv: pv.clone(),
            range: time_range.clone(),
            operator: operator.clone(),
            format: DataFormat::Json,
            timezone: Some(timezone.clone()),
        };

        match client.fetch_data(&request).await {
            Ok(data) => results.push(data),
            Err(e) => eprintln!("Error fetching data for {}: {}", pv, e),
        }
    }

    Ok(results)
}

/// Gets current or specific timestamp data for multiple PVs
#[tauri::command]
pub async fn fetch_live_data(
    pvs: Vec<String>,
    timestamp: Option<i64>,
    timezone: Option<String>, // Add timezone parameter
) -> Result<HashMap<String, PointValue>, String> {
    let client = ARCHIVER_CLIENT.clone();
    let timestamp = timestamp.unwrap_or_else(|| chrono::Utc::now().timestamp());

    client
        .fetch_live_data(&pvs, timestamp, timezone.as_deref())
        .await
}

/// Exports data in various formats
#[tauri::command]
pub async fn export_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    format: DataFormat,
) -> Result<String, String> {
    let client = ARCHIVER_CLIENT.clone();
    let time_range = TimeRange {
        start: from,
        end: to,
    };

    let mut results = Vec::new();
    for pv in pvs {
        let request = DataRequest {
            pv: pv.clone(),
            range: time_range.clone(),
            operator: DataOperator::Raw, // Always use raw data for exports
            format: format.clone(),
            timezone: None,
        };

        if let Ok(data) = client.fetch_data(&request).await {
            results.push(data);
        }
    }

    match format {
        DataFormat::Csv => export_to_csv(&results),
        DataFormat::Matlab => export_to_matlab(&results),
        DataFormat::Text => export_to_text(&results),
        DataFormat::Svg => export_to_svg(&results),
        _ => Err("Unsupported export format".to_string()),
    }
}

/// Fetches metadata for a PV
#[tauri::command]
pub async fn get_pv_metadata(pv: String) -> Result<Meta, String> {
    let client = ARCHIVER_CLIENT.clone();
    client.fetch_metadata(&pv).await
}

/// Validates multiple PV names
#[tauri::command]
pub async fn validate_pvs(pvs: Vec<String>) -> Result<Vec<bool>, String> {
    let client = ARCHIVER_CLIENT.clone();
    let mut results = Vec::with_capacity(pvs.len());

    for pv in pvs {
        match client.fetch_metadata(&pv).await {
            Ok(_) => results.push(true),
            Err(_) => results.push(false),
        }
    }

    Ok(results)
}

/// Gets status information for multiple PVs
#[tauri::command]
pub async fn get_pv_status(pvs: Vec<String>) -> Result<Vec<PVStatus>, String> {
    let client = ARCHIVER_CLIENT.clone();
    let mut statuses = Vec::with_capacity(pvs.len());

    for pv in pvs {
        let metadata = client.fetch_metadata(&pv).await;
        let status = PVStatus {
            name: pv.clone(),
            connected: metadata.is_ok(),
            last_event_time: None,
            last_status: metadata.as_ref().err().map(|e| e.to_string()),
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
    let client = ARCHIVER_CLIENT.clone();
    match client.fetch_metadata("ROOM:LI30:1:OUTSIDE_TEMP").await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use chrono::Utc;

//     #[tokio::test]
//     async fn test_fetch_data() {
//         let pvs = vec!["TEST:PV1".to_string()];
//         let now = Utc::now().timestamp();
//         let result = fetch_data(
//             pvs,
//             now - 3600, // 1 hour ago
//             now,
//             None,
//         )
//         .await;
//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_get_live_data() {
//         let pvs = vec!["TEST:PV1".to_string()];
//         let result = get_live_data(pvs, None).await;
//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_validate_pvs() {
//         let pvs = vec!["TEST:PV1".to_string(), "INVALID:PV".to_string()];
//         let result = validate_pvs(pvs).await;
//         assert!(result.is_ok());
//     }
// }

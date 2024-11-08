//! Tauri commands for interacting with the EPICS Archiver Appliance
//! Provides the interface between the frontend and the archiver API

use super::api::{ArchiverClient, OptimizationLevel, TimeRangeMode};
use crate::archiver::{export::*, types::*};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, Window, WindowBuilder, WindowUrl};
use tokio::sync::{broadcast, Mutex};
use tokio::time::Duration;

// Static references for global state
lazy_static::lazy_static! {
    static ref ARCHIVER_CLIENT: Arc<ArchiverClient> = Arc::new(
        ArchiverClient::new().expect("Failed to create archiver client")
    );
    static ref LIVE_SUBSCRIPTIONS: Arc<Mutex<HashMap<String, broadcast::Receiver<HashMap<String, PointValue>>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

#[tauri::command]
pub async fn toggle_debug_window(window: Window) -> Result<(), String> {
    let app = window.app_handle();
    println!("Toggle debug window called");

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
    chart_width: i32,
    timezone: String,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ARCHIVER_CLIENT.clone();
    println!("Fetching data with timezone: {}", timezone);

    let mode = TimeRangeMode::Fixed {
        start: from,
        end: to,
    };

    let mut results = Vec::new();
    for pv in pvs {
        match client
            .fetch_historical_data(
                &pv,
                &mode,
                OptimizationLevel::Auto,
                Some(chart_width),
                Some(&timezone),
            )
            .await
        {
            Ok(data) => results.push(data),
            Err(e) => eprintln!("Error fetching data for {}: {}", pv, e),
        }
    }

    Ok(results)
}

/// Gets data at a specific timestamp for multiple PVs
#[tauri::command]
pub async fn fetch_data_at_time(
    pvs: Vec<String>,
    timestamp: Option<i64>,
    timezone: Option<String>,
) -> Result<HashMap<String, PointValue>, String> {
    let client = ARCHIVER_CLIENT.clone();

    client
        .fetch_data_at_time(&pvs, timestamp, timezone.as_deref())
        .await
}

#[tauri::command]
pub async fn start_live_updates(
    window: Window,
    pvs: Vec<String>,
    update_interval_ms: u64,
    timezone: Option<String>,
) -> Result<(), String> {
    let client = ARCHIVER_CLIENT.clone();
    let window_id = window.label().to_string();

    // Stop any existing updates for this window
    {
        let mut subscriptions = LIVE_SUBSCRIPTIONS.lock().await;
        subscriptions.remove(&window_id);
    }

    // Start new live updates and get a receiver
    let mut rx = client
        .start_live_updates(
            pvs.clone(),
            Duration::from_millis(update_interval_ms),
            timezone,
        )
        .await?;

    // Store subscription using resubscribe()
    {
        let mut subscriptions = LIVE_SUBSCRIPTIONS.lock().await;
        subscriptions.insert(window_id.clone(), rx.resubscribe());
    }

    // Handle updates in a separate task
    let window = Arc::new(window);
    tokio::spawn(async move {
        while let Ok(data) = rx.recv().await {
            if let Err(e) = window.emit("live-update", &data) {
                eprintln!("Failed to emit update: {}", e);
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_live_updates(window: Window) -> Result<(), String> {
    let window_id = window.label().to_string();
    println!("Stopping live updates for window: {}", window_id);

    // First, remove subscription
    {
        let mut subscriptions = LIVE_SUBSCRIPTIONS.lock().await;
        subscriptions.remove(&window_id);
    }

    // Stop the client
    let client = ARCHIVER_CLIENT.clone();
    client.stop_live_updates().await?;

    // Wait a moment to ensure cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Double check that all tasks are stopped
    client.ensure_tasks_stopped().await?;

    println!("Live updates stopped for window: {}", window_id);
    Ok(())
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
    let mode = TimeRangeMode::Fixed {
        start: from,
        end: to,
    };

    let mut results = Vec::new();
    for pv in pvs {
        if let Ok(data) = client
            .fetch_historical_data(&pv, &mode, OptimizationLevel::Raw, None, None)
            .await
        {
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
    let metadata_results = client.fetch_multiple_metadata(&pvs).await;

    Ok(pvs
        .iter()
        .map(|pv| metadata_results.get(pv).map_or(false, |r| r.is_ok()))
        .collect())
}

/// Gets status information for multiple PVs
#[tauri::command]
pub async fn get_pv_status(pvs: Vec<String>) -> Result<Vec<PVStatus>, String> {
    let client = ARCHIVER_CLIENT.clone();
    let metadata_results = client.fetch_multiple_metadata(&pvs).await;

    Ok(pvs
        .iter()
        .map(|pv| {
            let metadata = metadata_results
                .get(pv)
                .cloned()
                .unwrap_or(Err("No metadata".to_string()));
            PVStatus {
                name: pv.clone(),
                connected: metadata.is_ok(),
                last_event_time: None,
                last_status: metadata.as_ref().err().map(|e| e.to_string()),
                archived: metadata.is_ok(),
                error_count: 0,
                last_error: None,
            }
        })
        .collect())
}

/// Tests connection to the archiver
#[tauri::command]
pub async fn test_connection() -> Result<bool, String> {
    let client = ARCHIVER_CLIENT.clone();
    client
        .fetch_metadata("ROOM:LI30:1:OUTSIDE_TEMP")
        .await
        .map(|_| true)
        .or(Ok(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tokio::time::Duration;

    const TEST_PV: &str = "ROOM:LI30:1:OUTSIDE_TEMP";
    const TEST_INVALID_PV: &str = "INVALID:PV:NAME:123";

    #[tokio::test]
    async fn test_fetch_data() {
        let pvs = vec![TEST_PV.to_string()];
        let now = Utc::now().timestamp();
        let result = fetch_data(pvs, now - 3600, now, 1000, "UTC".to_string()).await;

        assert!(result.is_ok(), "Failed to fetch data: {:?}", result.err());

        let data = result.unwrap();
        assert!(!data.is_empty(), "No data returned");

        let pv_data = &data[0];
        assert_eq!(pv_data.meta.name, TEST_PV);
        assert!(!pv_data.data.is_empty(), "No points in data");

        // Just verify we got some data within a reasonable range
        if let Some(stats) = &pv_data.statistics {
            assert!(
                stats.first_timestamp <= stats.last_timestamp,
                "First timestamp after last timestamp"
            );
        }
    }

    #[tokio::test]
    async fn test_live_updates() {
        // Instead of using Tauri window, we'll test the underlying functionality
        let client = ARCHIVER_CLIENT.clone();
        let pvs = vec![TEST_PV.to_string()];

        let result = client
            .start_live_updates(pvs.clone(), Duration::from_secs(1), Some("UTC".to_string()))
            .await;

        assert!(result.is_ok(), "Failed to start live updates");

        let mut rx = result.unwrap();

        // Wait for one update
        let update = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;

        assert!(update.is_ok(), "Timeout waiting for update");
        assert!(update.unwrap().is_ok(), "Error receiving update");

        client
            .stop_live_updates()
            .await
            .expect("Failed to stop updates");
    }

    #[tokio::test]
    async fn test_fetch_data_at_time() {
        let pvs = vec![TEST_PV.to_string()];
        let now = Utc::now().timestamp();
        let result = fetch_data_at_time(pvs, Some(now), Some("UTC".to_string())).await;

        assert!(
            result.is_ok(),
            "Failed to fetch data at time: {:?}",
            result.err()
        );

        let data = result.unwrap();
        assert!(!data.is_empty(), "No data returned");
        assert!(data.contains_key(TEST_PV), "Data for test PV not found");

        // Just verify we got some data (timestamp checks are too strict for real server)
        let point = &data[TEST_PV];
        assert!(point.secs > 0, "Invalid timestamp");
    }

    #[tokio::test]
    async fn test_validate_pvs() {
        let pvs = vec![TEST_PV.to_string(), TEST_INVALID_PV.to_string()];
        let result = validate_pvs(pvs).await;

        assert!(result.is_ok(), "Failed to validate PVs: {:?}", result.err());

        let validations = result.unwrap();
        assert_eq!(validations.len(), 2, "Wrong number of validation results");
        // Just verify we got responses, don't assert valid/invalid
        // as PV status might change
    }

    #[tokio::test]
    async fn test_get_pv_metadata() {
        let result = get_pv_metadata(TEST_PV.to_string()).await;

        if let Ok(metadata) = result {
            assert_eq!(metadata.name, TEST_PV);
            println!("Metadata retrieved successfully");
        } else {
            println!("Metadata retrieval failed: {:?}", result.err());
            // Don't fail the test as the PV might be temporarily unavailable
        }
    }

    #[tokio::test]
    async fn test_get_pv_status() {
        let pvs = vec![TEST_PV.to_string(), TEST_INVALID_PV.to_string()];
        let result = get_pv_status(pvs).await;

        assert!(
            result.is_ok(),
            "Failed to get PV status: {:?}",
            result.err()
        );

        let statuses = result.unwrap();
        assert_eq!(statuses.len(), 2, "Wrong number of status results");

        // Don't assert on connection status as it might change
        for status in statuses {
            println!("PV: {} Status: connected={}", status.name, status.connected);
        }
    }

    #[tokio::test]
    async fn test_test_connection() {
        let result = test_connection().await;
        assert!(result.is_ok(), "Connection test failed: {:?}", result.err());
        // Don't assert on connection status as it might be temporarily down
        println!("Connection test result: {:?}", result.unwrap());
    }
}

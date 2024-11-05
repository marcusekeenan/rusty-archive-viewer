use crate::archiver::{
    types::*,
    api::ArchiveViewerApi,
    error::{ArchiverError, Result as ArchiverResult},
    export::{export_to_csv, export_to_matlab, export_to_text, export_to_svg},
    health::{HealthStatus, HealthMonitor, SystemStatus},
}; 

use std::collections::HashMap;
use tokio::sync::OnceCell;
use std::sync::Arc;
use chrono::Utc;
use tracing::{debug, error, info, warn};

use super::error::ErrorContext;

// Global instances
static CLIENT: OnceCell<Arc<ArchiveViewerApi>> = OnceCell::const_new();
static HEALTH_MONITOR: OnceCell<Arc<HealthMonitor>> = OnceCell::const_new();

// Update the default URL to match your EPICS archiver installation
const LCLS_ARCHIVER_URL: &str = "http://lcls-archapp.slac.stanford.edu";

async fn get_client() -> ArchiverResult<Arc<ArchiveViewerApi>> {
    let error_context = ErrorContext::new("Commands", "get_client");

    if let Some(client) = CLIENT.get() {
        debug!("Returning existing client instance");
        Ok(client.clone())
    } else {
        debug!("Creating new client instance");
        
        // Get base URL from environment or use default
        let base_url = std::env::var("EPICS_ARCHIVER_URL")
            .or_else(|_| std::env::var("ARCHIVER_URL"))
            .unwrap_or_else(|_| LCLS_ARCHIVER_URL.to_string());

        info!("Initializing archiver client with URL: {}", base_url);

        let client = match ArchiveViewerApi::new(Some(base_url.clone())).await {
            Ok(client) => {
                info!("Successfully created client with LCLS URL");
                client
            },
            Err(e) => {
                error!("Failed to initialize LCLS archiver client: {}", e);
                return Err(ArchiverError::InitializationError {
                    message: format!("Failed to initialize LCLS archiver client: {}", e),
                    context: format!("URL: {}", base_url),
                    source: Some(Box::new(e)),
                    error_context: Some(error_context.clone()),
                });
            }
        };

        if let Err(_) = CLIENT.set(client.clone()) {
            error!("Failed to store LCLS client instance");
            return Err(ArchiverError::InitializationError {
                message: "Failed to store LCLS client instance".into(),
                context: "Client initialization".into(),
                source: None,
                error_context: Some(error_context),
            });
        }

        debug!("LCLS client initialized successfully");
        Ok(client)
    }
}
fn to_string_error(err: ArchiverError) -> String {
    match &err {
        ArchiverError::InitializationError { message, context, .. } => {
            format!("Initialization error: {} ({})", message, context)
        },
        ArchiverError::ConnectionError { message, context, .. } => {
            format!("Connection error: {} ({})", message, context)
        },
        ArchiverError::ServerError { message, status, body, .. } => {
            format!("Server error: {} (status: {}{})",
                message,
                status,
                body.as_ref().map(|b| format!(", body: {}", b)).unwrap_or_default()
            )
        },
        _ => format!("{}", err)
    }
}

#[tauri::command]
pub async fn fetch_binned_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    options: Option<ExtendedFetchOptions>,
) -> std::result::Result<Vec<NormalizedPVData>, String> {
    debug!("Fetching binned data for {} PVs", pvs.len());
    
    let client = match get_client().await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to get client: {}", e);
            return Err(to_string_error(e));
        }
    };

    let time_range = TimeRange { start: from, end: to };
    let resolution = options.and_then(|opt| opt.operator);

    debug!("Time range: {:?}, Resolution: {:?}", time_range, resolution);

    client.fetch_data(pvs, time_range, resolution)
        .await
        .map_err(|e| {
            error!("Failed to fetch data: {}", e);
            to_string_error(e)
        })
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

#[tauri::command]
pub async fn get_data_at_time(
    pvs: Vec<String>,
    timestamp: i64,
    options: Option<ExtendedFetchOptions>,
) -> std::result::Result<HashMap<String, PointValue>, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let time_range = TimeRange {
        start: timestamp,
        end: timestamp + 1,
    };

    client.fetch_data(pvs, time_range, None)
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

#[tauri::command]
pub async fn get_pv_status(pvs: Vec<String>) -> std::result::Result<Vec<PVStatus>, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let time_range = TimeRange {
        start: Utc::now().timestamp() - 60,
        end: Utc::now().timestamp(),
    };

    let mut statuses = Vec::with_capacity(pvs.len());
    
    for pv in pvs {
        let result = client.fetch_data(
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

#[tauri::command]
pub async fn test_connection() -> std::result::Result<bool, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let now = Utc::now().timestamp();
    
    match client.fetch_data(
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

#[tauri::command]
pub async fn get_pv_metadata(pv: String) -> std::result::Result<Meta, String> {
    let client = get_client().await.map_err(to_string_error)?;
    let time_range = TimeRange {
        start: Utc::now().timestamp() - 60,
        end: Utc::now().timestamp(),
    };

    client.fetch_data(
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
                egu: String::new(),
                description: None,
                precision: None,
                archive_parameters: None,
                display_limits: None,
                alarm_limits: None,
            })
            .ok_or("No data available for PV".to_string())
    })
}

pub async fn get_health_monitor() -> ArchiverResult<Arc<HealthMonitor>> {
    let error_context = ErrorContext::new("Commands", "get_health_monitor");

    if let Some(monitor) = HEALTH_MONITOR.get() {
        Ok(monitor.clone())
    } else {
        let monitor = Arc::new(HealthMonitor::new(
            chrono::Duration::seconds(10),  // check interval
            1000                            // max history
        ));

        // Start the monitor
        let monitor_clone = monitor.clone();
        monitor_clone.start();

        if let Err(_) = HEALTH_MONITOR.set(monitor.clone()) {
            return Err(ArchiverError::HealthCheckError {
                message: "Failed to initialize health monitor".to_string(),
                context: "Health monitor initialization".to_string(),
                source: None,
                error_context: Some(error_context),
            });
        }
        
        Ok(monitor)
    }
}

#[tauri::command]
pub async fn get_health_status() -> std::result::Result<HealthStatus, String> {
    let monitor = get_health_monitor().await.map_err(to_string_error)?;
    monitor.get_current_status()
        .await
        .map_err(to_string_error)
}
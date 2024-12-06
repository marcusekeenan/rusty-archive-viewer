use chrono::Utc;
use serde::Deserialize;
use tauri::State;

use crate::client::ArchiverClient;
use crate::types::{DataFormat, ProcessingMode, UPlotData, PVMetadata, Error};
use crate::Config;

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
pub struct FetchDataParams {
    pub pvs: Vec<String>,
    pub from: i64,
    pub to: i64,
    pub mode: Option<ProcessingMode>,
    pub format: Option<DataFormat>,
}

#[tauri::command]
pub async fn fetch_data(
    state: State<'_, AppState>,
    params: FetchDataParams,
) -> Result<UPlotData, String> {
    if params.pvs.is_empty() {
        return Err("No PVs provided".to_string());
    }

    if params.to <= params.from {
        return Err("End time must be after start time".to_string());
    }

    println!("Fetch data params received: {:?}", params);

    let result = state
        .client
        .fetch_data(
            params.pvs,
            params.from,
            params.to,
            params.mode,
            params.format.unwrap_or(DataFormat::Raw),
        )
        .await;

        match &result {
            Ok(data) => {
                println!("Successfully fetched data:");
                println!("  Timestamps count: {}", data.timestamps.len());
                println!("  Series count: {}", data.series.len());
                println!("  Meta count: {}", data.meta.len());
                
                // Print first few timestamps
                println!("  First 5 timestamps: {:?}", 
                    data.timestamps.iter().take(5).collect::<Vec<_>>());
                
                // Print first few values from each series
                for (i, series) in data.series.iter().enumerate() {
                    println!("  Series {} first 5 values: {:?}", 
                        i, series.iter().take(5).collect::<Vec<_>>());
                }
                
                // Print metadata
                println!("  Meta info: {:?}", data.meta);
            }
            Err(e) => {
                println!("Error fetching data: {:?}", e);
            }
        }

    result.map_err(|e| match e {
        Error::Network(e) => format!("Network error: {}", e),
        Error::Decode(msg) => format!("Data decode error: {}", msg),
        Error::Invalid(msg) => format!("Invalid request: {}", msg),
    })
}

#[tauri::command]
pub async fn get_pv_metadata(
    state: State<'_, AppState>,
    pv: String,
) -> Result<PVMetadata, String> {
    if pv.is_empty() {
        return Err("PV name cannot be empty".to_string());
    }

    let meta = state
        .client
        .get_metadata(&pv)
        .await
        .map_err(|e| match e {
            Error::Network(e) => format!("Network error: {}", e),
            Error::Decode(msg) => format!("Metadata decode error: {}", msg),
            Error::Invalid(msg) => format!("Invalid metadata: {}", msg),
        })?;

    Ok(PVMetadata {
        name: meta.name,
        EGU: meta.EGU,
        PREC: meta.PREC,
        DESC: meta.DESC,
        LOPR: meta.LOPR,
        HOPR: meta.HOPR,
        DRVL: meta.DRVL,
        DRVH: meta.DRVH,
        LOW: meta.LOW,
        HIGH: meta.HIGH,
        LOLO: meta.LOLO,
        HIHI: meta.HIHI,
    })
}

#[tauri::command]
pub async fn test_connection(
    state: State<'_, AppState>,
    format: Option<DataFormat>,
) -> Result<bool, String> {
    let now = Utc::now().timestamp();
    let five_minutes_ago = now - 300; // Test with 5 minutes of data
    let format = format.unwrap_or(DataFormat::Raw);
    
    // Test connection with a known good PV over a reasonable time range
    match state
        .client
        .fetch_data(
            vec!["ROOM:LI30:1:OUTSIDE_TEMP".to_string()],
            five_minutes_ago,
            now,
            Some(ProcessingMode::Raw),
            format,
        )
        .await
    {
        Ok(_) => Ok(true),
        Err(e) => {
            println!("Connection test failed: {:?}", e);
            Ok(false)
        }
    }
}

#[tauri::command]
pub fn get_current_timestamp() -> i64 {
    Utc::now().timestamp()
}
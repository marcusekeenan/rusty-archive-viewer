use chrono::Utc;
use serde::Serialize;
use tauri::State;

use crate::client::ArchiverClient;
use crate::types::{Config, PVData, Meta, Point};  // Added Point here

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

#[derive(Debug, Serialize)]
pub struct FetchDataResult {
    pub data: Vec<PVData>,
}

#[tauri::command]
pub async fn fetch_data(
    state: State<'_, AppState>,
    pvs: Vec<String>,
    from: i64,
    to: i64,
) -> Result<Vec<PVData>, String> {
    let mut all_pv_data = Vec::new();

    for pv in pvs {
        match state.client.fetch_data(&pv, from, to).await {
            Ok(pv_data) => all_pv_data.push(pv_data),
            Err(e) => return Err(format!("Error fetching data for {}: {}", pv, e)),
        }
    }

    if all_pv_data.is_empty() {
        Err("No data found for any PV".to_string())
    } else {
        Ok(all_pv_data)
    }
}

#[tauri::command]
pub async fn fetch_latest(
   state: State<'_, AppState>,
   pv: String,
) -> Result<Point, String> {
   let end = Utc::now().timestamp();
   let start = end - 5; // Last 5 seconds

   let pv_data = state.client
       .fetch_data(&pv, start, end)
       .await
       .map_err(|e| e.to_string())?;

   pv_data.data.last()
       .cloned()
       .ok_or_else(|| "No data available".to_string())
}

#[tauri::command]
pub async fn test_connection(state: State<'_, AppState>) -> Result<bool, String> {
    let now = Utc::now().timestamp();
    match state.client
        .fetch_data("ROOM:LI30:1:OUTSIDE_TEMP", now - 60, now)
        .await 
    {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[tauri::command]
pub async fn get_pv_metadata(
    state: State<'_, AppState>,
    pv: String,
) -> Result<Meta, String> {
    state.client
        .get_metadata(&pv)
        .await
        .map_err(|e| e.to_string())
}
// archiver/commands.rs

use super::client::{ArchiverClient, set_archiver_url};
use super::types::*;
use std::time::{Duration, SystemTime};
use tauri::command;

#[command]
pub async fn fetch_archiver_data(
    pv: String,
    from: i64,
    to: i64,
    options: Option<FetchOptions>,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ArchiverClient::new();
    let from_time = SystemTime::UNIX_EPOCH + Duration::from_secs(from as u64);
    let to_time = SystemTime::UNIX_EPOCH + Duration::from_secs(to as u64);

    client
        .fetch_pv_data(&pv, from_time, to_time, options)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub async fn fetch_binned_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    options: Option<FetchOptions>,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ArchiverClient::new();
    let from_time = SystemTime::UNIX_EPOCH + Duration::from_secs(from as u64);
    let to_time = SystemTime::UNIX_EPOCH + Duration::from_secs(to as u64);

    client
        .fetch_binned_data(&pvs, from_time, to_time, options)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub fn set_archiver_url_command(url: String) {
    set_archiver_url(url);
}

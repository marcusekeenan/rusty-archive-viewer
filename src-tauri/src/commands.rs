use crate::archiver_client::{ArchiverClient, FetchOptions, NormalizedPVData};
use std::time::{Duration, SystemTime};
use tauri::command;

#[command]
pub async fn fetch_pv_data_command(
    pv: String,
    from: u64,
    to: u64,
    options: Option<FetchOptions>,
) -> Result<Vec<NormalizedPVData>, String> {
    let client = ArchiverClient::new("http://localhost:17665/retrieval");
    let from_time = SystemTime::UNIX_EPOCH + Duration::from_secs(from);
    let to_time = SystemTime::UNIX_EPOCH + Duration::from_secs(to);

    client
        .fetch_pv_data(&pv, from_time, to_time, options)
        .await
        .map_err(|e| e.to_string())
}

// Add other commands as needed

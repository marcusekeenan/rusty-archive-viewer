// archiver/commands.rs

use super::client::{set_archiver_url, ArchiverClient};
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
    // Create an ArchiverClient instance
    let client = ArchiverClient::new();

    // Convert UNIX timestamps to SystemTime
    let from_time = SystemTime::UNIX_EPOCH + Duration::from_secs(from as u64);
    let to_time = SystemTime::UNIX_EPOCH + Duration::from_secs(to as u64);

    // Fetch the PV data
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
    // Create an ArchiverClient instance
    let client = ArchiverClient::new();

    // Convert UNIX timestamps to SystemTime
    let from_time = SystemTime::UNIX_EPOCH + Duration::from_secs(from as u64);
    let to_time = SystemTime::UNIX_EPOCH + Duration::from_secs(to as u64);

    // Fetch binned data for the PVs
    client
        .fetch_binned_data(&pvs, from_time, to_time, options)
        .await
        .map_err(|e| e.to_string())
}

#[command]
pub fn set_archiver_url_command(url: String) {
    set_archiver_url(url);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_fetch_pv_data_from_archiver() {
        // Set the archiver URL explicitly for testing
        set_archiver_url("http://lcls-archapp.slac.stanford.edu/retrieval/data".to_string());

        let client = ArchiverClient::new();
        println!("Archiver base URL: {}", client.get_base_url());

        let from = SystemTime::now() - Duration::from_secs(600); // 10 minutes ago
        let to = SystemTime::now();

        let result = client
            .fetch_pv_data(
                "ROOM:LI30:1:OUTSIDE_TEMP", // Valid PV
                from,
                to,
                Some(FetchOptions {
                    operator: None, // Use raw data
                }),
            )
            .await;

        assert!(
            result.is_ok(),
            "Failed to fetch PV data: {:?}",
            result.err()
        );

        let data = result.unwrap();
        assert!(
            !data.is_empty(),
            "No data returned for PV in the given time range"
        );

        println!("Received data: {:?}", data);
    }
}

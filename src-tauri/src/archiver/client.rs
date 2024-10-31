// archiver/client.rs

use super::types::*;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use reqwest::Client;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

// In archiver/client.rs
lazy_static! {
    static ref ARCHIVER_URL: Mutex<String> =
        Mutex::new("http://lcls-archapp.slac.stanford.edu/retrieval/data".to_string());
}

pub struct ArchiverClient {
    client: Client,
    base_url: String,
}

impl ArchiverClient {
    /// Creates a new instance of `ArchiverClient` with the current base URL.
    pub fn new() -> Self {
        let base_url = ARCHIVER_URL.lock().unwrap().clone();
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Formats a `SystemTime` into the format expected by the archiver.
    fn format_date_for_archiver(date: SystemTime) -> String {
        let datetime: DateTime<Utc> = date.into();
        datetime.to_rfc3339().replace("Z", "-00:00")
    }

    /// Fetches PV data from the archiver.
    pub async fn fetch_pv_data(
        &self,
        pv: &str,
        from: SystemTime,
        to: SystemTime,
        options: Option<FetchOptions>,
    ) -> Result<Vec<NormalizedPVData>, ArchiverError> {
        let options = options.unwrap_or_default();
        let url = format!("{}/getData.json", self.base_url);

        let duration = to
            .duration_since(from)
            .map_err(ArchiverError::SystemTimeError)?;

        let use_optimized = duration > Duration::from_secs(3600);
        let operator = options.operator.or_else(|| {
            if use_optimized {
                Self::get_operator_for_time_range(duration)
            } else {
                None
            }
        });
        let pv_query = if let Some(op) = operator {
            format!("{}({})", op, pv)
        } else {
            pv.to_string()
        };

        // Print the request URL and parameters for debugging
        println!("Request URL: {}", url);
        println!(
            "Query Params: pv={}, from={}, to={}",
            pv_query,
            Self::format_date_for_archiver(from),
            Self::format_date_for_archiver(to)
        );

        let response = timeout(
            Duration::from_secs(30),
            self.client
                .get(&url)
                .query(&[
                    ("pv", pv_query.as_str()),
                    ("from", &Self::format_date_for_archiver(from)),
                    ("to", &Self::format_date_for_archiver(to)),
                ])
                .send(),
        )
        .await
        .map_err(|_| ArchiverError::Timeout)??;

        if !response.status().is_success() {
            return Err(ArchiverError::HttpError(
                response.error_for_status().unwrap_err(),
            ));
        }

        let pv_data: Vec<PVData> = response.json().await.map_err(|e| {
            ArchiverError::InvalidFormat(format!("Failed to parse response JSON: {}", e))
        })?;
        if pv_data.is_empty() {
            return Err(ArchiverError::InvalidFormat("Empty response".into()));
        }

        // Convert PVData to NormalizedPVData
        let normalized_data = pv_data
            .into_iter()
            .map(|pv_datum| {
                let normalized_points = pv_datum
                    .data
                    .into_iter()
                    .map(|point| point.to_normalized_point())
                    .collect::<Result<Vec<NormalizedPoint>, ArchiverError>>()?;

                Ok(NormalizedPVData {
                    meta: pv_datum.meta,
                    data: normalized_points,
                })
            })
            .collect::<Result<Vec<NormalizedPVData>, ArchiverError>>()?;

        Ok(normalized_data)
    }

    /// Fetches binned data for multiple PVs.
    pub async fn fetch_binned_data(
        &self,
        pvs: &[String],
        from: SystemTime,
        to: SystemTime,
        options: Option<FetchOptions>,
    ) -> Result<Vec<NormalizedPVData>, ArchiverError> {
        if pvs.is_empty() {
            return Err(ArchiverError::NoPVsSpecified);
        }

        // For now, just fetch the first PV
        self.fetch_pv_data(&pvs[0], from, to, options).await
    }

    /// Determines the appropriate operator based on the time range.
    pub fn get_operator_for_time_range(duration: Duration) -> Option<String> {
        match duration.as_secs() {
            d if d <= 900 => None,                            // <= 15 minutes: raw data
            d if d <= 7200 => Some("optimized_720".into()),   // <= 2 hours: ~10s resolution
            d if d <= 21600 => Some("optimized_720".into()),  // <= 6 hours: ~30s resolution
            d if d <= 86400 => Some("optimized_1440".into()), // <= 24 hours: ~1min resolution
            d if d <= 604800 => Some("optimized_2016".into()), // <= 7 days: ~5min resolution
            _ => Some("optimized_4320".into()),               // > 7 days: ~10min resolution
        }
    }
}

/// Sets the base URL for the archiver client.
pub fn set_archiver_url(url: String) {
    let mut archiver_url = ARCHIVER_URL.lock().unwrap();
    *archiver_url = url;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[tokio::test]
    async fn test_fetch_pv_data() {
        // Set the archiver URL explicitly for testing
        set_archiver_url("http://lcls-archapp.slac.stanford.edu/retrieval/data".to_string());

        let client = ArchiverClient::new();
        println!("Archiver base URL: {}", client.base_url);

        let from = SystemTime::now() - Duration::from_secs(3600); // 1 hour ago
        let to = SystemTime::now();

        let result = client
            .fetch_pv_data(
                "ROOM:LI30:1:OUTSIDE_TEMP",
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

    #[test]
    fn test_operator_selection() {
        // Test different time ranges
        assert_eq!(
            ArchiverClient::get_operator_for_time_range(Duration::from_secs(600)),
            None
        );
        assert_eq!(
            ArchiverClient::get_operator_for_time_range(Duration::from_secs(3600)),
            Some("optimized_720".to_string())
        );
        assert_eq!(
            ArchiverClient::get_operator_for_time_range(Duration::from_secs(86400)),
            Some("optimized_1440".to_string())
        );
        assert_eq!(
            ArchiverClient::get_operator_for_time_range(Duration::from_secs(604800)),
            Some("optimized_2016".to_string())
        );
        assert_eq!(
            ArchiverClient::get_operator_for_time_range(Duration::from_secs(864000)),
            Some("optimized_4320".to_string())
        );
    }
}

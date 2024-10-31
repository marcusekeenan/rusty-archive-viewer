// archiver/client.rs

use super::types::*;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use reqwest::Client;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

lazy_static! {
    static ref ARCHIVER_URL: Mutex<String> =
        Mutex::new("http://localhost:17665/retrieval".to_string());
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

    /// Processes a `Point` and extracts a `ProcessedValue` if possible.
    fn process_value(point: &Point) -> Option<ProcessedValue> {
        match &point.val {
            Value::Array(arr) if arr.len() >= 5 => Some(ProcessedValue {
                value: arr[0],         // mean
                stddev: arr[1],        // standard deviation
                min: arr[2],           // minimum
                max: arr[3],           // maximum
                count: arr[4] as i64,  // count
            }),
            Value::Single(val) => Some(ProcessedValue {
                value: *val,
                min: *val,
                max: *val,
                stddev: 0.0,
                count: 1,
            }),
            _ => None,
        }
    }

    /// Normalizes raw PV data into a standard format.
    fn normalize_data(data: Vec<PVData>) -> Vec<NormalizedPVData> {
        data.into_iter()
            .map(|pv_data| {
                let normalized_points = pv_data
                    .data
                    .iter()
                    .filter_map(|point| {
                        let processed = Self::process_value(point)?;
                        let timestamp =
                            point.secs * 1000 + point.nanos.unwrap_or(0) / 1_000_000;

                        Some(NormalizedPoint {
                            timestamp,
                            severity: point.severity.unwrap_or(0),
                            status: point.status.unwrap_or(0),
                            value: processed.value,
                            min: processed.min,
                            max: processed.max,
                            stddev: processed.stddev,
                            count: processed.count,
                        })
                    })
                    .collect();

                NormalizedPVData {
                    meta: pv_data.meta,
                    data: normalized_points,
                }
            })
            .collect()
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

        let raw_data: Vec<PVData> = response.json().await?;
        if raw_data.is_empty() {
            return Err(ArchiverError::InvalidFormat("Empty response".into()));
        }

        Ok(Self::normalize_data(raw_data))
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
        let client = ArchiverClient::new();
        let from = SystemTime::now() - Duration::from_secs(3600);
        let to = SystemTime::now();

        let result = client
            .fetch_pv_data(
                "TEST:PV",
                from,
                to,
                Some(FetchOptions {
                    operator: Some("mean".to_string()),
                }),
            )
            .await;

        assert!(result.is_ok());
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

    #[test]
    fn test_process_value() {
        // Test single value
        let single_point = Point {
            secs: 0,
            nanos: None,
            val: Value::Single(42.0),
            severity: None,
            status: None,
        };
        let processed = ArchiverClient::process_value(&single_point).unwrap();
        assert_eq!(processed.value, 42.0);
        assert_eq!(processed.min, 42.0);
        assert_eq!(processed.max, 42.0);
        assert_eq!(processed.stddev, 0.0);
        assert_eq!(processed.count, 1);

        // Test array value
        let array_point = Point {
            secs: 0,
            nanos: None,
            val: Value::Array(vec![1.0, 0.5, 0.0, 2.0, 10.0]),
            severity: None,
            status: None,
        };
        let processed = ArchiverClient::process_value(&array_point).unwrap();
        assert_eq!(processed.value, 1.0);
        assert_eq!(processed.stddev, 0.5);
        assert_eq!(processed.min, 0.0);
        assert_eq!(processed.max, 2.0);
        assert_eq!(processed.count, 10);
    }
}

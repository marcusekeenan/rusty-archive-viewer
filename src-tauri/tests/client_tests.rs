use chrono::Utc;
use rusty_archive_viewer::{
    Config,
    client::ArchiverClient,
    constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
    types::Meta,
};

// Known good PVs for testing
const TEST_PVS: [&str; 4] = [
    "ROOM:LI30:1:OUTSIDE_TEMP",
    "CPT:PSI5:5205:PRESS",
    "CFT:PSI8:8601:FLOW",
    "CTE:PSI5:5504:TEMP"
];

const INVALID_PV: &str = "INVALID:PV:NAME:123";

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests fetching data for a single PV.
    #[tokio::test]
    async fn test_fetch_data() {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT.as_secs(),
        };
        let client = ArchiverClient::new(config);

        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;

        let result = client.fetch_data(TEST_PVS[0], one_hour_ago, now).await;
        assert!(
            result.is_ok(),
            "Failed to fetch data for {}: {:?}",
            TEST_PVS[0],
            result.err()
        );

        let pv_data = result.unwrap();

        // Print metadata
        println!("\nMetadata for {}:", TEST_PVS[0]);
        for (key, value) in pv_data.meta.0.iter() {
            println!("  {}: {}", key, value);
        }

        // Print sample data points
        println!("\nFirst 5 data points:");
        for point in pv_data.data.iter().take(5) {
            println!(
                "  Time: {}s {}ns, Value: {}, Severity: {}, Status: {}",
                point.secs, point.nanos, point.val, point.severity, point.status
            );
        }

        // Print statistics
        if let (Some(first), Some(last)) = (pv_data.data.first(), pv_data.data.last()) {
            println!("\nSummary:");
            println!("  Total points: {}", pv_data.data.len());
            println!("  Time range: {}s to {}s", first.secs, last.secs);

            // Calculate average sample rate
            let time_span = (last.secs - first.secs) as f64;
            let rate = pv_data.data.len() as f64 / time_span;
            println!("  Average sample rate: {:.2} samples/second", rate);
        }
    }

    /// Tests fetching data for multiple PVs.
    #[tokio::test]
    async fn test_multiple_pvs() {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT.as_secs(),
        };
        let client = ArchiverClient::new(config);

        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;

        println!("\nTesting multiple PVs:");
        for &pv in TEST_PVS.iter() {
            let result = client.fetch_data(pv, one_hour_ago, now).await;
            match result {
                Ok(data) => {
                    println!("\n{}:", pv);
                    println!("  Points: {}", data.data.len());
                    if let Some(point) = data.data.first() {
                        println!("  First value: {}", point.val);
                    }
                    println!(
                        "  EGU: {}",
                        data.meta.0.get("EGU").unwrap_or(&"N/A".to_string())
                    );
                }
                Err(e) => println!("  Error fetching {}: {}", pv, e),
            }
        }
    }

    /// Tests various error conditions for invalid data.
    #[tokio::test]
    async fn test_error_conditions() {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT.as_secs(),
        };
        let client = ArchiverClient::new(config);
        let now = Utc::now().timestamp();

        println!("\nTesting error conditions:");

        // Test 1: Invalid PV
        let result = client.fetch_data(INVALID_PV, now - 3600, now).await;
        println!("\nInvalid PV test:");
        println!("  Result: {:?}", result);
        assert!(result.is_err(), "Expected error for invalid PV");

        // Test 2: Invalid time range
        let result = client.fetch_data(TEST_PVS[0], now, now - 3600).await;
        println!("\nInvalid time range test:");
        println!("  Result: {:?}", result);
        assert!(result.is_err(), "Expected error for invalid time range");

        // Test 3: Future time range
        let result = client.fetch_data(TEST_PVS[0], now + 3600, now + 7200).await;
        println!("\nFuture time range test:");
        println!("  Result: {:?}", result);
        assert!(result.is_err(), "Expected error for future time range");
    }

    /// Tests metadata fetching for all PVs.
    #[tokio::test]
    async fn test_metadata() {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT.as_secs(),
        };
        let client = ArchiverClient::new(config);

        println!("\nTesting metadata for different PV types:");
        for &pv in TEST_PVS.iter() {
            let now = Utc::now().timestamp();
            let result = client.fetch_data(pv, now - 60, now).await;

            if let Ok(data) = result {
                println!("\n{}:", pv);
                println!("  Type-specific metadata:");
                let important_fields = ["EGU", "PREC", "HOPR", "LOPR", "DESC"];
                for field in important_fields {
                    if let Some(value) = data.meta.0.get(field) {
                        println!("    {}: {}", field, value);
                    }
                }
            } else {
                println!("  Error fetching metadata for {}: {:?}", pv, result.err());
            }
        }
    }
}

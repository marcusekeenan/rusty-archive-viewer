use chrono::Utc;
use rusty_archive_viewer::{
    client::ArchiverClient,
    constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
    types::{DataFormat, PVData, ProcessingMode},
    Config,
};

// Known good PVs for testing
const TEST_PVS: [&str; 1] = ["ROOM:LI30:1:OUTSIDE_TEMP"];

const INVALID_PV: &str = "INVALID:PV:NAME:123";

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn setup_client() -> ArchiverClient {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: 60, // Adjust as needed
        };
        ArchiverClient::new(config)
    }

    #[tokio::test]
    async fn test_data_formats() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;
        let pv = TEST_PVS[0];

        println!("\nTesting different data formats for {}:", pv);

        for format in [DataFormat::Raw, DataFormat::Json] {
            let result = client.fetch_data(pv, one_hour_ago, now, format).await;

            match result {
                Ok((data, size)) => {
                    println!("\nFormat {:?}:", format);
                    println!("  Points: {}", data.data.len());
                    println!("  Data size: {} bytes", size);
                    if let Some(first) = data.data.first() {
                        println!("  First value: {}", first.val);
                        println!("  Timestamp: secs={}, nanos={}", first.secs, first.nanos);
                    }
                }
                Err(e) => println!("  Error with format {:?}: {}", format, e),
            }
        }
    }

    #[tokio::test]
    async fn test_historical_parallel_fetch() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;

        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();

        println!("\nTesting parallel fetch for multiple PVs:");

        let fetch_tasks = pvs.iter().map(|pv| {
            let client = client.clone();
            let pv = pv.clone();
            async move {
                client
                    .fetch_data(&pv, one_hour_ago, now, DataFormat::Raw)
                    .await
            }
        });

        let results: Vec<_> = futures::future::join_all(fetch_tasks).await;

        for (i, result) in results.into_iter().enumerate() {
            match result {
                Ok((data, size)) => {
                    println!("\n{}:", TEST_PVS[i]);
                    println!("  Points: {}", data.data.len());
                    println!("  Data size: {} bytes", size);
                    if let Some(point) = data.data.first() {
                        println!("  First value: {}", point.val);
                    }
                }
                Err(e) => println!("  Error fetching data for {}: {}", TEST_PVS[i], e),
            }
        }
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let client = setup_client();
        let now = Utc::now().timestamp();

        let test_cases = [
            (INVALID_PV, now - 3600, now, "invalid PV"),
            (TEST_PVS[0], now, now - 3600, "invalid time range"),
            (TEST_PVS[0], -1000000000, now, "very old timestamp"),
        ];

        println!("\nTesting error conditions:");

        for (pv, start, end, case) in test_cases.iter() {
            println!("\nTesting {}", case);

            let result = client.fetch_data(pv, *start, *end, DataFormat::Raw).await;

            assert!(result.is_err(), "Expected error for {}", case);
            println!("Got expected error: {:?}", result.err());
        }
    }

    #[tokio::test]
    async fn test_metadata() {
        let client = setup_client();

        for pv in TEST_PVS.iter() {
            let result = client.get_metadata(pv).await;

            match result {
                Ok(meta) => {
                    println!("\nMetadata for {}:", pv);
                    println!("{:#?}", meta);
                }
                Err(e) => println!("Error fetching metadata for {}: {}", pv, e),
            }
        }
    }

    #[tokio::test]
    async fn test_performance() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_day_ago = now - 86400;
        let pv = TEST_PVS[0];

        println!("\nPerformance test:");

        let start = Instant::now();
        let result = client
            .fetch_data(pv, one_day_ago, now, DataFormat::Raw)
            .await;
        let duration = start.elapsed();

        match result {
            Ok((data, size)) => {
                println!("  Fetch time: {:?}", duration);
                println!("  Points: {}", data.data.len());
                println!("  Data size: {} bytes", size);
                println!(
                    "  Points per second: {:.2}",
                    data.data.len() as f64 / duration.as_secs_f64()
                );
                println!(
                    "  Bytes per second: {:.2}",
                    size as f64 / duration.as_secs_f64()
                );
            }
            Err(e) => println!("Error in performance test: {}", e),
        }
    }

    #[tokio::test]
    async fn test_live_data() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let five_minutes_ago = now - 300;

        println!("\nTesting live data fetch:");

        for pv in TEST_PVS.iter() {
            let result = client
                .fetch_data(pv, five_minutes_ago, now, DataFormat::Raw)
                .await;

            match result {
                Ok((data, size)) => {
                    println!("\nLive data for {}:", pv);
                    println!("  Points: {}", data.data.len());
                    println!("  Data size: {} bytes", size);
                    if let Some(last) = data.data.last() {
                        println!("  Latest value: {}", last.val);
                        println!(
                            "  Latest timestamp: secs={}, nanos={}",
                            last.secs, last.nanos
                        );
                    }
                }
                Err(e) => println!("Error fetching live data for {}: {}", pv, e),
            }
        }
    }
}

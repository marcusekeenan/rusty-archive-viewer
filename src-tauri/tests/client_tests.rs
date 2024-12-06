use chrono::Utc;
use rusty_archive_viewer::{
    client::ArchiverClient,
    constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
    types::{DataFormat, ProcessingMode, UPlotData},
    Config, Meta,
};

const TEST_PV: &str = "CTE:CM01:2502:B1:TEMP";
const INVALID_PV: &str = "INVALID:PV:NAME:123";

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn setup_client() -> ArchiverClient {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: 60,
        };
        ArchiverClient::new(config)
    }

    #[tokio::test]
    async fn test_basic_modes() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_day_ago = now - 86400;
        let pvs = vec![TEST_PV.to_string()];

        let modes = [
            ProcessingMode::Raw,
            ProcessingMode::Mean,
            ProcessingMode::Max,
            ProcessingMode::Min,
            ProcessingMode::FirstSample,
            ProcessingMode::LastSample,
        ];

        println!("\nTesting basic processing modes over 24 hours:");

        for mode in modes {
            println!("\nTesting mode: {:?}", mode);
            let result = client
                .fetch_data(pvs.clone(), one_day_ago, now, Some(mode), DataFormat::Raw)
                .await;

            match result {
                Ok(data) => {
                    println!("  Points: {}", data.series[0].len());
                    if let Some(first) = data.series[0].first() {
                        println!("  First value: {}", first);
                    }
                }
                Err(e) => println!("Error: {}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_optimized_mode() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let pvs = vec![TEST_PV.to_string()];

        // Test different time ranges for optimization
        let ranges = [
            (now - 3600, "1 hour"),
            (now - 86400, "1 day"),
            (now - 7 * 86400, "1 week"),
        ];

        println!("\nTesting optimized mode with different time ranges:");

        for (start, range_desc) in ranges {
            println!("\nTesting range: {}", range_desc);
            
            // Test both Raw and Json formats
            for format in [DataFormat::Raw, DataFormat::Json] {
                println!("Using format: {:?}", format);
                let result = client
                    .fetch_data(
                        pvs.clone(),
                        start,
                        now,
                        Some(ProcessingMode::Optimized),
                        format,
                    )
                    .await;

                match result {
                    Ok(data) => {
                        println!("  Points returned: {}", data.series[0].len());
                        if let Some(first) = data.series[0].first() {
                            println!("  First value: {}", first);
                        }
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let client = setup_client();
        let now = Utc::now().timestamp();

        let test_cases = [
            (vec![INVALID_PV.to_string()], now - 3600, now, "invalid PV"),
            (
                vec![TEST_PV.to_string()],
                now,
                now - 3600,
                "end time before start time",
            ),
        ];

        println!("\nTesting error conditions:");

        for (pvs, start, end, case) in test_cases.iter() {
            println!("\nTesting {}", case);
            let result = client
                .fetch_data(pvs.clone(), *start, *end, Some(ProcessingMode::Raw), DataFormat::Raw)
                .await;

            assert!(result.is_err(), "Expected error for {}", case);
            println!("Got expected error: {:?}", result.err());
        }
    }

    #[tokio::test]
    async fn test_metadata() {
        let client = setup_client();
        let result = client.get_metadata(TEST_PV).await;

        match result {
            Ok(meta) => {
                println!("\nMetadata for {}:", TEST_PV);
                println!("{:#?}", meta);
            }
            Err(e) => println!("Error fetching metadata: {}", e),
        }
    }

    #[tokio::test]
    async fn test_live_data() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let five_minutes_ago = now - 300;
        let pvs = vec![TEST_PV.to_string()];

        println!("\nTesting live data fetch with Raw mode:");

        let result = client
            .fetch_data(pvs, five_minutes_ago, now, Some(ProcessingMode::Raw), DataFormat::Raw)
            .await;

        match result {
            Ok(data) => {
                println!("  Points: {}", data.series[0].len());
                if let Some(last) = data.series[0].last() {
                    println!("  Latest value: {}", last);
                    println!("  Latest timestamp: {}", data.timestamps.last().unwrap());
                }
            }
            Err(e) => println!("Error fetching live data: {}", e),
        }
    }
}
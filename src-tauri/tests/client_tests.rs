use chrono::Utc;
use rusty_archive_viewer::{
    client::ArchiverClient,
    constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
    types::{UPlotData, ProcessingMode, BinningOperation, DataFormat},
    Config,
};

// Known good PVs for testing
const TEST_PVS: [&str; 4] = [
    "ROOM:LI30:1:OUTSIDE_TEMP",
    "CPT:PSI5:5205:PRESS",
    "CFT:PSI8:8601:FLOW",
    "CTE:PSI5:5504:TEMP",
];

const INVALID_PV: &str = "INVALID:PV:NAME:123";

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn setup_client() -> ArchiverClient {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT.as_secs(),
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
            let result = client
                .fetch_data_with_processing(pv, one_hour_ago, now, ProcessingMode::Raw, format)
                .await;

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
    async fn test_processing_modes() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;
        let pv = TEST_PVS[0];

        let modes = [
            ProcessingMode::Raw,
            ProcessingMode::Optimized(100),
            ProcessingMode::Binning {
                bin_size: 300,
                operation: BinningOperation::Mean,
            },
            ProcessingMode::Binning {
                bin_size: 300,
                operation: BinningOperation::Max,
            },
        ];

        for format in [DataFormat::Raw, DataFormat::Json] {
            println!("\nTesting processing modes with format {:?}:", format);
            
            for mode in modes.iter() {
                let result = client
                    .fetch_data_with_processing(pv, one_hour_ago, now, mode.clone(), format)
                    .await;

                match result {
                    Ok((data, size)) => {
                        println!("\nMode {:?}:", mode);
                        println!("  Points: {}", data.data.len());
                        println!("  Data size: {} bytes", size);
                        if let Some(first) = data.data.first() {
                            println!("  First value: {}", first.val);
                        }
                    }
                    Err(e) => println!("  Error with mode {:?}: {}", mode, e),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_historical_parallel_fetch() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;

        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();

        for format in [DataFormat::Raw, DataFormat::Json] {
            println!("\nTesting parallel fetch with format {:?}:", format);
            
            let result = client
                .fetch_historical_data(pvs.clone(), one_hour_ago, now, ProcessingMode::Raw, format)
                .await;

            match result {
                Ok((data_vec, total_size)) => {
                    println!("Total data size: {} bytes", total_size);
                    for (i, data) in data_vec.iter().enumerate() {
                        println!("\n{}:", TEST_PVS[i]);
                        println!("  Points: {}", data.data.len());
                        if let Some(point) = data.data.first() {
                            println!("  First value: {}", point.val);
                        }
                    }
                }
                Err(e) => println!("Error in parallel fetch: {}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_uplot_data_format() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;

        let pvs: Vec<String> = TEST_PVS.iter().take(2).map(|&s| s.to_string()).collect();
        
        for format in [DataFormat::Raw, DataFormat::Json] {
            println!("\nTesting UPlot conversion with format {:?}:", format);
            
            let result = client
                .fetch_data_uplot(
                    pvs.clone(), 
                    one_hour_ago, 
                    now, 
                    Some(ProcessingMode::Optimized(800)),
                    format
                )
                .await;

            match result {
                Ok((uplot_data, size)) => {
                    println!("UPlot data structure:");
                    println!("  Data size: {} bytes", size);
                    println!("  Timestamp count: {}", uplot_data.timestamps.len());
                    println!("  Series count: {}", uplot_data.series.len());
                    println!("  Metadata count: {}", uplot_data.meta.len());
                    
                    assert_eq!(uplot_data.series.len(), uplot_data.meta.len(), 
                        "Series count should match metadata count");
                    
                    if let Some(series) = uplot_data.series.first() {
                        assert_eq!(series.len(), uplot_data.timestamps.len(),
                            "Series length should match timestamp count");
                    }
                }
                Err(e) => println!("Error fetching UPlot data: {}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_binning_operations() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_day_ago = now - 86400;
        let pv = TEST_PVS[0];

        let operations = [
            BinningOperation::Mean,
            BinningOperation::Max,
            BinningOperation::Min,
            BinningOperation::StdDev,
            BinningOperation::Count,
            BinningOperation::FirstSample,
            BinningOperation::LastSample,
            BinningOperation::Median,
            BinningOperation::CAPlotBinning,
        ];

        for format in [DataFormat::Raw, DataFormat::Json] {
            println!("\nTesting binning operations with format {:?}:", format);
            
            for operation in operations.iter() {
                let mode = ProcessingMode::Binning {
                    bin_size: 3600,
                    operation: operation.clone(),
                };

                let result = client
                    .fetch_data_with_processing(pv, one_day_ago, now, mode, format)
                    .await;

                match result {
                    Ok((data, size)) => {
                        println!("\nBinning operation {:?}:", operation);
                        println!("  Total points: {}", data.data.len());
                        println!("  Data size: {} bytes", size);
                        if let Some(point) = data.data.first() {
                            println!("  Sample value: {}", point.val);
                        }
                    }
                    Err(e) => println!("Error with operation {:?}: {}", operation, e),
                }
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

        for format in [DataFormat::Raw, DataFormat::Json] {
            println!("\nTesting error conditions with format {:?}:", format);
            
            for (pv, start, end, case) in test_cases.iter() {
                println!("\nTesting {}", case);
                
                let result = client
                    .fetch_data_with_processing(pv, *start, *end, ProcessingMode::Raw, format)
                    .await;
                    
                assert!(result.is_err(), "Expected error for {}", case);
                println!("Got expected error: {:?}", result.err());
            }
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
                    for (key, value) in meta.0.iter() {
                        println!("  {}: {}", key, value);
                    }
                    
                    assert!(meta.0.contains_key("name"), "Missing name field");
                    assert!(meta.0.contains_key("EGU"), "Missing engineering units");
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

        for format in [DataFormat::Raw, DataFormat::Json] {
            println!("\nPerformance test with format {:?}:", format);

            let start = Instant::now();
            let result = client
                .fetch_data_with_processing(pv, one_day_ago, now, ProcessingMode::Raw, format)
                .await;
            let duration = start.elapsed();

            match result {
                Ok((data, size)) => {
                    println!("  Fetch time: {:?}", duration);
                    println!("  Points: {}", data.data.len());
                    println!("  Data size: {} bytes", size);
                    println!("  Points per second: {:.2}", data.data.len() as f64 / duration.as_secs_f64());
                    println!("  Bytes per second: {:.2}", size as f64 / duration.as_secs_f64());
                }
                Err(e) => println!("Error in performance test: {}", e),
            }
        }
    }

    #[tokio::test]
    async fn test_live_data() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let five_minutes_ago = now - 300;

        for format in [DataFormat::Raw, DataFormat::Json] {
            println!("\nTesting live data fetch with format {:?}:", format);

            for pv in TEST_PVS.iter() {
                let result = client
                    .fetch_data_with_processing(pv, five_minutes_ago, now, ProcessingMode::Raw, format)
                    .await;

                match result {
                    Ok((data, size)) => {
                        println!("\nLive data for {}:", pv);
                        println!("  Points: {}", data.data.len());
                        println!("  Data size: {} bytes", size);
                        if let Some(last) = data.data.last() {
                            println!("  Latest value: {}", last.val);
                            println!("  Latest timestamp: secs={}, nanos={}", last.secs, last.nanos);
                        }
                    }
                    Err(e) => println!("Error fetching live data for {}: {}", pv, e),
                }
            }
        }
    }
}
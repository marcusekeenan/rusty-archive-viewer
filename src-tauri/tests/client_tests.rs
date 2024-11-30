use chrono::Utc;
use rusty_archive_viewer::{
    client::{ArchiverClient, ProcessingMode},
    constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
    types::Meta,
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

    fn setup_client() -> ArchiverClient {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT.as_secs(),
        };
        ArchiverClient::new(config)
    }

    /// Tests fetching data with different processing modes
    #[tokio::test]
    async fn test_processing_modes() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;
        let pv = TEST_PVS[0];

        println!("\nTesting different processing modes for {}:", pv);

        // Test different processing modes
        let modes = [
            ProcessingMode::Raw,
            ProcessingMode::Optimized(100),
            ProcessingMode::Mean(300), // 5-minute bins
            ProcessingMode::Max(300),
            ProcessingMode::Jitter(300),
        ];

        for mode in modes.iter() {
            let result = client
                .fetch_data_with_processing(pv, one_hour_ago, now, mode)
                .await;

            match result {
                Ok(data) => {
                    println!("\nMode {:?}:", mode);
                    println!("  Points: {}", data.data.len());
                    if let Some(first) = data.data.first() {
                        println!("  First value: {}", first.val);
                    }
                }
                Err(e) => println!("  Error with mode {:?}: {}", mode, e),
            }
        }
    }

    /// Tests parallel fetching for multiple PVs
    #[tokio::test]
    async fn test_historical_parallel_fetch() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_hour_ago = now - 3600;

        println!("\nTesting parallel historical data fetch:");

        let mode = ProcessingMode::Optimized(800);
        // Convert &str array to Vec<String>
        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();

        let result = client
            .fetch_historical_data(pvs, one_hour_ago, now, &mode)
            .await;

        match result {
            Ok(data_vec) => {
                for (pv, data) in TEST_PVS.iter().zip(data_vec.iter()) {
                    println!("\n{}:", pv);
                    println!("  Points: {}", data.data.len());
                    if let Some(point) = data.data.first() {
                        println!("  First value: {}", point.val);
                    }
                }
            }
            Err(e) => println!("Error in parallel fetch: {}", e),
        }
    }

    /// Tests live data fetching
    #[tokio::test]
    async fn test_live_data() {
        let client = setup_client();

        println!("\nTesting live data fetch:");

        // Convert &str array to Vec<String>
        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();

        let result = client.fetch_live_data(pvs, None).await;

        match result {
            Ok(data_vec) => {
                for (pv, data) in TEST_PVS.iter().zip(data_vec.iter()) {
                    println!("\n{}:", pv);
                    println!("  Points: {}", data.data.len());
                    if let Some(point) = data.data.last() {
                        println!("  Latest value: {}", point.val);
                        println!("  Timestamp: {}s {}ns", point.secs, point.nanos);

                        // Add timestamp difference from now
                        let now = Utc::now().timestamp();
                        let diff = now - point.secs;
                        println!("  Age: {}s ago", diff);
                    }
                }
            }
            Err(e) => println!("Error in live fetch: {}", e),
        }
    }
    /// Tests error conditions with different processing modes
    #[tokio::test]
    async fn test_error_conditions() {
        let client = setup_client();
        let now = Utc::now().timestamp();

        println!("\nTesting error conditions with processing modes:");

        let modes = [
            ProcessingMode::Raw,
            ProcessingMode::Optimized(100),
            ProcessingMode::Mean(300),
        ];

        for mode in modes.iter() {
            // Test invalid PV
            let result = client
                .fetch_data_with_processing(INVALID_PV, now - 3600, now, mode)
                .await;
            println!("\nInvalid PV test with mode {:?}:", mode);
            println!("  Result: {:?}", result);
            assert!(result.is_err(), "Expected error for invalid PV");

            // Test invalid time range
            let result = client
                .fetch_data_with_processing(TEST_PVS[0], now, now - 3600, mode)
                .await;
            println!("\nInvalid time range test with mode {:?}:", mode);
            println!("  Result: {:?}", result);
            assert!(result.is_err(), "Expected error for invalid time range");
        }
    }

    #[tokio::test]
    async fn test_binned_data() {
        let client = setup_client();
        let now = Utc::now().timestamp();
        let one_day_ago = now - 86400; // 24 hours
        let pv = TEST_PVS[0].to_string();

        println!("\nTesting binned data for {}:", pv);

        let bin_sizes = [300_u32, 900_u32, 3600_u32]; // 5 min, 15 min, 1 hour

        let modes = [
            ProcessingMode::Mean(0),
            ProcessingMode::Max(0),
            ProcessingMode::Min(0),
            ProcessingMode::StdDev(0),
        ];

        for bin_size in bin_sizes.iter() {
            for mode_type in modes.iter() {
                let mode = match mode_type {
                    ProcessingMode::Mean(_) => ProcessingMode::Mean(*bin_size),
                    ProcessingMode::Max(_) => ProcessingMode::Max(*bin_size),
                    ProcessingMode::Min(_) => ProcessingMode::Min(*bin_size),
                    ProcessingMode::StdDev(_) => ProcessingMode::StdDev(*bin_size),
                    _ => continue,
                };

                let result = client
                    .fetch_data_with_processing(&pv, one_day_ago, now, &mode)
                    .await;

                match result {
                    Ok(data) => {
                        println!("\nBin size: {}s, Mode: {:?}", bin_size, mode);
                        println!("  Points: {}", data.data.len());
                        if let Some(point) = data.data.first() {
                            println!("  First value: {}", point.val);
                        }

                        // Convert bin_size to i64 for calculation
                        let expected_bins = (now - one_day_ago) / (*bin_size as i64);
                        println!("  Expected bins: {}", expected_bins);
                        println!("  Actual data points: {}", data.data.len());
                    }
                    Err(e) => println!(
                        "  Error with bin size {}s, mode {:?}: {}",
                        bin_size, mode, e
                    ),
                }
            }
        }
    }

    #[tokio::test]
async fn test_metadata_consistency() {
    let client = setup_client();
    let now = Utc::now().timestamp();
    let one_hour_ago = now - 3600;
    let pv = TEST_PVS[0].to_string();

    println!("\nTesting metadata consistency across processing modes for {}:", pv);

    let modes = [
        ProcessingMode::Raw,
        ProcessingMode::Optimized(100),
        ProcessingMode::Mean(300),
    ];

    // All metadata fields we care about
    let important_fields = [
        "name", "DRVH", "EGU", "HIGH", "HIHI", "DRVL", 
        "PREC", "LOW", "LOLO", "LOPR", "HOPR", "NELM", "DESC"
    ];

    let mut base_metadata: Option<Meta> = None;

    for mode in modes.iter() {
        let result = client
            .fetch_data_with_processing(&pv, one_hour_ago, now, mode)
            .await;

        match result {
            Ok(data) => {
                println!("\nMode {:?}:", mode);
                
                if let Some(ref base) = base_metadata {
                    println!("Comparing metadata with baseline:");
                    
                    for field in important_fields.iter() {
                        let current = data.meta.0.get(*field);
                        let baseline = base.0.get(*field);
                        
                        // Print values for debugging
                        println!("  Field '{}' comparison:", field);
                        println!("    Base:    {:?}", baseline);
                        println!("    Current: {:?}", current);
                        
                        // Skip comparison for fields that might legitimately change
                        if *field != "name" {  // name might include processing info
                            if current != baseline {
                                println!("WARNING: Mismatch in field '{}' for mode {:?}", field, mode);
                                println!("  Expected: {:?}", baseline);
                                println!("  Got:      {:?}", current);
                            }
                        }
                    }
                } else {
                    println!("Establishing baseline metadata with {:?} mode", mode);
                    println!("Available fields:");
                    for (key, value) in data.meta.0.iter() {
                        println!("  {}: {:?}", key, value);
                    }
                    base_metadata = Some(data.meta);
                }

                // Print sample data point to verify value format
                if let Some(point) = data.data.first() {
                    println!("\nSample data point:");
                    println!("  Timestamp: {}s {}ns", point.secs, point.nanos);
                    println!("  Value: {:?}", point.val);
                    println!("  Severity: {}", point.severity);
                    println!("  Status: {}", point.status);
                }
            }
            Err(e) => {
                println!("Error fetching data with mode {:?}: {}", mode, e);
                println!("Continuing to next mode...");
                continue;
            }
        }
    }

    // Ensure we got some metadata
    assert!(base_metadata.is_some(), "No metadata was retrieved during test");
}
}

use chrono::Utc;
use std::time::Instant;
use rusty_archive_viewer::{
    Config,
    client::{ArchiverClient, ProcessingMode},
    constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
};

const TEST_PVS: [&str; 17] = [
    "CTE:CM16:2502:B1:TEMP",
    "CTE:CM17:2502:B1:TEMP",
    "CTE:CM18:2502:B1:TEMP",
    "CTE:CM19:2502:B1:TEMP",
    "CTE:CM20:2502:B1:TEMP",
    "CTE:CM21:2502:B1:TEMP",
    "CTE:CM22:2502:B1:TEMP",
    "CTE:CM23:2502:B1:TEMP",
    "CTE:CM24:2502:B1:TEMP",
    "CTE:CM25:2502:B1:TEMP",
    "CTE:CM26:2502:B1:TEMP",
    "CTE:CM27:2502:B1:TEMP",
    "CTE:CM28:2502:B1:TEMP",
    "CTE:CM29:2502:B1:TEMP",
    "CTE:CM30:2502:B1:TEMP",
    "CTE:CM31:2502:B1:TEMP",
    "CTE:CM32:2502:B1:TEMP",
];

// Time ranges for testing
const ONE_HOUR: i64 = 3600;
const ONE_DAY: i64 = 86400;
const ONE_WEEK: i64 = 604800;
const ONE_MONTH: i64 = 2592000;

#[cfg(test)]
mod performance_tests {
    use super::*;

    fn setup_client() -> ArchiverClient {
        let config = Config {
            url: DEFAULT_BASE_URL.to_string(),
            timeout_secs: DEFAULT_TIMEOUT.as_secs(),
        };
        ArchiverClient::new(config)
    }

    async fn measure_fetch_time(
        client: &ArchiverClient,
        pvs: &[String],
        duration: i64,
        mode: &ProcessingMode,
    ) -> (usize, f64, Vec<usize>) {
        let now = Utc::now().timestamp();
        let start_time = now - duration;
        
        let start = Instant::now();
        let result = client.fetch_historical_data(pvs.to_vec(), start_time, now, mode).await;
        let elapsed = start.elapsed().as_secs_f64();

        match result {
            Ok(data) => {
                let point_counts: Vec<usize> = data.iter()
                    .map(|pv_data| pv_data.data.len())
                    .collect();
                let total_points: usize = point_counts.iter().sum();
                (total_points, elapsed, point_counts)
            },
            Err(e) => {
                println!("Error in measurement: {}", e);
                (0, elapsed, vec![0; pvs.len()])
            }
        }
    }

    #[tokio::test]
    async fn benchmark_time_ranges() {
        let client = setup_client();
        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();
        
        let time_ranges = [
            ("1 hour", ONE_HOUR),
            ("1 day", ONE_DAY),
            ("1 week", ONE_WEEK),
            ("1 month", ONE_MONTH),
        ];

        println!("\nBenchmarking different time ranges:");
        for (range_name, duration) in time_ranges.iter() {
            let mode = ProcessingMode::Raw;
            let (total_points, elapsed, point_counts) = measure_fetch_time(
                &client, 
                &pvs, 
                *duration, 
                &mode
            ).await;

            println!("\n{} fetch:", range_name);
            println!("  Total time: {:.3}s", elapsed);
            println!("  Total points: {}", total_points);
            println!("  Points per second: {:.2}", total_points as f64 / elapsed);
            println!("  Points per PV: {:?}", point_counts);
        }
    }

    #[tokio::test]
    async fn benchmark_processing_modes() {
        let client = setup_client();
        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();
        let duration = ONE_DAY;

        let modes = [
            ProcessingMode::Raw,
            ProcessingMode::Optimized(1000),
            ProcessingMode::Mean(300),
            ProcessingMode::Max(300),
            ProcessingMode::Jitter(300),
        ];

        println!("\nBenchmarking different processing modes (24h of data):");
        for mode in modes.iter() {
            let (total_points, elapsed, point_counts) = measure_fetch_time(
                &client,
                &pvs,
                duration,
                mode
            ).await;

            println!("\nMode {:?}:", mode);
            println!("  Total time: {:.3}s", elapsed);
            println!("  Total points: {}", total_points);
            println!("  Points per second: {:.2}", total_points as f64 / elapsed);
            println!("  Points per PV: {:?}", point_counts);
        }
    }

    #[tokio::test]
    async fn benchmark_concurrent_requests() {
        let client = setup_client();
        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();
        let duration = ONE_HOUR;
        let mode = ProcessingMode::Raw;

        println!("\nBenchmarking concurrent vs sequential requests (1h of data):");

        // Sequential requests
        let start = Instant::now();
        let mut total_points = 0;
        for pv in pvs.iter() {
            if let Ok(data) = client
                .fetch_data_with_processing(pv, Utc::now().timestamp() - duration, Utc::now().timestamp(), &mode)
                .await 
            {
                total_points += data.data.len();
            }
        }
        let sequential_time = start.elapsed().as_secs_f64();

        // Concurrent requests
        let start = Instant::now();
        let (concurrent_points, _, _) = measure_fetch_time(&client, &pvs, duration, &mode).await;
        let concurrent_time = start.elapsed().as_secs_f64();

        println!("\nSequential requests:");
        println!("  Total time: {:.3}s", sequential_time);
        println!("  Total points: {}", total_points);
        println!("  Points per second: {:.2}", total_points as f64 / sequential_time);

        println!("\nConcurrent requests:");
        println!("  Total time: {:.3}s", concurrent_time);
        println!("  Total points: {}", concurrent_points);
        println!("  Points per second: {:.2}", concurrent_points as f64 / concurrent_time);
        println!("\nSpeedup factor: {:.2}x", sequential_time / concurrent_time);
    }

    #[tokio::test]
    async fn benchmark_live_data_latency() {
        let client = setup_client();
        let pvs: Vec<String> = TEST_PVS.iter().map(|&s| s.to_string()).collect();
        
        println!("\nMeasuring live data latency:");
        
        let iterations = 10;
        let mut latencies = Vec::with_capacity(iterations);
        
        for i in 1..=iterations {
            let request_time = Instant::now();
            let result = client.fetch_live_data(pvs.clone(), None).await;
            let response_time = request_time.elapsed().as_secs_f64();
            
            match result {
                Ok(data) => {
                    let now = Utc::now().timestamp();
                    let data_timestamps: Vec<_> = data.iter()
                        .filter_map(|pv| pv.data.last())
                        .map(|point| now - point.secs)
                        .collect();
                    
                    println!("\nIteration {}:", i);
                    println!("  Request-response time: {:.3}s", response_time);
                    println!("  Data ages (seconds): {:?}", data_timestamps);
                    
                    latencies.push(response_time);
                }
                Err(e) => println!("Error in iteration {}: {}", i, e),
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        
        if !latencies.is_empty() {
            let avg_latency: f64 = latencies.iter().sum::<f64>() / latencies.len() as f64;
            let max_latency = latencies.iter().fold(0f64, |a, &b| a.max(b));
            let min_latency = latencies.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            println!("\nLatency Statistics (seconds):");
            println!("  Average: {:.3}", avg_latency);
            println!("  Minimum: {:.3}", min_latency);
            println!("  Maximum: {:.3}", max_latency);
        }
    }
}
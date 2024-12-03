use chrono::{DateTime, Utc};
use rusty_archive_viewer::{
    client::ArchiverClient,
    constants::{DEFAULT_BASE_URL, DEFAULT_TIMEOUT},
    types::{DataFormat, PVData, ProcessingMode},
    Config,
};
use serde::Serialize;
use std::{error::Error, time::Instant};
use tokio::time::{sleep, Duration};

const TEST_PVS: [&str; 35] = [
    "CTE:CM01:2502:B1:TEMP",
    "CTE:CM02:2502:B1:TEMP",
    "CTE:CM03:2502:B1:TEMP",
    "CTE:CMH1:2502:B1:TEMP",
    "CTE:CMH2:2502:B1:TEMP",
    "CTE:CM04:2502:B1:TEMP",
    "CTE:CM05:2502:B1:TEMP",
    "CTE:CM06:2502:B1:TEMP",
    "CTE:CM07:2502:B1:TEMP",
    "CTE:CM08:2502:B1:TEMP",
    "CTE:CM09:2502:B1:TEMP",
    "CTE:CM10:2502:B1:TEMP",
    "CTE:CM11:2502:B1:TEMP",
    "CTE:CM12:2502:B1:TEMP",
    "CTE:CM13:2502:B1:TEMP",
    "CTE:CM14:2502:B1:TEMP",
    "CTE:CM15:2502:B1:TEMP",
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
    "CTE:CM33:2502:B1:TEMP",
];

const PV_GROUPS: [usize; 5] = [1, 5, 10, 20, 35];
const ITERATIONS: usize = 10;
const LIVE_DATA_DURATION: i64 = 60; // 1 minute of live data
const HISTORICAL_TIME_RANGES: [(i64, &str); 3] =
    [(60, "1 minute"), (300, "5 minutes"), (3600, "1 hour")];

#[derive(Debug, Serialize)]
struct PerformanceReport {
    timestamp: DateTime<Utc>,
    live_data_results: Vec<LiveDataResult>,
    historical_data_results: Vec<HistoricalDataResult>,
}

#[derive(Debug, Serialize)]
struct LiveDataResult {
    pv_count: usize,
    raw_metrics: FormatMetrics,
    json_metrics: FormatMetrics,
    comparison: ComparisonMetrics,
}

#[derive(Debug, Serialize)]
struct HistoricalDataResult {
    pv_count: usize,
    time_range: String,
    raw_metrics: FormatMetrics,
    json_metrics: FormatMetrics,
    comparison: ComparisonMetrics,
}

#[derive(Debug, Serialize)]
struct FormatMetrics {
    average_fetch_time: f64,
    average_processing_time: f64,
    average_total_time: f64,
    average_response_size: usize,
    average_points_count: usize,
    average_points_per_second: f64,
    average_bytes_per_point: f64,
}

#[derive(Debug, Serialize)]
struct ComparisonMetrics {
    speedup: f64,
    size_ratio: f64,
}

impl PerformanceReport {
    fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            live_data_results: Vec::new(),
            historical_data_results: Vec::new(),
        }
    }

    fn save(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

async fn run_performance_test(
    client: &ArchiverClient,
    pvs: &[String],
    start: i64,
    end: i64,
    format: DataFormat,
    iterations: usize,
) -> Result<FormatMetrics, Box<dyn Error>> {
    let mut total_fetch_time = 0.0;
    let mut total_processing_time = 0.0;
    let mut total_response_size = 0;
    let mut total_points_count = 0;

    for _ in 0..iterations {
        let fetch_start = Instant::now();
        let (result, size) = client
            .fetch_historical_data(pvs.to_vec(), start, end, ProcessingMode::Raw, format)
            .await?;
        let fetch_time = fetch_start.elapsed().as_secs_f64();

        let processing_start = Instant::now();
        let _processed_result: Vec<PVData> = result.into_iter().map(PVData::from).collect();
        let processing_time = processing_start.elapsed().as_secs_f64();

        total_fetch_time += fetch_time;
        total_processing_time += processing_time;
        total_response_size += size;
        total_points_count += _processed_result
            .iter()
            .map(|pv| pv.data.len())
            .sum::<usize>();
    }

    let average_fetch_time = total_fetch_time / iterations as f64;
    let average_processing_time = total_processing_time / iterations as f64;
    let average_total_time = average_fetch_time + average_processing_time;
    let average_response_size = total_response_size / iterations;
    let average_points_count = total_points_count / iterations;

    Ok(FormatMetrics {
        average_fetch_time,
        average_processing_time,
        average_total_time,
        average_response_size,
        average_points_count,
        average_points_per_second: average_points_count as f64 / average_total_time,
        average_bytes_per_point: average_response_size as f64 / average_points_count as f64,
    })
}

#[tokio::test]
async fn comprehensive_performance_analysis() -> Result<(), Box<dyn Error>> {
    let config = Config {
        url: DEFAULT_BASE_URL.to_string(),
        timeout_secs: DEFAULT_TIMEOUT.as_secs(),
    };
    let client = ArchiverClient::new(config);
    let mut report = PerformanceReport::new();

    // Live data performance test
    for &pv_count in &PV_GROUPS {
        let pvs: Vec<String> = TEST_PVS
            .iter()
            .take(pv_count)
            .map(|&s| s.to_string())
            .collect();

        println!("Testing live data for {} PVs", pv_count);

        let end = Utc::now().timestamp();
        let start = end - LIVE_DATA_DURATION;

        let raw_metrics =
            run_performance_test(&client, &pvs, start, end, DataFormat::Raw, ITERATIONS).await?;
        let json_metrics =
            run_performance_test(&client, &pvs, start, end, DataFormat::Json, ITERATIONS).await?;

        let speedup = json_metrics.average_total_time / raw_metrics.average_total_time;
        let size_ratio = raw_metrics.average_bytes_per_point / json_metrics.average_bytes_per_point;

        report.live_data_results.push(LiveDataResult {
            pv_count,
            raw_metrics,
            json_metrics,
            comparison: ComparisonMetrics {
                speedup,
                size_ratio,
            },
        });

        println!(
            "Live data: Raw format is {:.2}x faster with {:.2}x size efficiency",
            speedup, size_ratio
        );

        // Allow some time between tests to avoid overwhelming the server
        sleep(Duration::from_secs(1)).await;
    }

    // Historical data performance test
    for &pv_count in &PV_GROUPS {
        let pvs: Vec<String> = TEST_PVS
            .iter()
            .take(pv_count)
            .map(|&s| s.to_string())
            .collect();

        for &(duration, range_name) in HISTORICAL_TIME_RANGES.iter() {
            println!(
                "Testing {} PVs for {} historical range",
                pv_count, range_name
            );

            let end = Utc::now().timestamp();
            let start = end - duration;

            let raw_metrics =
                run_performance_test(&client, &pvs, start, end, DataFormat::Raw, ITERATIONS)
                    .await?;
            let json_metrics =
                run_performance_test(&client, &pvs, start, end, DataFormat::Json, ITERATIONS)
                    .await?;

            let speedup = json_metrics.average_total_time / raw_metrics.average_total_time;
            let size_ratio =
                raw_metrics.average_bytes_per_point / json_metrics.average_bytes_per_point;

            report.historical_data_results.push(HistoricalDataResult {
                pv_count,
                time_range: range_name.to_string(),
                raw_metrics,
                json_metrics,
                comparison: ComparisonMetrics {
                    speedup,
                    size_ratio,
                },
            });

            println!(
                "Historical data ({}): Raw format is {:.2}x faster with {:.2}x size efficiency",
                range_name, speedup, size_ratio
            );

            // Allow some time between tests to avoid overwhelming the server
            sleep(Duration::from_secs(1)).await;
        }
    }

    report.save("comprehensive_performance_report.json")?;
    Ok(())
}

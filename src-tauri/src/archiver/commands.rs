use crate::archiver::constants::{API_CONFIG, ERRORS};
use crate::archiver::types::{
    Meta, 
    NormalizedPVData, 
    Point, 
    ProcessedPoint, 
    PVData, 
    PVStatus,
    Value,
};
use chrono::{TimeZone, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct ExtendedFetchOptions {
    pub operator: Option<String>,
    pub timezone: Option<String>,
    pub chart_width: Option<i32>,
}

#[derive(Debug)]
struct ProcessedValue {
    value: f64,
    min: f64,
    max: f64,
    stddev: f64,
    count: i64,
}

fn create_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_millis(API_CONFIG.timeouts_ms.default))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| {
            println!("Client build error: {}", e);
            e.to_string()
        })
}

fn format_date_for_archiver(timestamp_ms: i64) -> Option<String> {
    println!("Formatting timestamp: {}", timestamp_ms);
    Utc.timestamp_millis_opt(timestamp_ms)
        .single()
        .map(|dt| dt.to_rfc3339().replace("Z", "-00:00"))
}

fn process_point(point: &Point) -> Option<ProcessedValue> {
    match &point.val {
        Value::Array(arr) if arr.len() >= 5 => Some(ProcessedValue {
            value: arr[0],
            stddev: arr[1],
            min: arr[2],
            max: arr[3],
            count: arr[4] as i64,
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

fn normalize_data(pv_data: &PVData) -> NormalizedPVData {
    let normalized_points = pv_data
        .data
        .iter()
        .filter_map(|point| {
            process_point(point).map(|processed| ProcessedPoint {
                timestamp: point.secs * 1000 + point.nanos.unwrap_or(0) / 1_000_000,
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
        meta: pv_data.meta.clone(),
        data: normalized_points,
    }
}

#[tauri::command]
pub async fn fetch_pv_data(
    pv: String,
    from: i64,
    to: i64,
    options: Option<ExtendedFetchOptions>,
) -> Result<NormalizedPVData, String> {
    println!("Fetching PV data: {}, from: {}, to: {}", pv, from, to);
    let client = create_client()?;

    let from_formatted = format_date_for_archiver(from * 1000)
        .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
    let to_formatted = format_date_for_archiver(to * 1000)
        .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

    let duration_seconds = (to - from);
    let use_optimized = duration_seconds > 3600;

    let pv_query = if use_optimized {
        if let Some(opts) = &options {
            if let Some(operator) = &opts.operator {
                format!("{}({})", operator, pv)
            } else {
                pv.clone()
            }
        } else {
            pv.clone()
        }
    } else {
        pv.clone()
    };

    println!("Request URL: {}/getData.json", API_CONFIG.base_url);
    println!("Query params: pv={}, from={}, to={}", pv_query, from_formatted, to_formatted);

    let response = client
        .get(&format!("{}/getData.json", API_CONFIG.base_url))
        .query(&[
            ("pv", &pv_query),
            ("from", &from_formatted),
            ("to", &to_formatted),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    println!("Response status: {}", response.status());

    if !response.status().is_success() {
        return Err(format!("{}: {}", ERRORS.server_error, response.status()));
    }

    let raw_data: Vec<PVData> = response.json().await.map_err(|e| e.to_string())?;
    if raw_data.is_empty() {
        return Err(ERRORS.no_data.to_string());
    }

    Ok(normalize_data(&raw_data[0]))
}

#[tauri::command]
pub async fn fetch_binned_data(
    pvs: Vec<String>,
    from: i64,
    to: i64,
    options: Option<ExtendedFetchOptions>,
) -> Result<Vec<NormalizedPVData>, String> {
    println!("Fetching binned data: {:?}, from: {}, to: {}", pvs, from, to);
    
    if pvs.is_empty() {
        return Err("No PVs specified".to_string());
    }

    let client = create_client()?;

    let from_formatted = format_date_for_archiver(from * 1000)
        .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;
    let to_formatted = format_date_for_archiver(to * 1000)
        .ok_or_else(|| ERRORS.invalid_timerange.to_string())?;

    let duration_seconds = (to - from);
    let use_optimized = duration_seconds > 3600;

    let mut results = Vec::new();
    for pv in pvs {
        let pv_query = if use_optimized {
            if let Some(opts) = &options {
                if let Some(operator) = &opts.operator {
                    format!("{}({})", operator, pv)
                } else {
                    pv.clone()
                }
            } else {
                pv.clone()
            }
        } else {
            pv.clone()
        };

        println!("Querying PV: {}", pv_query);
        let request_url = format!("{}/getData.json", API_CONFIG.base_url);
        println!("Request URL: {}", request_url);
        
        let response = client
            .get(&request_url)
            .query(&[
                ("pv", &pv_query),
                ("from", &from_formatted),
                ("to", &to_formatted),
            ])
            .send()
            .await
            .map_err(|e| e.to_string())?;

        println!("Response status: {}", response.status());

        if !response.status().is_success() {
            return Err(format!("{}: {}", ERRORS.server_error, response.status()));
        }

        let raw_data: Vec<PVData> = response.json().await.map_err(|e| e.to_string())?;
        if !raw_data.is_empty() {
            results.push(normalize_data(&raw_data[0]));
        }
    }

    if results.is_empty() {
        println!("No data in results");
        return Err(ERRORS.no_data.to_string());
    }

    println!("Returning {} results", results.len());
    Ok(results)
}

#[tauri::command]
pub async fn get_pv_metadata(pv: String) -> Result<Meta, String> {
    let client = create_client()?;
    let request_url = format!("{}/getMetadata", API_CONFIG.base_url);
    
    let response = client
        .get(&request_url)
        .query(&[("pv", &pv)])
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("{}: {}", ERRORS.server_error, response.status()));
    }

    let metadata: Meta = response.json().await.map_err(|e| e.to_string())?;
    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archiver::types::Value;

    #[test]
    fn test_process_point() {
        // Test single value
        let point = Point {
            secs: 1234567890,
            nanos: Some(0),
            val: Value::Single(42.0),
            severity: Some(0),
            status: Some(0),
        };
        let processed = process_point(&point).unwrap();
        assert_eq!(processed.value, 42.0);
        assert_eq!(processed.min, 42.0);
        assert_eq!(processed.max, 42.0);
        assert_eq!(processed.stddev, 0.0);
        assert_eq!(processed.count, 1);

        // Test array value
        let point = Point {
            secs: 1234567890,
            nanos: Some(0),
            val: Value::Array(vec![10.0, 2.0, 8.0, 12.0, 5.0]),
            severity: Some(0),
            status: Some(0),
        };
        let processed = process_point(&point).unwrap();
        assert_eq!(processed.value, 10.0);
        assert_eq!(processed.min, 8.0);
        assert_eq!(processed.max, 12.0);
        assert_eq!(processed.stddev, 2.0);
        assert_eq!(processed.count, 5);
    }

    #[test]
    fn test_format_date_for_archiver() {
        let timestamp = 1609459200000; // 2021-01-01 00:00:00 UTC
        let formatted = format_date_for_archiver(timestamp).unwrap();
        assert_eq!(formatted, "2021-01-01T00:00:00.000+00:00");
    }
}
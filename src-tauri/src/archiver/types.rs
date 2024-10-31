// archiver/types.rs

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchiverError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),
    #[error("No PVs specified")]
    NoPVsSpecified,
    #[error("Timeout error")]
    Timeout,
    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedValue {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub stddev: f64,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Single(f64),
    Array(Vec<f64>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Point {
    pub secs: i64,
    pub nanos: Option<i64>,
    pub val: Value,
    pub severity: Option<i32>,
    pub status: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub egu: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PVData {
    pub meta: Meta,
    pub data: Vec<Point>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NormalizedPoint {
    pub timestamp: i64,
    pub severity: i32,
    pub status: i32,
    pub value: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub stddev: Option<f64>,
    pub count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NormalizedPVData {
    pub meta: Meta,
    pub data: Vec<NormalizedPoint>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FetchOptions {
    pub operator: Option<String>,
}

// Place the impl block here
impl Point {
    pub fn to_normalized_point(&self) -> Result<NormalizedPoint, ArchiverError> {
        // Convert the timestamp from secs and nanos to milliseconds
        let timestamp = self.secs * 1000 + self.nanos.unwrap_or(0) / 1_000_000;

        // Extract severity and status, defaulting to 0 if None
        let severity = self.severity.unwrap_or(0);
        let status = self.status.unwrap_or(0);

        match &self.val {
            Value::Single(value) => {
                // Raw data case
                Ok(NormalizedPoint {
                    timestamp,
                    severity,
                    status,
                    value: *value,
                    min: None,
                    max: None,
                    stddev: None,
                    count: None,
                })
            }
            Value::Array(values) => {
                // Statistical data case
                if values.len() == 5 {
                    // [mean, stddev, min, max, count]
                    let mean = values[0];
                    let stddev = values[1];
                    let min = values[2];
                    let max = values[3];
                    let count = values[4];
                    Ok(NormalizedPoint {
                        timestamp,
                        severity,
                        status,
                        value: mean,
                        min: Some(min),
                        max: Some(max),
                        stddev: Some(stddev),
                        count: Some(count as i64),
                    })
                } else {
                    Err(ArchiverError::InvalidFormat(format!(
                        "Expected array of 5 elements for statistical data, got {} elements",
                        values.len()
                    )))
                }
            }
        }
    }
}

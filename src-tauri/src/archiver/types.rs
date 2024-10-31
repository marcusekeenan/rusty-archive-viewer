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
pub struct Point {
    pub secs: i64,
    pub nanos: Option<i64>,
    pub val: Value,
    pub severity: Option<i32>,
    pub status: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Single(f64),
    Array(Vec<f64>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub EGU: String,
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
    pub min: f64,
    pub max: f64,
    pub stddev: f64,
    pub count: i64,
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

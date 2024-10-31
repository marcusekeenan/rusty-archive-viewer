use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PVData {
    pub meta: Meta,
    pub data: Vec<Point>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub name: String,
    #[serde(default)]
    pub EGU: String,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedPoint {
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
    pub data: Vec<ProcessedPoint>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct FetchOptions {
    pub operator: Option<String>,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct PVStatus {
    pub name: String,
    pub connected: bool,
    pub last_event_time: Option<i64>,
    pub last_status: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operator {
    pub name: String,
    pub description: String,
    pub requires_param: bool,
    pub params: Option<Vec<String>>,
}

// ... rest of your existing types ...
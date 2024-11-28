use serde_json::Value as JsonValue;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta(pub HashMap<String, String>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub secs: i64,
    pub nanos: i32,
    pub val: JsonValue,
    pub severity: i32,
    pub status: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVData {
    pub meta: Meta,
    pub data: Vec<Point>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Decode error: {0}")]
    Decode(String),
    #[error("Invalid data: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub timeout_secs: u64,
}

// Implement Default for Config if needed
impl Default for Config {
    fn default() -> Self {
        Self {
            url: crate::constants::DEFAULT_BASE_URL.to_string(),
            timeout_secs: crate::constants::DEFAULT_TIMEOUT.as_secs(),
        }
    }
}
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DataFormat {
    Raw,   // Protocol Buffer format
    Json,  // JSON format
}

impl Default for DataFormat {
    fn default() -> Self {
        DataFormat::Raw
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UPlotData {
    pub timestamps: Vec<f64>,
    pub series: Vec<Vec<f64>>,
    pub meta: Vec<Meta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingMode {
    Raw,
    Optimized(usize),
    Binning {
        bin_size: u32,
        operation: BinningOperation,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinningOperation {
    Mean, Max, Min, Jitter, StdDev, Count,
    FirstSample, LastSample, FirstFill, LastFill,
    Median, Variance, PopVariance,
    Kurtosis, Skewness, Linear, Loess,
    CAPlotBinning,
}

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

impl Default for Config {
    fn default() -> Self {
        Self {
            url: crate::constants::DEFAULT_BASE_URL.to_string(),
            timeout_secs: crate::constants::DEFAULT_TIMEOUT.as_secs(),
        }
    }
}

impl fmt::Display for BinningOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mean => write!(f, "mean"),
            Self::Max => write!(f, "max"),
            Self::Min => write!(f, "min"),
            Self::Jitter => write!(f, "jitter"),
            Self::StdDev => write!(f, "std"),
            Self::Count => write!(f, "count"),
            Self::FirstSample => write!(f, "firstSample"),
            Self::LastSample => write!(f, "lastSample"),
            Self::FirstFill => write!(f, "firstFill"),
            Self::LastFill => write!(f, "lastFill"),
            Self::Median => write!(f, "median"),
            Self::Variance => write!(f, "variance"),
            Self::PopVariance => write!(f, "popvariance"),
            Self::Kurtosis => write!(f, "kurtosis"),
            Self::Skewness => write!(f, "skewness"),
            Self::Linear => write!(f, "linear"),
            Self::Loess => write!(f, "loess"),
            Self::CAPlotBinning => write!(f, "caplotbinning"),
        }
    }
}

impl fmt::Display for ProcessingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessingMode::Raw => write!(f, ""),
            ProcessingMode::Optimized(points) => write!(f, "optimized({})", points),
            ProcessingMode::Binning { bin_size, operation } => match operation {
                BinningOperation::CAPlotBinning => write!(f, "caplot"),
                _ => write!(f, "{}_{}", operation.to_string().to_lowercase(), bin_size)
            },
        }
    }
}

impl ProcessingMode {
    pub fn determine_optimal(start: i64, end: i64) -> Self {
        let duration = end - start;
        match duration {
            d if d < 86400 => ProcessingMode::Raw,
            d if d < 604800 => ProcessingMode::Optimized(2000),
            d if d < 2592000 => ProcessingMode::Optimized(3000),
            _ => ProcessingMode::Optimized(4000),
        }
    }
}
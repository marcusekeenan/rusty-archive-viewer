use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

// ===== Basic Types =====

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DataFormat {
    Raw,
    Json,
}

impl Default for DataFormat {
    fn default() -> Self {
        DataFormat::Raw
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingMode {
    Optimized,
    Raw,
    Mean,
    Max,
    Min,
    Jitter,
    StdDev,
    Count,
    FirstSample,
    LastSample,
    FirstFill,
    LastFill,
    Median,
    Variance,
    PopVariance,
    Kurtosis,
    Skewness,
    Linear,
    Loess,
    CAPlotBinning,
}

// ===== Data Structures =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UPlotData {
    pub timestamps: Vec<f64>,
    pub series: Vec<Vec<f64>>,
    pub meta: Vec<Meta>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DRVH: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub EGU: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub HIGH: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub HIHI: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DRVL: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub PREC: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LOW: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LOLO: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LOPR: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub HOPR: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub NELM: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DESC: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVMetadata {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub EGU: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub PREC: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DESC: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LOPR: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub HOPR: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DRVL: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DRVH: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LOW: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub HIGH: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub LOLO: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub HIHI: Option<String>,
}

// ===== Point Value Types =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PointValue {
    Float(f32),
    Double(f64),
    Int(i32),
    Long(i64),
    Short(i16),
    Byte(u8),
    String(String),
    Enum(i32),
    ByteArray(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub secs: i64,
    pub nanos: i32,
    pub val: PointValue,
    pub severity: i32,
    pub status: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointJson {
    pub secs: i64,
    pub nanos: i32,
    pub val: serde_json::Value,
    pub severity: Option<u8>,
    pub status: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVData {
    pub meta: Meta,
    pub data: Vec<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVDataJson {
    pub meta: Meta,
    pub data: Vec<PointJson>,
}

// ===== Error and Config Types =====

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

// ===== ProcessingMode Implementation =====

impl ProcessingMode {
   
    pub fn get_operator(&self) -> String {
        match self {
            ProcessingMode::Raw => String::new(),
            ProcessingMode::Optimized => "optimized".to_string(), 
            ProcessingMode::Mean => "mean".to_string(),
            ProcessingMode::Max => "max".to_string(),
            ProcessingMode::Min => "min".to_string(),
            ProcessingMode::Jitter => "jitter".to_string(),
            ProcessingMode::StdDev => "std".to_string(),
            ProcessingMode::Count => "count".to_string(),
            ProcessingMode::FirstSample => "firstSample".to_string(),
            ProcessingMode::LastSample => "lastSample".to_string(),
            ProcessingMode::FirstFill => "firstFill".to_string(),
            ProcessingMode::LastFill => "lastFill".to_string(),
            ProcessingMode::Median => "median".to_string(),
            ProcessingMode::Variance => "variance".to_string(),
            ProcessingMode::PopVariance => "popvariance".to_string(),
            ProcessingMode::Kurtosis => "kurtosis".to_string(),
            ProcessingMode::Skewness => "skewness".to_string(),
            ProcessingMode::Linear => "linear".to_string(),
            ProcessingMode::Loess => "loess".to_string(),
            ProcessingMode::CAPlotBinning => "caplotbinning".to_string(),
        }
    }

    pub fn format_pv(&self, pv: &str) -> String {
        match self {
            ProcessingMode::Raw => pv.to_string(),
            ProcessingMode::Optimized => format!("optimized({})", pv),
            ProcessingMode::Mean => format!("mean({})", pv),
            ProcessingMode::Max => format!("max({})", pv),
            ProcessingMode::Min => format!("min({})", pv),
            ProcessingMode::Jitter => format!("jitter({})", pv),
            ProcessingMode::StdDev => format!("std({})", pv),
            ProcessingMode::Count => format!("count({})", pv),
            ProcessingMode::FirstSample => format!("firstSample({})", pv),
            ProcessingMode::LastSample => format!("lastSample({})", pv),
            ProcessingMode::FirstFill => format!("firstFill({})", pv),
            ProcessingMode::LastFill => format!("lastFill({})", pv),
            ProcessingMode::Median => format!("median({})", pv),
            ProcessingMode::Variance => format!("variance({})", pv),
            ProcessingMode::PopVariance => format!("popvariance({})", pv),
            ProcessingMode::Kurtosis => format!("kurtosis({})", pv),
            ProcessingMode::Skewness => format!("skewness({})", pv),
            ProcessingMode::Linear => format!("linear({})", pv),
            ProcessingMode::Loess => format!("loess({})", pv),
            ProcessingMode::CAPlotBinning => format!("caplotbinning({})", pv),
        }
    }
}

// ===== Display Implementations =====

impl fmt::Display for ProcessingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_operator())
    }
}

impl fmt::Display for PointValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PointValue::Float(v) => write!(f, "{}", v),
            PointValue::Double(v) => write!(f, "{}", v),
            PointValue::Int(v) => write!(f, "{}", v),
            PointValue::Long(v) => write!(f, "{}", v),
            PointValue::Short(v) => write!(f, "{}", v),
            PointValue::Byte(v) => write!(f, "{}", v),
            PointValue::Enum(v) => write!(f, "{}", v),
            PointValue::String(s) => write!(f, "{}", s),
            PointValue::ByteArray(bytes) => write!(f, "{:?}", bytes),
        }
    }
}

// ===== Conversion Implementations =====

impl From<serde_json::Value> for PointValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            Value::Number(num) => {
                if let Some(f64_val) = num.as_f64() {
                    PointValue::Double(f64_val)
                } else if let Some(i64_val) = num.as_i64() {
                    PointValue::Long(i64_val)
                } else if let Some(u64_val) = num.as_u64() {
                    PointValue::Long(u64_val as i64)
                } else {
                    PointValue::String(num.to_string())
                }
            }
            Value::String(s) => PointValue::String(s),
            Value::Bool(b) => PointValue::String(b.to_string()),
            Value::Array(arr) => {
                let byte_array: Vec<u8> = arr
                    .into_iter()
                    .filter_map(|v| v.as_u64().and_then(|u| u.try_into().ok()))
                    .collect();
                PointValue::ByteArray(byte_array)
            }
            _ => PointValue::String(format!("{:?}", value)),
        }
    }
}

impl PointJson {
    pub fn to_point_value(&self) -> Result<PointValue, Error> {
        match &self.val {
            Value::Number(num) => {
                if let Some(f) = num.as_f64() {
                    Ok(PointValue::Double(f))
                } else if let Some(i) = num.as_i64() {
                    Ok(PointValue::Long(i))
                } else if let Some(u) = num.as_u64() {
                    Ok(PointValue::Int(u as i32))
                } else {
                    Err(Error::Decode("Unsupported number type".to_string()))
                }
            }
            Value::String(s) => Ok(PointValue::String(s.clone())),
            Value::Array(arr) => Ok(PointValue::ByteArray(
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as u8))
                    .collect(),
            )),
            _ => Err(Error::Decode(format!(
                "Unexpected value type: {:?}",
                self.val
            ))),
        }
    }
}
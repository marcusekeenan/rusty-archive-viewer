use serde::{Deserialize, Serialize, Deserializer};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVData {
    pub meta: Meta,
    pub data: Vec<Point>,
    #[serde(default)]
    pub statistics: Option<Statistics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub name: String,
    #[serde(alias = "EGU")]
    pub egu: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(alias = "PREC")]
    #[serde(deserialize_with = "deserialize_string_or_number")]
    #[serde(default)]
    pub precision: Option<i32>,
    #[serde(default)]
    pub archive_parameters: Option<ArchiveParameters>,
    #[serde(default)]
    pub display_limits: Option<DisplayLimits>,
    #[serde(default)]
    pub alarm_limits: Option<AlarmLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayLimits {
    pub low: f64,
    pub high: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlarmLimits {
    pub low: f64,
    pub high: f64,
    pub lolo: f64,
    pub hihi: f64,
}

impl DataFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataFormat::Json => "json",
            DataFormat::Csv => "csv",
            DataFormat::Raw => "raw",
            DataFormat::Matlab => "mat",
            DataFormat::Text => "txt",
            DataFormat::Svg => "svg",
        }
    }
}


// Custom deserializer for string or number precision
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => {
            s.parse().map(Some).map_err(|_| D::Error::custom("Invalid precision value"))
        },
        serde_json::Value::Number(n) => {
            n.as_i64().map(|n| Some(n as i32)).ok_or_else(|| D::Error::custom("Invalid number"))
        },
        _ => Ok(None),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveParameters {
    pub sampling_period: f64,
    pub sampling_method: String,
    pub last_modified: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub secs: i64,
    pub nanos: Option<i64>,
    pub val: Value,
    pub severity: Option<i32>,
    pub status: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Single(f64),
    Array(Vec<f64>),
    Text(String),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct ProcessedValue {
    pub value: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub stddev: Option<f64>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedPVData {
    pub meta: Meta,
    pub data: Vec<ProcessedPoint>,
    pub statistics: Option<Statistics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub count: i64,
    pub first_timestamp: i64,
    pub last_timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChunk {
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchStrategy {
    pub operator: String,
    pub chunk_size: i64,
    pub max_points: usize,
    pub use_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOptions {
    pub operator: Option<String>,
    pub chunk_size: Option<i64>,
    pub use_cache: Option<bool>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVStatus {
    pub name: String,
    pub connected: bool,
    pub last_event_time: Option<i64>,
    pub last_status: Option<String>,
    pub archived: bool,
    pub error_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub timestamp: SystemTime,
    pub expires: SystemTime,
}

impl<T> CacheEntry<T> {
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires
    }
}

/// Update types.rs with new structures

/// Response format options based on API documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataFormat {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "raw")]
    Raw,
    #[serde(rename = "mat")]
    Matlab,
    #[serde(rename = "txt")]
    Text,
    #[serde(rename = "svg")]
    Svg,
}

/// Expanded fetch options based on API documentation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtendedFetchOptions {
    pub operator: Option<String>,
    pub timezone: Option<String>,
    pub chart_width: Option<i32>,
    pub batch_size: Option<usize>,
    pub fetch_latest_metadata: Option<bool>,
    pub retired_pv_template: Option<String>,
    pub do_not_chunk: Option<bool>,
    pub ca_count: Option<i32>,
    pub ca_how: Option<i32>,
    pub use_raw_processing: Option<bool>,
    pub format: Option<DataFormat>,  // Added format field
}

/// Point-in-time data response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointInTimeData {
    pub meta: Meta,
    pub data: PointValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointValue {
    pub secs: i64,
    pub nanos: Option<i64>,
    pub val: Value,
    pub severity: Option<i32>,
    pub status: Option<i32>,
}

/// Data operator configuration
/// Data operator configuration that matches the EPICS Archiver Appliance capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataOperator {
    #[serde(rename = "raw")]
    Raw,
    
    // Default binning operators
    #[serde(rename = "firstSample")]
    FirstSample(Option<i32>),  // Optional binning interval in seconds
    #[serde(rename = "lastSample")]
    LastSample(Option<i32>),
    
    // Fill operators (with bin center timestamp)
    #[serde(rename = "firstFill")]
    FirstFill(Option<i32>),
    #[serde(rename = "lastFill")]
    LastFill(Option<i32>),
    
    // Statistical operators
    #[serde(rename = "mean")]
    Mean(Option<i32>),
    #[serde(rename = "min")]
    Min(Option<i32>),
    #[serde(rename = "max")]
    Max(Option<i32>),
    #[serde(rename = "count")]
    Count(Option<i32>),
    
    // Special operators
    #[serde(rename = "ncount")]
    NCount,  // Total number of samples in time span
    #[serde(rename = "nth")]
    Nth(i32),  // Every nth value
    
    // Statistical measures with binning
    #[serde(rename = "median")]
    Median(Option<i32>),
    #[serde(rename = "std")]
    Std(Option<i32>),
    #[serde(rename = "variance")]
    Variance(Option<i32>),
    #[serde(rename = "popvariance")]
    PopVariance(Option<i32>),
    
    // Advanced statistical operators
    #[serde(rename = "jitter")]
    Jitter(Option<i32>),
    #[serde(rename = "kurtosis")]
    Kurtosis(Option<i32>),
    #[serde(rename = "skewness")]
    Skewness(Option<i32>),
    
    // Flyer detection
    #[serde(rename = "ignoreflyers")]
    IgnoreFlyers {
        bin_size: Option<i32>,
        deviations: f64,  // Default is 3.0
    },
    #[serde(rename = "flyers")]
    Flyers {
        bin_size: Option<i32>,
        deviations: f64,
    },
}

impl DataOperator {
    pub fn to_string(&self) -> String {
        match self {
            DataOperator::Raw => "raw".to_string(),
            DataOperator::FirstSample(None) => "firstSample".to_string(),
            DataOperator::FirstSample(Some(bin)) => format!("firstSample_{}", bin),
            DataOperator::LastSample(None) => "lastSample".to_string(),
            DataOperator::LastSample(Some(bin)) => format!("lastSample_{}", bin),
            DataOperator::FirstFill(None) => "firstFill".to_string(),
            DataOperator::FirstFill(Some(bin)) => format!("firstFill_{}", bin),
            DataOperator::LastFill(None) => "lastFill".to_string(),
            DataOperator::LastFill(Some(bin)) => format!("lastFill_{}", bin),
            DataOperator::Mean(None) => "mean".to_string(),
            DataOperator::Mean(Some(bin)) => format!("mean_{}", bin),
            DataOperator::Min(None) => "min".to_string(),
            DataOperator::Min(Some(bin)) => format!("min_{}", bin),
            DataOperator::Max(None) => "max".to_string(),
            DataOperator::Max(Some(bin)) => format!("max_{}", bin),
            DataOperator::Count(None) => "count".to_string(),
            DataOperator::Count(Some(bin)) => format!("count_{}", bin),
            DataOperator::NCount => "ncount".to_string(),
            DataOperator::Nth(n) => format!("nth_{}", n),
            DataOperator::Median(None) => "median".to_string(),
            DataOperator::Median(Some(bin)) => format!("median_{}", bin),
            DataOperator::Std(None) => "std".to_string(),
            DataOperator::Std(Some(bin)) => format!("std_{}", bin),
            DataOperator::Variance(None) => "variance".to_string(),
            DataOperator::Variance(Some(bin)) => format!("variance_{}", bin),
            DataOperator::PopVariance(None) => "popvariance".to_string(),
            DataOperator::PopVariance(Some(bin)) => format!("popvariance_{}", bin),
            DataOperator::Jitter(None) => "jitter".to_string(),
            DataOperator::Jitter(Some(bin)) => format!("jitter_{}", bin),
            DataOperator::Kurtosis(None) => "kurtosis".to_string(),
            DataOperator::Kurtosis(Some(bin)) => format!("kurtosis_{}", bin),
            DataOperator::Skewness(None) => "skewness".to_string(),
            DataOperator::Skewness(Some(bin)) => format!("skewness_{}", bin),
            DataOperator::IgnoreFlyers { bin_size: None, deviations } => 
                format!("ignoreflyers_{}", deviations),
            DataOperator::IgnoreFlyers { bin_size: Some(bin), deviations } => 
                format!("ignoreflyers_{}_{}", bin, deviations),
            DataOperator::Flyers { bin_size: None, deviations } => 
                format!("flyers_{}", deviations),
            DataOperator::Flyers { bin_size: Some(bin), deviations } => 
                format!("flyers_{}_{}", bin, deviations),
        }
    }

    // Helper method to get default bin size based on time range
    pub fn default_bin_size(duration_seconds: i64) -> i32 {
        match duration_seconds {
            d if d <= 3600 => 10,      // <= 1 hour: 10 second bins
            d if d <= 86400 => 60,     // <= 1 day: 1 minute bins
            d if d <= 604800 => 300,   // <= 1 week: 5 minute bins
            _ => 900,                  // > 1 week: 15 minute bins
        }
    }
}

/// Time range specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

/// Expanded PV status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedPVStatus {
    pub name: String,
    pub connected: bool,
    pub archived: bool,
    pub archive_enabled: bool,
    pub last_event_time: Option<i64>,
    pub last_status: Option<String>,
    pub error_count: u32,
    pub last_error: Option<String>,
    pub sampling_period: Option<f64>,
    pub sampling_method: Option<String>,
    pub archival_state: String,
    pub last_modified: Option<SystemTime>,
}

impl Point {
    /// Extracts the value as an f64 if possible
    pub fn value_as_f64(&self) -> Option<f64> {
        match &self.val {
            Value::Single(v) => Some(*v),
            Value::Array(arr) if !arr.is_empty() => Some(arr[0]),
            _ => None,
        }
    }
}

impl DataOperator {
    /// Returns true if this operator supports binning
    pub fn supports_binning(&self) -> bool {
        !matches!(self, 
            DataOperator::Raw | 
            DataOperator::NCount | 
            DataOperator::Nth(_)
        )
    }
}
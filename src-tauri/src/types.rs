use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DataFormat {
    Raw,  // Protocol Buffer format
    Json, // JSON format
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


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub(crate) name: String,
    pub(crate) DRVH: Option<String>,
    pub(crate) EGU: Option<String>,
    pub(crate) HIGH: Option<String>,
    pub(crate) HIHI: Option<String>,
    pub(crate) DRVL: Option<String>,
    pub(crate) PREC: Option<String>,
    pub(crate) LOW: Option<String>,
    pub(crate) LOLO: Option<String>,
    pub(crate) LOPR: Option<String>,
    pub(crate) HOPR: Option<String>,
    pub(crate) NELM: Option<String>,
    pub(crate) DESC: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullMeta {
    pub hostName: Option<String>,
    pub paused: Option<String>,
    pub HIGH: Option<String>,
    pub creationTime: Option<String>,
    pub lowerAlarmLimit: Option<String>,
    pub PREC: Option<String>,
    pub precision: Option<String>,
    pub lowerCtrlLimit: Option<String>,
    pub units: Option<String>,
    pub computedBytesPerEvent: Option<String>,
    pub computedEventRate: Option<String>,
    pub DESC: Option<String>,
    pub usePVAccess: Option<String>,
    pub computedStorageRate: Option<String>,
    pub modificationTime: Option<String>,
    pub upperDisplayLimit: Option<String>,
    pub upperWarningLimit: Option<String>,
    pub NELM: Option<String>,
    pub DBRType: Option<String>,
    pub dataStores: Option<Vec<String>>, // Adjusted to handle array
    pub DRVH: Option<String>,
    pub upperAlarmLimit: Option<String>,
    pub userSpecifiedEventRate: Option<String>,
    pub HIHI: Option<String>,
    pub DRVL: Option<String>,
    pub LOLO: Option<String>,
    pub LOPR: Option<String>,
    pub HOPR: Option<String>,
    pub useDBEProperties: Option<String>,
    pub hasReducedDataSet: Option<String>,
    pub lowerWarningLimit: Option<String>,
    pub chunkKey: Option<String>,
    pub applianceIdentity: Option<String>,
    pub scalar: Option<String>,
    pub EGU: Option<String>,
    pub pvName: Option<String>,
    pub upperCtrlLimit: Option<String>,
    pub LOW: Option<String>,
    pub lowerDisplayLimit: Option<String>,
    pub samplingPeriod: Option<String>,
    pub elementCount: Option<String>,
    pub samplingMethod: Option<String>,
    pub archiveFields: Option<Vec<String>>, // Adjusted to handle array
    pub extraFields: Option<HashMap<String, Value>>, // Adjusted to handle nested object
}

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
    // Add other types as needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointJson {
    pub secs: i64,
    pub nanos: i32,
    pub val: Value,
    pub severity: Option<u8>,
    pub status: Option<u8>,
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
                    Ok(PointValue::Int(u as i32)) // Downcast to i32
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

    pub fn to_point(&self) -> Result<Point, Error> {
        let val = self.to_point_value()?;
        Ok(Point {
            secs: self.secs,
            nanos: self.nanos,
            val,
            severity: self.severity.unwrap_or(0) as i32,
            status: self.status.unwrap_or(0) as i32,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PVDataJson {
    pub meta: Meta,
    pub data: Vec<PointJson>,
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
            PointValue::ByteArray(bytes) => {
                // Format as hex or any way you prefer
                write!(f, "{:?}", bytes)
            }
        }
    }
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
            ProcessingMode::Binning {
                bin_size,
                operation,
            } => match operation {
                BinningOperation::CAPlotBinning => write!(f, "caplot"),
                _ => write!(f, "{}_{}", operation.to_string().to_lowercase(), bin_size),
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

impl From<serde_json::Value> for PointValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Number(num) => {
                if let Some(f64_val) = num.as_f64() {
                    PointValue::Double(f64_val)
                } else if let Some(i64_val) = num.as_i64() {
                    PointValue::Long(i64_val)
                } else if let Some(u64_val) = num.as_u64() {
                    PointValue::Long(u64_val as i64) // Fallback to signed long
                } else {
                    PointValue::String(num.to_string())
                }
            }
            serde_json::Value::String(s) => PointValue::String(s),
            serde_json::Value::Bool(b) => PointValue::String(b.to_string()),
            serde_json::Value::Array(arr) => {
                let byte_array: Vec<u8> = arr
                    .into_iter()
                    .filter_map(|v| v.as_u64().and_then(|u| u.try_into().ok()))
                    .collect();
                PointValue::ByteArray(byte_array)
            }
            _ => PointValue::String(format!("{:?}", value)), // Fallback for unsupported types
        }
    }
}

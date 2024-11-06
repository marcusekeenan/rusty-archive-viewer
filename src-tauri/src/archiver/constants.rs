// constants.rs
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::time::Duration;

pub static API_CONFIG: Lazy<APIConfig> = Lazy::new(|| APIConfig {
    base_url: "http://lcls-archapp.slac.stanford.edu/retrieval/data",
    timeouts: TimeoutConfig::default(),
    request_limits: RequestLimits::default(),
    data_points: DataPointLimits::default(),
    cache_config: CacheConfig::default(),
});

pub static OPERATORS: Lazy<HashMap<&'static str, Operator>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Basic operators
    m.insert("raw", Operator::new("raw", "Returns raw unprocessed data"));
    m.insert(
        "mean",
        Operator::new("mean", "Returns the average value of samples in a bin"),
    );
    m.insert(
        "median",
        Operator::new("median", "Returns the median value in a bin"),
    );

    // Statistical operators
    m.insert(
        "std",
        Operator::new("std", "Returns the standard deviation of values in a bin"),
    );
    m.insert(
        "var",
        Operator::new("var", "Returns the variance of values in a bin"),
    );

    // Sampling operators
    m.insert(
        "firstSample",
        Operator::new("firstSample", "Returns the first sample in a bin"),
    );
    m.insert(
        "lastSample",
        Operator::new("lastSample", "Returns the last sample in a bin"),
    );

    // Optimized operators for different time ranges
    m.insert(
        "optimized_360",
        Operator::new("optimized_360", "10-second resolution optimization"),
    );
    m.insert(
        "optimized_720",
        Operator::new("optimized_720", "30-second resolution optimization"),
    );
    m.insert(
        "optimized_1440",
        Operator::new("optimized_1440", "1-minute resolution optimization"),
    );
    m.insert(
        "optimized_2016",
        Operator::new("optimized_2016", "5-minute resolution optimization"),
    );
    m.insert(
        "optimized_4320",
        Operator::new("optimized_4320", "10-minute resolution optimization"),
    );

    m
});

#[derive(Debug, Clone)]
pub struct APIConfig {
    pub base_url: &'static str,
    pub timeouts: TimeoutConfig,
    pub request_limits: RequestLimits,
    pub data_points: DataPointLimits,
    pub cache_config: CacheConfig,
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub default: Duration,
    pub long: Duration,
    pub extended: Duration,
    pub connection: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            default: Duration::from_secs(30),
            long: Duration::from_secs(60),
            extended: Duration::from_secs(120),
            connection: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestLimits {
    pub max_concurrent: usize,
    pub max_retries: usize,
    pub batch_size: usize,
    pub rate_limit: usize,
}

impl Default for RequestLimits {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            max_retries: 3,
            batch_size: 10,
            rate_limit: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataPointLimits {
    pub default_points: usize,
    pub max_points: usize,
    pub min_points: usize,
    pub chunk_size: usize,
}

impl Default for DataPointLimits {
    fn default() -> Self {
        Self {
            default_points: 1000,
            max_points: 10000,
            min_points: 100,
            chunk_size: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_cache_size: usize,
    pub cache_ttl: Duration,
    pub metadata_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 1000,
            cache_ttl: Duration::from_secs(300),
            metadata_ttl: Duration::from_secs(3600),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub name: &'static str,
    pub description: &'static str,
    pub requires_param: bool,
    pub params: Vec<&'static str>,
}

impl Operator {
    pub fn new(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            requires_param: false,
            params: Vec::new(),
        }
    }
}

pub static ERRORS: Lazy<ErrorConstants> = Lazy::new(|| ErrorConstants {
    invalid_timerange: "Invalid time range specified",
    timeout: "Request timed out",
    no_data: "No data available",
    invalid_pv: "Invalid PV name or format",
    server_error: "Server error occurred",
    rate_limit: "Rate limit exceeded",
    connection_error: "Connection error",
    parse_error: "Data parsing error",
    cache_error: "Cache operation failed",
});

#[derive(Debug, Clone)]
pub struct ErrorConstants {
    pub invalid_timerange: &'static str,
    pub timeout: &'static str,
    pub no_data: &'static str,
    pub invalid_pv: &'static str,
    pub server_error: &'static str,
    pub rate_limit: &'static str,
    pub connection_error: &'static str,
    pub parse_error: &'static str,
    pub cache_error: &'static str,
}

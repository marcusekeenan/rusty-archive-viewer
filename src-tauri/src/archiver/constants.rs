use once_cell::sync::Lazy;
use std::collections::HashMap;

pub static API_CONFIG: Lazy<APIConfig> = Lazy::new(|| APIConfig {
    base_url: "http://lcls-archapp.slac.stanford.edu/retrieval/data",  // Updated to use HTTPS
    timeouts_ms: TimeoutConfig {
        default: 30_000,
        long: 60_000,
        extended: 120_000,
    },
    batch_sizes: BatchSizes {
        default: 5,
        large: 10,
        small: 3,
    },
    target_points: TargetPoints {
        default: 1000,
        high_res: 2000,
        low_res: 500,
    },
});

pub static OPERATORS: Lazy<HashMap<&'static str, Operator>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("mean", Operator {
        name: "mean",
        description: "Returns the average value of samples in a bin",
        requires_param: false,
        params: vec![],
    });
    m.insert("firstSample", Operator {
        name: "firstSample",
        description: "Returns the first sample in a bin",
        requires_param: false,
        params: vec![],
    });
    // Add other operators as needed
    m
});

#[derive(Debug, Clone)]
pub struct APIConfig {
    pub base_url: &'static str,
    pub timeouts_ms: TimeoutConfig,
    pub batch_sizes: BatchSizes,
    pub target_points: TargetPoints,
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub default: u64,
    pub long: u64,
    pub extended: u64,
}

#[derive(Debug, Clone)]
pub struct BatchSizes {
    pub default: usize,
    pub large: usize,
    pub small: usize,
}

#[derive(Debug, Clone)]
pub struct TargetPoints {
    pub default: usize,
    pub high_res: usize,
    pub low_res: usize,
}

#[derive(Debug, Clone)]
pub struct Operator {
    pub name: &'static str,
    pub description: &'static str,
    pub requires_param: bool,
    pub params: Vec<&'static str>,
}

pub static ERRORS: Lazy<ErrorConstants> = Lazy::new(|| ErrorConstants {
    invalid_timerange: "Invalid time range specified",
    timeout: "Request timed out",
    no_data: "No data available",
    invalid_pv: "Invalid PV name",
    server_error: "Server error",
    rate_limit: "Rate limit exceeded",
});

#[derive(Debug, Clone)]
pub struct ErrorConstants {
    pub invalid_timerange: &'static str,
    pub timeout: &'static str,
    pub no_data: &'static str,
    pub invalid_pv: &'static str,
    pub server_error: &'static str,
    pub rate_limit: &'static str,
}




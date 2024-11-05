// validation.rs

use crate::archiver::{
    error::{ArchiverError, ErrorContext, Result},
    types::*
};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    time::Duration,
};
use tracing::{debug, warn};

/// Regular expression for validating PV names
static PV_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Za-z0-9_\-:\.]{1,255}$")
        .expect("Failed to compile PV name regex")
});

/// Set of valid operator base names
static VALID_OPERATORS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::with_capacity(20);
    // Basic operators
    set.extend(["raw", "firstSample", "lastSample", "firstFill", "lastFill"]);
    
    // Statistical operators
    set.extend([
        "mean", "min", "max", "count", "median", "std",
        "variance", "popvariance", "jitter", "kurtosis", "skewness"
    ]);
    
    // Special operators
    set.extend(["optimized", "nth", "ignoreflyers", "flyers"]);
    set
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestParameters {
    pub pvs: Vec<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub operator: Option<String>,
    pub chart_width: Option<i32>,
    pub options: Option<ExtendedFetchOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveUpdateParameters {
    pub pvs: Vec<String>,
    #[serde(with = "humantime_serde")]
    pub update_interval: Duration,
    pub buffer_size: usize,
    pub operator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationLimits {
    pub max_pvs: usize,
    #[serde(with = "humantime_serde")]
    pub max_time_range: Duration,
    pub min_bin_size: i64,
    pub max_bin_size: i64,
    #[serde(with = "humantime_serde")]
    pub min_update_interval: Duration,
    #[serde(with = "humantime_serde")]
    pub max_update_interval: Duration,
    pub max_buffer_size: usize,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_pvs: 100,
            max_time_range: Duration::from_secs(365 * 24 * 60 * 60), // 1 year
            min_bin_size: 1,
            max_bin_size: 86400, // 24 hours in seconds
            min_update_interval: Duration::from_millis(100),
            max_update_interval: Duration::from_secs(60),
            max_buffer_size: 100000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Validator;

impl Validator {
    pub fn validate_pv_name(name: &str) -> Result<()> {
        let error_context = ErrorContext::new("Validator", "validate_pv_name")
            .with_info(format!("pv_name: {}", name));

        if !PV_NAME_REGEX.is_match(name) {
            return Err(ArchiverError::InvalidRequest {
                message: "Invalid PV name".into(),
                context: format!("PV: {}", name),
                validation_errors: vec!["PV name contains invalid characters or exceeds length limit".into()],
                error_context: Some(error_context),
            });
        }

        Ok(())
    }

    pub fn validate_time_range(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<()> {
        let error_context = ErrorContext::new("Validator", "validate_time_range")
            .with_info(format!("start: {}, end: {}", start, end));

        let mut errors = Vec::new();
        let now = Utc::now();
        let max_past = now - ChronoDuration::days(365 * 10);
        let max_future = now + ChronoDuration::minutes(1);

        if end <= start {
            errors.push("End time must be after start time");
        }

        if end - start > ChronoDuration::days(365) {
            errors.push("Time range cannot exceed 1 year");
        }

        if start < max_past {
            errors.push("Start time cannot be more than 10 years in the past");
        }

        if end > max_future {
            errors.push("End time cannot be more than 1 minute in the future");
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid time range".into(),
                context: format!("start: {}, end: {}", start, end),
                validation_errors: errors.into_iter().map(String::from).collect(),
                error_context: Some(error_context),
            })
        }
    }

    pub fn validate_operator(operator: &str) -> Result<()> {
        let error_context = ErrorContext::new("Validator", "validate_operator")
            .with_info(format!("operator: {}", operator));

        let parts: Vec<&str> = operator.split('_').collect();
        let base_op = parts[0];

        if !VALID_OPERATORS.contains(base_op) {
            return Err(ArchiverError::InvalidRequest {
                message: "Invalid operator".into(),
                context: format!("operator: {}", operator),
                validation_errors: vec![format!("Unknown operator: {}", base_op)],
                error_context: Some(error_context),
            });
        }

        let mut errors = Vec::new();

        match base_op {
            "raw" => {
                if parts.len() > 1 {
                    errors.push("Raw operator does not accept parameters");
                }
            }
            "nth" => {
                match parts.get(1).and_then(|n| n.parse::<i32>().ok()) {
                    Some(n) if n <= 0 => {
                        errors.push("nth operator requires a positive integer parameter");
                    }
                    None => {
                        errors.push("nth operator parameter must be a valid integer");
                    }
                    _ => {}
                }
            }
            "ignoreflyers" | "flyers" => {
                match (parts.get(1), parts.get(2)) {
                    (Some(bin_size), Some(deviations)) => {
                        if let Err(_) = bin_size.parse::<i32>() {
                            errors.push("Bin size must be a valid integer");
                        }
                        if let Err(_) = deviations.parse::<f64>() {
                            errors.push("Deviation threshold must be a valid number");
                        }
                    }
                    _ => errors.push("Flyer operators require bin size and deviation threshold parameters"),
                }
            }
            _ => {
                if let Some(bin_size) = parts.get(1) {
                    match bin_size.parse::<i32>() {
                        Ok(size) if size <= 0 || size > 86400 => {
                            errors.push("Bin size must be between 1 and 86400 seconds");
                        }
                        Err(_) => {
                            errors.push("Bin size must be a valid integer");
                        }
                        _ => {}
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid operator parameters".into(),
                context: format!("operator: {}", operator),
                validation_errors: errors.into_iter().map(String::from).collect(),
                error_context: Some(error_context),
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestValidator {
    limits: ValidationLimits,
}

impl RequestValidator {
    pub fn new() -> Self {
        Self {
            limits: ValidationLimits::default(),
        }
    }

    pub fn with_limits(limits: ValidationLimits) -> Self {
        Self { limits }
    }

    pub fn validate_data_request(&self, params: &RequestParameters) -> Result<()> {
        let error_context = ErrorContext::new("RequestValidator", "validate_data_request");
        let mut errors = Vec::new();

        // Validate PV count
        if params.pvs.is_empty() {
            errors.push("No PVs specified".to_string());
        } else if params.pvs.len() > self.limits.max_pvs {
            errors.push(format!("Too many PVs in request (maximum {})", self.limits.max_pvs));
        }

        // Validate each PV name
        for pv in &params.pvs {
            if let Err(e) = Validator::validate_pv_name(pv) {
                if let ArchiverError::InvalidRequest { validation_errors, .. } = e {
                    errors.extend(validation_errors);
                }
            }
        }

        // Validate time range
        let duration = params.end_time - params.start_time;
        if duration > ChronoDuration::from_std(self.limits.max_time_range).unwrap() {
            errors.push(format!("Time range too large (maximum {:?})", self.limits.max_time_range));
        }

        // Validate operator if present
        if let Some(operator) = &params.operator {
            if let Err(e) = Validator::validate_operator(operator) {
                if let ArchiverError::InvalidRequest { validation_errors, .. } = e {
                    errors.extend(validation_errors);
                }
            }

            // Validate bin size if present
            if let Some(bin_size_str) = operator.split('_').nth(1) {
                if let Ok(bin_size) = bin_size_str.parse::<i64>() {
                    if bin_size < self.limits.min_bin_size || bin_size > self.limits.max_bin_size {
                        errors.push(format!(
                            "Bin size must be between {} and {} seconds",
                            self.limits.min_bin_size,
                            self.limits.max_bin_size
                        ));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Request validation failed".into(),
                context: "Request validation".into(),
                validation_errors: errors,
                error_context: Some(error_context),
            })
        }
    }

    pub fn validate_live_update_params(&self, params: &LiveUpdateParameters) -> Result<()> {
        let error_context = ErrorContext::new("RequestValidator", "validate_live_update_params");
        let mut errors = Vec::new();

        // Validate PVs
        if params.pvs.is_empty() {
            errors.push("No PVs specified for live updates".to_string());
        } else if params.pvs.len() > self.limits.max_pvs {
            errors.push(format!("Too many PVs for live updates (maximum {})", self.limits.max_pvs));
        }

        // Validate update interval
        if params.update_interval < self.limits.min_update_interval || 
           params.update_interval > self.limits.max_update_interval {
            errors.push(format!(
                "Update interval must be between {:?} and {:?}",
                self.limits.min_update_interval,
                self.limits.max_update_interval
            ));
        }

        // Validate buffer size
        if params.buffer_size == 0 {
            errors.push("Buffer size must be greater than 0".to_string());
        } else if params.buffer_size > self.limits.max_buffer_size {
            errors.push(format!("Buffer size too large (maximum {})", self.limits.max_buffer_size));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid live update parameters".into(),
                context: "Live update validation".into(),
                validation_errors: errors,
                error_context: Some(error_context),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pv_name_validation() {
        // Valid cases
        assert!(Validator::validate_pv_name("ROOM:TEMP:1").is_ok());
        assert!(Validator::validate_pv_name("Device_1.ReadBack").is_ok());
        assert!(Validator::validate_pv_name("TEST-123").is_ok());

        // Invalid cases
        assert!(Validator::validate_pv_name("").is_err());
        assert!(Validator::validate_pv_name("ROOM TEMP").is_err());
        assert!(Validator::validate_pv_name("Device#1").is_err());
        assert!(Validator::validate_pv_name("Test/PV").is_err());
    }

    #[test]
    fn test_custom_validation_limits() {
        let custom_limits = ValidationLimits {
            max_pvs: 50,
            max_time_range: Duration::from_secs(3600), // 1 hour
            min_bin_size: 5,
            max_bin_size: 300,
            min_update_interval: Duration::from_secs(1),
            max_update_interval: Duration::from_secs(30),
            max_buffer_size: 1000,
        };

        let validator = RequestValidator::with_limits(custom_limits);
        let now = Utc::now();

        let test_params = RequestParameters {
            pvs: vec!["TEST:PV1".to_string(); 51], // Exceeds new limit of 50
            start_time: now - ChronoDuration::minutes(90), // Exceeds new limit of 1 hour
            end_time: now,
            operator: Some("mean_2".to_string()), // Below new min_bin_size of 5
            chart_width: Some(800),
            options: None,
        };

        match validator.validate_data_request(&test_params) {
            Err(ArchiverError::InvalidRequest { 
                validation_errors, 
                error_context: Some(ctx), 
                .. 
            }) => {
                assert!(validation_errors.iter().any(|e| e.contains("Too many PVs")));
                assert!(validation_errors.iter().any(|e| e.contains("Time range too large")));
                assert_eq!(ctx.source_component, "RequestValidator");
                assert_eq!(ctx.operation, "validate_data_request");
            }
            _ => panic!("Expected InvalidRequest error with multiple validation failures"),
        }
    }
}
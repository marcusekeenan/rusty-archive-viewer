// validation.rs

use crate::archiver::{
    error::{ArchiverError, Result},
    types::*
};


use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

// Static Patterns
static PV_NAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[A-Za-z0-9_\-:\.]+$").expect("Failed to compile PV name regex")
});

static VALID_OPERATORS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    // Basic operators
    set.insert("raw");
    set.insert("firstSample");
    set.insert("lastSample");
    set.insert("firstFill");
    set.insert("lastFill");
    
    // Statistical operators
    set.insert("mean");
    set.insert("min");
    set.insert("max");
    set.insert("count");
    set.insert("median");
    set.insert("std");
    set.insert("variance");
    set.insert("popvariance");
    set.insert("jitter");
    set.insert("kurtosis");
    set.insert("skewness");
    
    // Special operators
    set.insert("optimized");
    set.insert("nth");
    set.insert("ignoreflyers");
    set.insert("flyers");
    set
});

// Request Types
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
    pub max_update_interval: Duration,
    #[serde(with = "humantime_serde")]
    pub min_update_interval: Duration,
    pub max_buffer_size: usize,
}

impl Default for ValidationLimits {
    fn default() -> Self {
        Self {
            max_pvs: 100,
            max_time_range: Duration::from_secs(365 * 24 * 60 * 60), // 1 year
            min_bin_size: 1,
            max_bin_size: 86400,
            max_update_interval: Duration::from_secs(60),
            min_update_interval: Duration::from_millis(100),
            max_buffer_size: 100000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Validator;

impl Validator {
    pub fn validate_pv_name(name: &str) -> Result<()> {
        let mut errors = Vec::new();

        if name.is_empty() {
            errors.push("PV name cannot be empty");
        }

        if name.len() > 255 {
            errors.push("PV name exceeds maximum length of 255 characters");
        }

        if !PV_NAME_REGEX.is_match(name) {
            errors.push("PV name contains invalid characters");
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid PV name".into(),
                context: format!("PV: {}", name),
                validation_errors: errors.into_iter().map(String::from).collect(),
            })
        }
    }

    pub fn validate_time_range(start: DateTime<Utc>, end: DateTime<Utc>) -> Result<()> {
        let mut errors = Vec::new();
        let now = Utc::now();

        if end <= start {
            errors.push("End time must be after start time");
        }

        let duration = end - start;
        if duration > chrono::Duration::days(365) {
            errors.push("Time range cannot exceed 1 year");
        }

        if start < now - chrono::Duration::days(365 * 10) {
            errors.push("Start time cannot be more than 10 years in the past");
        }

        if end > now + chrono::Duration::minutes(1) {
            errors.push("End time cannot be more than 1 minute in the future");
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid time range".into(),
                context: format!("start: {}, end: {}", start, end),
                validation_errors: errors.into_iter().map(String::from).collect(),
            })
        }
    }

    pub fn validate_operator(operator: &str) -> Result<()> {
        let mut errors = Vec::new();
        let parts: Vec<&str> = operator.split('_').collect();
        let base_op = parts[0];

        if !VALID_OPERATORS.contains(base_op) {
            errors.push(format!("Unknown operator: {}", base_op));
        }

        // Validate operator parameters
        match base_op {
            "nth" => {
                if let Some(n) = parts.get(1) {
                    if let Ok(value) = n.parse::<i32>() {
                        if value <= 0 {
                            errors.push(String::from("nth operator requires a positive integer parameter"));
                        }
                    } else {
                        errors.push(String::from("nth operator parameter must be a valid integer"));
                    }
                } else {
                    errors.push(String::from("nth operator requires a parameter"));
                }
            }
            "mean" | "firstSample" | "lastSample" | "firstFill" | "lastFill" |
            "min" | "max" | "count" | "median" | "std" | "variance" | "popvariance" |
            "jitter" | "kurtosis" | "skewness" => {
                if let Some(bin_size) = parts.get(1) {
                    if let Ok(value) = bin_size.parse::<i32>() {
                        if value <= 0 {
                            errors.push(String::from("Bin size must be positive"));
                        }
                        if value > 86400 {
                            errors.push(String::from("Bin size cannot exceed 86400 seconds (24 hours)"));
                        }
                    } else {
                        errors.push(String::from("Bin size must be a valid integer"));
                    }
                }
            }
            "ignoreflyers" | "flyers" => {
                match (parts.get(1), parts.get(2)) {
                    (Some(bin_size), Some(deviations)) => {
                        if let Ok(bin) = bin_size.parse::<i32>() {
                            if bin <= 0 {
                                errors.push("Bin size must be positive".to_string());
                            }
                        } else {
                            errors.push("Bin size must be a valid integer".to_string());
                        }
                        
                        if let Ok(dev) = deviations.parse::<f64>() {
                            if dev <= 0.0 {
                                errors.push("Deviation threshold must be positive".to_string());
                            }
                        } else {
                            errors.push("Deviation threshold must be a valid number".to_string());
                        }
                    }
                    _ => errors.push("Flyer operators require bin size and deviation threshold parameters".to_string()),
                }
            }
            "raw" => {
                if parts.len() > 1 {
                    errors.push("Raw operator does not accept parameters".to_string());
                }
            }
            _ => {}
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid operator".into(),
                context: format!("operator: {}", operator),
                validation_errors: errors,
            })
        }
    }

    pub fn validate_request_params(params: &RequestParameters) -> Result<()> {
        let mut errors = Vec::new();
    
        // Validate PVs
        if params.pvs.is_empty() {
            errors.push("No PVs specified".to_string());
        } else {
            for pv in &params.pvs {
                if let Err(e) = Self::validate_pv_name(pv) {
                    match &e {
                        ArchiverError::InvalidRequest { message, context, .. } => {
                            errors.push(format!("Invalid PV '{}': {} ({})", pv, message, context));
                        }
                        _ => errors.push(format!("Invalid PV '{}': {}", pv, e)),
                    }
                }
            }
        }
    
        // Validate time range
        if let Err(e) = Self::validate_time_range(params.start_time, params.end_time) {
            match &e {
                ArchiverError::InvalidRequest { message, context, .. } => {
                    errors.push(format!("Invalid time range: {} ({})", message, context));
                }
                _ => errors.push(format!("Invalid time range: {}", e)),
            }
        }
    
        // Validate operator if present
        if let Some(operator) = &params.operator {
            if let Err(e) = Self::validate_operator(operator) {
                match &e {
                    ArchiverError::InvalidRequest { message, context, .. } => {
                        errors.push(format!("Invalid operator: {} ({})", message, context));
                    }
                    _ => errors.push(format!("Invalid operator: {}", e)),
                }
            }
        }
    
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid request parameters".into(),
                context: "Request validation".into(),
                validation_errors: errors,
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
        // First run basic validation
        Validator::validate_request_params(params)?;

        let mut errors = Vec::new();

        // Check PV count
        if params.pvs.len() > self.limits.max_pvs {
            errors.push(format!(
                "Too many PVs in request (maximum {})",
                self.limits.max_pvs
            ));
        }

        // Check time range
        let duration = params.end_time - params.start_time;
        if duration > chrono::Duration::from_std(self.limits.max_time_range).unwrap() {
            errors.push(format!(
                "Time range too large (maximum {:?})",
                self.limits.max_time_range
            ));
        }

        // Check bin size if present
        if let Some(operator) = &params.operator {
            if let Some(bin_size_str) = operator.split('_').nth(1) {
                if let Ok(bin_size) = bin_size_str.parse::<i64>() {
                    if bin_size < self.limits.min_bin_size {
                        errors.push(format!(
                            "Bin size too small (minimum {} seconds)",
                            self.limits.min_bin_size
                        ));
                    }
                    if bin_size > self.limits.max_bin_size {
                        errors.push(format!(
                            "Bin size too large (maximum {} seconds)",
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
                message: "Request exceeds limits".into(),
                context: "Request validation".into(),
                validation_errors: errors,
            })
        }
    }

    pub fn validate_live_update_params(&self, params: &LiveUpdateParameters) -> Result<()> {
        let mut errors = Vec::<String>::new();  // Explicitly specify Vec<String>
    
        // Validate PVs
        if params.pvs.is_empty() {
            errors.push("No PVs specified for live updates".to_string());
        } else if params.pvs.len() > self.limits.max_pvs {
            errors.push(format!(
                "Too many PVs for live updates (maximum {})",
                self.limits.max_pvs
            ));
        }
    
        // Validate update interval
        if params.update_interval < self.limits.min_update_interval {
            errors.push(format!(
                "Update interval too small (minimum {:?})",
                self.limits.min_update_interval
            ));
        }
        if params.update_interval > self.limits.max_update_interval {
            errors.push(format!(
                "Update interval too large (maximum {:?})",
                self.limits.max_update_interval
            ));
        }
    
        // Validate buffer size
        if params.buffer_size == 0 {
            errors.push("Buffer size must be greater than 0".to_string());
        }
        if params.buffer_size > self.limits.max_buffer_size {
            errors.push(format!(
                "Buffer size too large (maximum {})",
                self.limits.max_buffer_size
            ));
        }
    
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ArchiverError::InvalidRequest {
                message: "Invalid live update parameters".into(),
                context: "Live update validation".into(),
                validation_errors: errors,  // Now all errors are Strings
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
    fn test_time_range_validation() {
        let now = Utc::now();

        // Valid cases
        assert!(Validator::validate_time_range(
            now - chrono::Duration::hours(1),
            now,
        ).is_ok());

        // Invalid cases
        assert!(Validator::validate_time_range(
            now,
            now - chrono::Duration::hours(1),
        ).is_err());

        assert!(Validator::validate_time_range(
            now - chrono::Duration::days(400),
            now,
        ).is_err());

        assert!(Validator::validate_time_range(
            now,
            now + chrono::Duration::hours(1),
        ).is_err());
    }

    #[test]
    fn test_operator_validation() {
        // Valid cases
        assert!(Validator::validate_operator("raw").is_ok());
        assert!(Validator::validate_operator("mean_60").is_ok());
        assert!(Validator::validate_operator("optimized_800").is_ok());
        assert!(Validator::validate_operator("nth_10").is_ok());
        assert!(Validator::validate_operator("ignoreflyers_300_3.0").is_ok());

        // Invalid cases
        assert!(Validator::validate_operator("invalid").is_err());
        assert!(Validator::validate_operator("mean_0").is_err());
        assert!(Validator::validate_operator("mean_invalid").is_err());
        assert!(Validator::validate_operator("nth_0").is_err());
        assert!(Validator::validate_operator("ignoreflyers_0_0.0").is_err());
    }

    #[test]
    fn test_request_validator() {
        let validator = RequestValidator::new();
        let now = Utc::now();

        let valid_params = RequestParameters {
            pvs: vec!["TEST:PV1".to_string()],
            start_time: now - chrono::Duration::hours(1),
            end_time: now,
            operator: Some("mean_60".to_string()),
            chart_width: Some(800),
            options: None,
        };

        assert!(validator.validate_data_request(&valid_params).is_ok());

        // Test excessive PVs
        let mut invalid_params = valid_params.clone();
        invalid_params.pvs = (0..101).map(|i| format!("PV:{}", i)).collect();
        assert!(validator.validate_data_request(&invalid_params).is_err());

        // Test invalid time range
        let mut invalid_params = valid_params.clone();
        invalid_params.start_time = now - chrono::Duration::days(366);
        assert!(validator.validate_data_request(&invalid_params).is_err());

        // Test invalid bin size
        let mut invalid_params = valid_params.clone();
        invalid_params.operator = Some("mean_0".to_string());
        assert!(validator.validate_data_request(&invalid_params).is_err());
    }

    #[test]
    fn test_live_update_validation() {
        let validator = RequestValidator::new();

        let valid_params = LiveUpdateParameters {
            pvs: vec!["TEST:PV1".to_string()],
            update_interval: Duration::from_secs(1),
            buffer_size: 1000,
            operator: None,
        };

        assert!(validator.validate_live_update_params(&valid_params).is_ok());

        // Test invalid update interval
        let mut invalid_params = valid_params.clone();
        invalid_params.update_interval = Duration::from_millis(50); // Too small
        assert!(validator.validate_live_update_params(&invalid_params).is_err());

        // Test invalid buffer size
        let mut invalid_params = valid_params.clone();
        invalid_params.buffer_size = 200000; // Too large
        assert!(validator.validate_live_update_params(&invalid_params).is_err());

        // Test too many PVs
        let mut invalid_params = valid_params.clone();
        invalid_params.pvs = (0..101).map(|i| format!("PV:{}", i)).collect();
        assert!(validator.validate_live_update_params(&invalid_params).is_err());
    }

    #[test]
    fn test_custom_validation_limits() {
        let custom_limits = ValidationLimits {
            max_pvs: 50,
            max_time_range: Duration::from_secs(3600), // 1 hour
            min_bin_size: 5,
            max_bin_size: 300,
            max_update_interval: Duration::from_secs(30),
            min_update_interval: Duration::from_secs(1),
            max_buffer_size: 1000,
        };

        let validator = RequestValidator::with_limits(custom_limits);
        let now = Utc::now();

        // Test with stricter limits
        let params = RequestParameters {
            pvs: vec!["TEST:PV1".to_string(); 51], // Exceeds new limit of 50
            start_time: now - chrono::Duration::minutes(90), // Exceeds new limit of 1 hour
            end_time: now,
            operator: Some("mean_2".to_string()), // Below new min_bin_size of 5
            chart_width: Some(800),
            options: None,
        };

        let result = validator.validate_data_request(&params);
        assert!(result.is_err());
        
        if let Err(ArchiverError::InvalidRequest { validation_errors, .. }) = result {
            assert!(validation_errors.iter().any(|e| e.contains("Too many PVs")));
            assert!(validation_errors.iter().any(|e| e.contains("Time range too large")));
        }
    }
}
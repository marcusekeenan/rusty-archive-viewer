// error.rs

use chrono::{DateTime, Utc};
use std::error::Error as StdError;
use thiserror::Error;
use std::time::Duration;

#[derive(Error, Debug)]
pub enum ArchiverError {
    #[error("Connection error: {message} (context: {context})")]
    ConnectionError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        retry_after: Option<Duration>,
    },

    #[error("Data fetch error: {message} (context: {context})")]
    DataError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        timestamp: DateTime<Utc>,
        pv: Option<String>,
    },

    #[error("Cache error: {message} (context: {context})")]
    CacheError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        cache_key: Option<String>,
    },

    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Duration,
        limit: usize,
        window: Duration,
    },

    #[error("Invalid request: {message}")]
    InvalidRequest {
        message: String,
        context: String,
        validation_errors: Vec<String>,
    },

    #[error("Server error: {message} (status: {status})")]
    ServerError {
        message: String,
        status: u16,
        body: Option<String>,
        retry_after: Option<Duration>,
    },

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),

    #[error(transparent)]
    ChronoError(#[from] chrono::ParseError),
}

impl ArchiverError {
    /// Returns true if the error is likely transient and the operation can be retried
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ConnectionError { .. } => true,
            Self::RateLimit { .. } => true,
            Self::ServerError { status, .. } => matches!(status, 502 | 503 | 504),
            Self::DataError { .. } => false,
            Self::CacheError { .. } => false,
            Self::InvalidRequest { .. } => false,
            Self::IoError(_) => false,
            Self::ReqwestError(e) => e.is_timeout() || e.is_connect(),
            Self::JsonError(_) => false,
            Self::ChronoError(_) => false,
        }
    }

    /// Returns a suggested retry delay if applicable
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::ConnectionError { retry_after, .. } => *retry_after,
            Self::RateLimit { retry_after, .. } => Some(*retry_after),
            Self::ServerError { retry_after, .. } => *retry_after,
            _ => None,
        }
    }

    /// Returns true if the error indicates a problem with the request itself
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidRequest { .. } | 
            Self::JsonError(_) |
            Self::ChronoError(_)
        )
    }

    /// Returns true if the error indicates a server-side problem
    pub fn is_server_error(&self) -> bool {
        matches!(
            self,
            Self::ServerError { status, .. } if *status >= 500
        )
    }

    /// Creates a new ConnectionError
    pub fn connection_error<T: Into<String>>(message: T, context: T) -> Self {
        Self::ConnectionError {
            message: message.into(),
            context: context.into(),
            source: None,
            retry_after: Some(Duration::from_secs(1)),
        }
    }

    /// Creates a new DataError
    pub fn data_error<T: Into<String>>(message: T, context: T, pv: Option<String>) -> Self {
        Self::DataError {
            message: message.into(),
            context: context.into(),
            source: None,
            timestamp: Utc::now(),
            pv,
        }
    }

    /// Creates a new CacheError
    pub fn cache_error<T: Into<String>>(message: T, context: T, cache_key: Option<String>) -> Self {
        Self::CacheError {
            message: message.into(),
            context: context.into(),
            source: None,
            cache_key,
        }
    }

    /// Creates a new RateLimit error
    pub fn rate_limit(limit: usize, window: Duration) -> Self {
        Self::RateLimit {
            message: format!("Rate limit of {} requests per {:?} exceeded", limit, window),
            retry_after: window,
            limit,
            window,
        }
    }

    /// Adds context to an existing error
    pub fn with_context<T: Into<String>>(self, context: T) -> Self {
        match self {
            Self::ConnectionError { message, retry_after, .. } => Self::ConnectionError {
                message,
                context: context.into(),
                source: None,
                retry_after,
            },
            // Add similar matches for other variants
            _ => self
        }
    }
}

/// Result type alias for ArchiverError
pub type Result<T> = std::result::Result<T, ArchiverError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_creation() {
        let err = ArchiverError::connection_error("Failed to connect", "Timeout");
        assert!(err.is_retryable());
        assert!(err.retry_after().is_some());
    }

    #[test]
    fn test_rate_limit_error() {
        let window = Duration::from_secs(60);
        let err = ArchiverError::rate_limit(100, window);
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(window));
    }

    #[test]
    fn test_data_error() {
        let err = ArchiverError::data_error(
            "Invalid data", 
            "Parsing failed",
            Some("TEST:PV1".to_string())
        );
        assert!(!err.is_retryable());
        assert!(err.retry_after().is_none());
    }

    #[test]
    fn test_is_client_error() {
        let err = ArchiverError::InvalidRequest {
            message: "Bad request".to_string(),
            context: "Validation failed".to_string(),
            validation_errors: vec!["Invalid PV name".to_string()],
        };
        assert!(err.is_client_error());
        assert!(!err.is_server_error());
    }
}
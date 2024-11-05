// error.rs

use chrono::{DateTime, Utc};
use std::error::Error as StdError;
use thiserror::Error;
use std::time::Duration;
use std::fmt;
use uuid::Uuid;
use tracing::{error, warn, debug};

/// Provides detailed context for error tracking and handling
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub timestamp: DateTime<Utc>,
    pub source_component: &'static str,
    pub operation: String,
    pub retry_count: u32,
    pub last_retry: Option<DateTime<Utc>>,
    pub trace_id: Uuid,
    pub additional_info: Option<String>,
}

impl ErrorContext {
    pub fn new(source_component: &'static str, operation: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            source_component,
            operation: operation.into(),
            retry_count: 0,
            last_retry: None,
            trace_id: Uuid::new_v4(),
            additional_info: None,
        }
    }

    pub fn with_info(mut self, info: impl Into<String>) -> Self {
        self.additional_info = Some(info.into());
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Operation '{}' in component '{}' at {} (trace_id: {})",
            self.operation,
            self.source_component,
            self.timestamp,
            self.trace_id
        )?;
        if let Some(info) = &self.additional_info {
            write!(f, " - Additional info: {}", info)?;
        }
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ArchiverError {
    #[error("Connection error: {message} ({context})")]
    ConnectionError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        retry_after: Option<Duration>,
        error_context: Option<ErrorContext>,
    },

    #[error("Data fetch error: {message} ({context})")]
    DataError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        timestamp: DateTime<Utc>,
        pv: Option<String>,
        error_context: Option<ErrorContext>,
    },

    #[error("Cache error: {message} ({context})")]
    CacheError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        cache_key: Option<String>,
        error_context: Option<ErrorContext>,
    },

    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        retry_after: Duration,
        limit: usize,
        window: Duration,
        error_context: Option<ErrorContext>,
    },

    #[error("Invalid request: {message} ({context})")]
    InvalidRequest {
        message: String,
        context: String,
        validation_errors: Vec<String>,
        error_context: Option<ErrorContext>,
    },

    #[error("Server error: {message} (status: {status})")]
    ServerError {
        message: String,
        status: u16,
        body: Option<String>,
        retry_after: Option<Duration>,
        error_context: Option<ErrorContext>,
    },

    #[error("Session error: {message}")]
    SessionError {
        message: String,
        context: String,
        session_id: Option<Uuid>,
        error_context: Option<ErrorContext>,
    },

    #[error("Health check error: {message} ({context})")]
    HealthCheckError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        error_context: Option<ErrorContext>,
    },

    #[error("Initialization error: {message}")]
    InitializationError {
        message: String,
        context: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
        error_context: Option<ErrorContext>,
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
    /// Creates a new error with context
    pub fn with_context(self, source_component: &'static str, operation: impl Into<String>) -> Self {
        let error_context = ErrorContext::new(source_component, operation);
        self.add_context(error_context)
    }

    /// Adds error context to an existing error
    pub fn add_context(self, error_context: ErrorContext) -> Self {
        match self {
            Self::ConnectionError { message, context, source, retry_after, .. } => Self::ConnectionError {
                message,
                context,
                source,
                retry_after,
                error_context: Some(error_context),
            },
            Self::DataError { message, context, source, timestamp, pv, .. } => Self::DataError {
                message,
                context,
                source,
                timestamp,
                pv,
                error_context: Some(error_context),
            },
            Self::CacheError { message, context, source, cache_key, .. } => Self::CacheError {
                message,
                context,
                source,
                cache_key,
                error_context: Some(error_context),
            },
            Self::RateLimit { message, retry_after, limit, window, .. } => Self::RateLimit {
                message,
                retry_after,
                limit,
                window,
                error_context: Some(error_context),
            },
            Self::InvalidRequest { message, context, validation_errors, .. } => Self::InvalidRequest {
                message,
                context,
                validation_errors,
                error_context: Some(error_context),
            },
            Self::ServerError { message, status, body, retry_after, .. } => Self::ServerError {
                message,
                status,
                body,
                retry_after,
                error_context: Some(error_context),
            },
            Self::SessionError { message, context, session_id, .. } => Self::SessionError {
                message,
                context,
                session_id,
                error_context: Some(error_context),
            },
            Self::HealthCheckError { message, context, source, .. } => Self::HealthCheckError {
                message,
                context,
                source,
                error_context: Some(error_context),
            },
            Self::InitializationError { message, context, source, .. } => Self::InitializationError {
                message,
                context,
                source,
                error_context: Some(error_context),
            },
            // For wrapped standard errors, we'll return them as-is
            other => other,
        }
    }

    /// Returns true if the error is likely transient and the operation can be retried
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ConnectionError { .. } => true,
            Self::RateLimit { .. } => true,
            Self::ServerError { status, .. } => matches!(status, 502 | 503 | 504),
            Self::SessionError { .. } => true,
            Self::DataError { .. } => false,
            Self::CacheError { .. } => false,
            Self::InvalidRequest { .. } => false,
            Self::IoError(_) => false,
            Self::ReqwestError(e) => e.is_timeout() || e.is_connect(),
            Self::JsonError(_) => false,
            Self::ChronoError(_) => false,
            Self::HealthCheckError { .. } => true,
            Self::InitializationError { .. } => false,
        }
    }

    /// Returns a suggested retry delay if applicable
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::ConnectionError { retry_after, .. } => *retry_after,
            Self::RateLimit { retry_after, .. } => Some(*retry_after),
            Self::ServerError { retry_after, .. } => *retry_after,
            Self::SessionError { .. } => Some(Duration::from_secs(1)),
            _ => None,
        }
    }

    /// Returns the number of times this error has been retried
    pub fn retry_count(&self) -> u32 {
        self.error_context()
            .map(|ctx| ctx.retry_count)
            .unwrap_or(0)
    }

    /// Access the error context if available
    pub fn error_context(&self) -> Option<&ErrorContext> {
        match self {
            Self::ConnectionError { error_context, .. } => error_context.as_ref(),
            Self::DataError { error_context, .. } => error_context.as_ref(),
            Self::CacheError { error_context, .. } => error_context.as_ref(),
            Self::RateLimit { error_context, .. } => error_context.as_ref(),
            Self::InvalidRequest { error_context, .. } => error_context.as_ref(),
            Self::ServerError { error_context, .. } => error_context.as_ref(),
            Self::SessionError { error_context, .. } => error_context.as_ref(),
            Self::HealthCheckError { error_context, .. } => error_context.as_ref(),
            Self::InitializationError { error_context, .. } => error_context.as_ref(),
            _ => None,
        }
    }

    /// Increment the retry count and update last retry timestamp
    pub fn increment_retry(&mut self) {
        if let Some(context) = self.error_context_mut() {
            context.retry_count += 1;
            context.last_retry = Some(Utc::now());
            debug!(
                "Retry {} for operation '{}' (trace_id: {})",
                context.retry_count,
                context.operation,
                context.trace_id
            );
        }
    }

    /// Get mutable access to the error context
    fn error_context_mut(&mut self) -> Option<&mut ErrorContext> {
        match self {
            Self::ConnectionError { error_context, .. } => error_context.as_mut(),
            Self::DataError { error_context, .. } => error_context.as_mut(),
            Self::CacheError { error_context, .. } => error_context.as_mut(),
            Self::RateLimit { error_context, .. } => error_context.as_mut(),
            Self::InvalidRequest { error_context, .. } => error_context.as_mut(),
            Self::ServerError { error_context, .. } => error_context.as_mut(),
            Self::SessionError { error_context, .. } => error_context.as_mut(),
            Self::HealthCheckError { error_context, .. } => error_context.as_mut(),
            Self::InitializationError { error_context, .. } => error_context.as_mut(),
            _ => None,
        }
    }

    /// Log the error with appropriate severity
    pub fn log(&self) {
        let context = self.error_context()
            .map(|ctx| ctx.to_string())
            .unwrap_or_else(|| "No context available".to_string());

        match self {
            Self::ConnectionError { message, .. } |
            Self::ServerError { message, .. } |
            Self::DataError { message, .. } => {
                error!("{} - {}", message, context);
            }
            Self::RateLimit { message, .. } |
            Self::CacheError { message, .. } => {
                warn!("{} - {}", message, context);
            }
            Self::SessionError { message, .. } |
            Self::InvalidRequest { message, .. } => {
                debug!("{} - {}", message, context);
            }
            _ => {
                error!("Unexpected error: {} - {}", self, context);
            }
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
    fn test_error_context() {
        let error = ArchiverError::ConnectionError {
            message: "Failed to connect".to_string(),
            context: "Test context".to_string(),
            source: None,
            retry_after: Some(Duration::from_secs(1)),
            error_context: None,
        }
        .with_context("TEST", "test_operation");

        assert!(error.error_context().is_some());
        assert_eq!(error.error_context().unwrap().source_component, "TEST");
        assert_eq!(error.error_context().unwrap().operation, "test_operation");
    }

    #[test]
    fn test_retryable_errors() {
        let connection_error = ArchiverError::ConnectionError {
            message: "Test".to_string(),
            context: "Test".to_string(),
            source: None,
            retry_after: None,
            error_context: None,
        };
        assert!(connection_error.is_retryable());

        let validation_error = ArchiverError::InvalidRequest {
            message: "Test".to_string(),
            context: "Test".to_string(),
            validation_errors: vec![],
            error_context: None,
        };
        assert!(!validation_error.is_retryable());
    }

    #[test]
    fn test_retry_count() {
        let mut error = ArchiverError::ConnectionError {
            message: "Test".to_string(),
            context: "Test".to_string(),
            source: None,
            retry_after: None,
            error_context: Some(ErrorContext::new("TEST", "test_operation")),
        };

        assert_eq!(error.retry_count(), 0);
        error.increment_retry();
        assert_eq!(error.retry_count(), 1);
    }
}
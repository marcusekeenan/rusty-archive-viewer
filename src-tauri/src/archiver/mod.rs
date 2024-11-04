// archiver/mod.rs

pub mod api;
pub mod commands;
pub mod constants;
pub mod types;
pub mod export;
pub mod cache;
pub mod error;
pub mod health;
pub mod metrics;
pub mod session;
pub mod validation;

#[cfg(test)]
mod tests;

// Re-export core types and functions
pub use self::{
    api::ArchiverClient,
    commands::{
        fetch_binned_data,
        fetch_data_with_operator,
        fetch_optimized_data,
        fetch_raw_data,
        get_data_at_time,
        get_pv_metadata,
        get_pv_status,
        test_connection,
    },
    error::{ArchiverError, Result},
    health::{HealthMonitor, HealthStatus, SystemStatus},
    metrics::{ApiMetrics, MetricsSummary},
    session::{Session, SessionManager, UserPreferences},
    validation::{Validator, RequestValidator},
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const API_VERSION: &str = "1.0.0";

/// Initialize a new archiver client with default configuration
pub fn init_client() -> Result<ArchiverClient> {
    ArchiverClient::new()
}

/// Initialize metrics collection
pub fn init_metrics() -> metrics::ApiMetrics {
    metrics::ApiMetrics::new()
}

/// Initialize health monitoring
pub fn init_health_monitor() -> health::HealthMonitor {
    health::HealthMonitor::new(
        chrono::Duration::seconds(10), // check interval
        1000,                         // max history entries
    )
}

/// Initialize session management
pub fn init_session_manager() -> session::SessionManager {
    session::SessionManager::new(
        1000,                          // max sessions
        chrono::Duration::hours(24),   // session timeout
    )
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio::test;

    #[test]
    async fn test_client_initialization() {
        let client = init_client();
        assert!(client.is_ok());
    }

    #[test]
    async fn test_full_system() {
        let client = init_client().unwrap();
        let metrics = init_metrics();
        let health = init_health_monitor();
        let sessions = init_session_manager();

        // Test basic connectivity
        let connection_test = test_connection().await;
        assert!(connection_test.is_ok());

        // More integration tests can be added here...
    }
}

// Internal utilities and helpers
#[doc(hidden)]
pub(crate) mod utils {
    use chrono::{DateTime, TimeZone, Utc};

    pub(crate) fn format_timestamp(ms: i64) -> String {
        Utc.timestamp_millis_opt(ms)
            .single()
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "invalid timestamp".to_string())
    }

    pub(crate) fn parse_timestamp(s: &str) -> Result<i64, String> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.timestamp_millis())
            .map_err(|e| format!("Failed to parse timestamp: {}", e))
    }
}
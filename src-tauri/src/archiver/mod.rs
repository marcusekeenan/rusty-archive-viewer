//! EPICS Archiver Appliance Interface Module
//! Provides stateless data access and processing capabilities for archived PV data.
//! This module focuses on efficient data retrieval and processing while maintaining
//! a clean separation between frontend state management and backend data operations.

pub mod api;
pub mod commands;
pub mod constants;
pub mod export;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export commands for data operations
pub use commands::{
    export_data,         // Data export in various formats
    fetch_data,          // Historical data fetching
    fetch_data_at_time,  // Point-in-time data retrieval
    get_pv_metadata,     // Metadata information
    get_pv_status,       // PV status checks
    test_connection,     // Connection testing
    toggle_debug_window, // Debug interface
    validate_pvs,        // PV validation
};

// Re-export core functionality from API
pub use api::{
    ArchiverClient,    // Stateless client for archiver interaction
    DataProcessor,     // Data processing utilities
    DataRequest,       // Request configuration structures
    OptimizationLevel, // Data optimization strategies
    TimeRangeMode,     // Time range specification options
};

// Re-export types and constants
pub use constants::{
    API_CONFIG, // API configuration constants
    ERRORS,     // Error message constants
    OPERATORS,  // Available data operators
};
pub use types::*;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_SUPPORTED_API_VERSION: &str = "1.0.0";

/// Basic usage example:
/// ```rust,no_run
/// use rusty_archive_viewer::archiver::{ArchiverClient, TimeRangeMode, OptimizationLevel};
///
/// async fn example() -> Result<(), String> {
///     let client = ArchiverClient::new()?;
///     let mode = TimeRangeMode::Fixed {
///         start: 0,
///         end: 1000,
///     };
///     
///     let data = client.fetch_data(
///         "SOME:PV:NAME",
///         &mode,
///         OptimizationLevel::Auto,
///         Some(1000),
///         Some("UTC"),
///     ).await?;
///     
///     Ok(())
/// }
/// ```
///
/// The module provides a stateless interface to the EPICS Archiver Appliance,
/// focusing on efficient data retrieval and processing. All state management
/// is handled by the frontend, while the backend focuses on:
///
/// - Efficient data retrieval
/// - Data optimization and processing
/// - Error handling and validation
/// - Format conversion and export

#[doc(hidden)]
#[deprecated(since = "1.0.0", note = "use the new stateless API instead")]
pub fn _legacy_helper() {}

/// Feature flags for compilation configuration
pub mod features {
    /// Indicates whether debug features are enabled
    pub const DEBUG_ENABLED: bool = cfg!(debug_assertions);

    /// Maximum number of concurrent requests
    pub const MAX_CONCURRENT_REQUESTS: usize = 10;

    /// Default chart width for optimization calculations
    pub const DEFAULT_CHART_WIDTH: i32 = 1000;
}

/// Utility functions for common operations
pub mod utils {
    use chrono::{DateTime, TimeZone, Utc};

    /// Converts a timestamp to a formatted string
    pub fn format_timestamp(ts: i64) -> String {
        if let Some(dt) = Utc.timestamp_millis_opt(ts).single() {
            dt.to_rfc3339()
        } else {
            "Invalid timestamp".to_string()
        }
    }

    /// Validates a time range
    pub fn validate_time_range(start: i64, end: i64) -> bool {
        start < end && start > 0 && end > 0
    }
}

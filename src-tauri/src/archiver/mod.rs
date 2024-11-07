//! EPICS Archiver Appliance Interface Module
//! Provides data access and processing capabilities for archived PV data with support
//! for both historical and real-time data retrieval.

pub mod api;
pub mod commands;
pub mod constants;
pub mod export;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export commonly used commands
pub use commands::{
    export_data,         // Data export in various formats
    fetch_data,          // Historical data fetching
    fetch_data_at_time,  // Point-in-time data retrieval
    get_pv_metadata,     // Metadata information
    get_pv_status,       // PV status and health
    test_connection,     // Connection testing
    toggle_debug_window, // Debug interface
    validate_pvs,        // PV validation
    start_live_updates,
    stop_live_updates,
};

// Re-export core functionality from API
pub use api::{
    ArchiverClient,    // Main client for interacting with the archiver
    DataProcessor,     // Data processing implementation
    DataRequest,       // Request configuration
    OptimizationLevel, // Data optimization configuration
    TimeRangeMode,     // Time range specification modes
};

// Re-export types and constants
pub use constants::{
    API_CONFIG, // API configuration
    ERRORS,     // Error constants
    OPERATORS,  // Available operators
};
pub use types::*;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_SUPPORTED_API_VERSION: &str = "1.0.0";

// #[doc(hidden)]
// pub fn _doc_test_helper() {}

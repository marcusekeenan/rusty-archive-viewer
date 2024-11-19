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
    export_data,        // Data export in various formats
    fetch_data,         // Historical data fetching
    fetch_data_at_time, // Point-in-time data retrieval
    get_pv_metadata,    // Metadata information
    get_pv_status,      // PV status and health
    start_live_updates,
    stop_live_updates,
    test_connection,     // Connection testing
    toggle_debug_window, // Debug interface
    validate_pvs,        // PV validation
};

// Re-export core functionality from API
pub use api::{
    ArchiverClient,
    DataProcessor,
    DataRequest,
    OptimizationLevel,
    TimeRangeMode,
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

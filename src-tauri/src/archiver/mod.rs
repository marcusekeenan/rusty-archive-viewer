//! EPICS Archiver Appliance Interface Module
//! Provides data access and processing capabilities for archived PV data

pub mod api;
pub mod commands;
pub mod constants;
pub mod export;
pub mod types;

#[cfg(test)]
mod tests;

// Re-export commonly used commands
pub use commands::{
    export_data,         // Data export functionality
    fetch_data,          // Main data fetching function
    fetch_live_data,     // Current/live data
    get_pv_metadata,     // Metadata retrieval
    get_pv_status,       // PV status information
    test_connection,     // Connection testing
    toggle_debug_window, // Debug window toggling
    validate_pvs,        // PV validation
};

// Re-export types and constants
pub use constants::*;
pub use types::*;

// Re-export error handling
pub use api::DataFetch;
pub use api::DataProcess;

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_SUPPORTED_API_VERSION: &str = "1.0.0";

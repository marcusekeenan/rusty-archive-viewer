// In src/lib.rs
pub mod client;
pub mod commands;
pub mod constants;
pub mod decode;
pub mod decode_helpers;
pub mod types;

pub use client::ArchiverClient;
pub use commands::{fetch_data, get_pv_metadata, test_connection, AppState};
pub use types::{Config, Error, Meta, PVData, Point};

// Generated protobuf code
pub mod epics {
    include!("epics.rs");
}

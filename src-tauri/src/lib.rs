// In src/lib.rs
pub mod client;
pub mod commands;
pub mod constants;
pub mod types;
pub mod decode;
pub mod decode_helpers;

pub use client::ArchiverClient;
pub use commands::{fetch_data, fetch_latest, test_connection, get_pv_metadata, AppState};
pub use types::{
    Config,
    Point,
    PVData,
    Error,
    Meta,
};

// Generated protobuf code
pub mod epics {
    include!("epics.rs");
}
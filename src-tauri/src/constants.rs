use std::time::Duration;

// Protocol constants
pub const ESCAPE_CHAR: u8 = 0x1B;
pub const ESCAPE_ESCAPE_CHAR: u8 = 0x01;
pub const NEWLINE_CHAR: u8 = 0x0A;
pub const NEWLINE_ESCAPE_CHAR: u8 = 0x02;
pub const CARRIAGERETURN_CHAR: u8 = 0x0D;
pub const CARRIAGERETURN_ESCAPE_CHAR: u8 = 0x03;

// Default configuration
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
pub const DEFAULT_BASE_URL: &str = "http://lcls-archapp.slac.stanford.edu/retrieval";

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
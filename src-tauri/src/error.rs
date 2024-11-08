use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Command failed: {0}")]
    CommandError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Resource error: {0}")]
    ResourceError(String)
}

impl Into<tauri::Error> for AppError {
    fn into(self) -> tauri::Error {
        tauri::Error::Generic(self.to_string())
    }
}
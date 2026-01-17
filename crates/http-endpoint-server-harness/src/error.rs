use thiserror::Error;

/// Errors that can occur during harness execution
#[derive(Error, Debug)]
pub enum HarnessError {
    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

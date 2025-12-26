//! Error types for VexLake

use thiserror::Error;

/// Result type alias for VexLake operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for VexLake operations
#[derive(Error, Debug)]
pub enum Error {
    /// Storage operation failed
    #[error("Storage error: {0}")]
    Storage(#[from] opendal::Error),

    /// Arrow/Parquet operation failed
    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    /// Serialization/deserialization failed
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Index operation failed
    #[error("Index error: {0}")]
    Index(String),

    /// Vector dimension mismatch
    #[error("Dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    /// Key not found
    #[error("Key not found: {0}")]
    NotFound(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// FFI error
    #[error("FFI error: {0}")]
    Ffi(String),

    /// Generic error
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::NotFound("test_key".to_string());
        assert!(err.to_string().contains("test_key"));
    }

    #[test]
    fn test_dimension_mismatch() {
        let err = Error::DimensionMismatch {
            expected: 128,
            actual: 256,
        };
        assert!(err.to_string().contains("128"));
        assert!(err.to_string().contains("256"));
    }
}

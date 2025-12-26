//! VexLake Core - Cloud-Native Vector Database Engine
//!
//! This crate provides the core functionality for VexLake:
//! - SIMD-accelerated vector distance computation
//! - HNSW/IVF index management
//! - S3/SeaweedFS storage via OpenDAL
//! - Parquet read/write via DataFusion
//! - FFI exports for Go integration
//!
//! # Architecture
//!
//! VexLake follows the "Sandwich Architecture":
//! - Go Layer: RESP protocol, client management
//! - Rust Core (this crate): ALL compute and I/O
//! - Storage: SeaweedFS via S3 API

pub mod error;
pub mod ffi;
pub mod index;
pub mod storage;
pub mod vector;

pub use error::{Error, Result};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Health check function
pub fn health_check() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_health_check() {
        assert!(health_check());
    }
}

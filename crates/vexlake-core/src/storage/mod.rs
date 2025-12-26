//! Storage layer for VexLake
//!
//! This module provides S3-compatible storage access via OpenDAL:
//! - Parquet file read/write
//! - Index file management
//! - Version metadata handling

pub mod metadata;
pub mod parquet;

use opendal::Operator;
pub use metadata::{MetadataManager, VersionInfo};
pub use parquet::{ParquetReader, ParquetWriter, VexSchema};

use crate::{Error, Result};

/// Storage configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct StorageConfig {
    /// S3 endpoint URL
    pub endpoint: String,
    /// S3 bucket name
    pub bucket: String,
    /// AWS access key ID
    pub access_key_id: Option<String>,
    /// AWS secret access key
    pub secret_access_key: Option<String>,
    /// AWS region
    pub region: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8333".to_string(),
            bucket: "vexlake".to_string(),
            access_key_id: None,
            secret_access_key: None,
            region: "us-east-1".to_string(),
        }
    }
}

/// Create an S3 operator from configuration
pub fn create_s3_operator(config: &StorageConfig) -> Result<Operator> {
    let mut builder = opendal::services::S3::default()
        .endpoint(&config.endpoint)
        .bucket(&config.bucket)
        .region(&config.region);

    if let Some(ref key) = config.access_key_id {
        builder = builder.access_key_id(key);
    }
    if let Some(ref secret) = config.secret_access_key {
        builder = builder.secret_access_key(secret);
    }

    // SeaweedFS specific optimizations
    builder = builder.enable_virtual_host_style();

    let op = Operator::new(builder)
        .map_err(Error::Storage)?
        .finish();

    Ok(op)
}

/// Create an in-memory operator for testing
pub fn create_memory_operator() -> Result<Operator> {
    let builder = opendal::services::Memory::default();
    let op = Operator::new(builder)
        .map_err(Error::Storage)?
        .finish();
    Ok(op)
}

/// Storage client for VexLake operations
pub struct StorageClient {
    operator: Operator,
}

impl StorageClient {
    /// Create a new storage client
    pub fn new(operator: Operator) -> Self {
        Self { operator }
    }

    /// Create from S3 configuration
    pub fn from_config(config: &StorageConfig) -> Result<Self> {
        let operator = create_s3_operator(config)?;
        Ok(Self::new(operator))
    }

    /// Create an in-memory client for testing
    pub fn memory() -> Result<Self> {
        let operator = create_memory_operator()?;
        Ok(Self::new(operator))
    }

    /// Get the underlying operator
    pub fn operator(&self) -> &Operator {
        &self.operator
    }

    /// Write data to storage
    pub async fn write(&self, path: &str, data: Vec<u8>) -> Result<()> {
        self.operator
            .write(path, data)
            .await
            .map_err(Error::Storage)
    }

    /// Read data from storage
    pub async fn read(&self, path: &str) -> Result<Vec<u8>> {
        self.operator
            .read(path)
            .await
            .map(|buf| buf.to_vec())
            .map_err(Error::Storage)
    }

    /// Check if a path exists
    pub async fn exists(&self, path: &str) -> Result<bool> {
        self.operator
            .exists(path)
            .await
            .map_err(Error::Storage)
    }

    /// Delete a path
    pub async fn delete(&self, path: &str) -> Result<()> {
        self.operator.delete(path).await.map_err(Error::Storage)
    }

    /// Delete all objects under a prefix
    pub async fn delete_all(&self, prefix: &str) -> Result<()> {
        self.operator.remove_all(prefix).await.map_err(Error::Storage)
    }

    /// List objects under a prefix
    pub async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        let entries = self
            .operator
            .list(prefix)
            .await
            .map_err(Error::Storage)?;

        Ok(entries.into_iter().map(|e| e.path().to_string()).collect())
    }
}

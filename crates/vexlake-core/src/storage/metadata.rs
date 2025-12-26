//! Versioned metadata management for VexLake
//!
//! VexLake uses a versioned metadata system to achieve snapshot isolation (MVCC).
//! Each version is stored as a JSON file in SeaweedFS.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::StorageClient;
use crate::{Error, Result};

/// Information about a VexLake data version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Version number
    pub version: u64,
    /// Timestamp of creation
    pub timestamp: u64,
    /// Map of partition ID to Parquet file path
    pub data_files: HashMap<String, String>,
    /// Map of index name to index file path
    pub index_files: HashMap<String, String>,
    /// Number of vectors in this version
    pub total_vectors: usize,
}

/// Manager for VexLake metadata
pub struct MetadataManager<'a> {
    client: &'a StorageClient,
}

impl<'a> MetadataManager<'a> {
    /// Create a new metadata manager
    pub fn new(client: &'a StorageClient) -> Self {
        Self { client }
    }

    /// Get the path for a specific version's metadata file
    fn version_path(version: u64) -> String {
        format!("_metadata/version_{}.json", version)
    }

    /// Get the path for the "latest" version pointer
    fn latest_path() -> String {
        "_metadata/latest".to_string()
    }

    /// Get the latest version number
    pub async fn get_latest_version_num(&self) -> Result<u64> {
        if !self.client.exists(&Self::latest_path()).await? {
            return Ok(0);
        }

        let data = self.client.read(&Self::latest_path()).await?;
        let content = String::from_utf8(data).map_err(|e| Error::Ffi(e.to_string()))?;
        content
            .trim()
            .parse::<u64>()
            .map_err(|e| Error::Ffi(e.to_string()))
    }

    /// Get details for a specific version
    pub async fn get_version(&self, version: u64) -> Result<VersionInfo> {
        if version == 0 {
            return Ok(VersionInfo {
                version: 0,
                timestamp: 0,
                data_files: HashMap::new(),
                index_files: HashMap::new(),
                total_vectors: 0,
            });
        }

        let data = self.client.read(&Self::version_path(version)).await?;
        serde_json::from_slice(&data).map_err(Error::Serialization)
    }

    /// Get details for the latest version
    pub async fn get_latest_version(&self) -> Result<VersionInfo> {
        let latest = self.get_latest_version_num().await?;
        self.get_version(latest).await
    }

    /// Commit a new version
    pub async fn commit_version(&self, info: VersionInfo) -> Result<()> {
        let version = info.version;
        let data = serde_json::to_vec(&info).map_err(Error::Serialization)?;

        // 1. Write the versioned metadata file
        self.client
            .write(&Self::version_path(version), data)
            .await?;

        // 2. Update the "latest" pointer (pseudo-atomic in S3)
        self.client
            .write(&Self::latest_path(), version.to_string().into_bytes())
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metadata_lifecycle() {
        let client = StorageClient::memory().unwrap();
        let manager = MetadataManager::new(&client);

        // Initial state
        assert_eq!(manager.get_latest_version_num().await.unwrap(), 0);

        // Commit version 1
        let mut data_files = HashMap::new();
        data_files.insert("0".to_string(), "data/part-0.parquet".to_string());

        let v1 = VersionInfo {
            version: 1,
            timestamp: 123456789,
            data_files,
            index_files: HashMap::new(),
            total_vectors: 100,
        };

        manager.commit_version(v1).await.unwrap();

        // Verify version 1
        assert_eq!(manager.get_latest_version_num().await.unwrap(), 1);
        let loaded = manager.get_latest_version().await.unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.total_vectors, 100);
    }
}

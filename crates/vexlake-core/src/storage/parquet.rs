//! Parquet data handling for VexLake using Arrow and DataFusion
//!
//! This module defines the VexLake data schema and provides utilities for
//! reading and writing vector data in Parquet format.

use arrow::array::{ArrayRef, FixedSizeListArray, Float32Array, RecordBatch, UInt64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use std::sync::Arc;

use crate::{Error, Result};
use super::StorageClient;

/// Schema for VexLake vector data
pub struct VexSchema;

impl VexSchema {
    /// Get the schema for a specific vector dimension
    pub fn get(dimension: usize) -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::UInt64, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            Field::new("metadata", DataType::Utf8, true),
        ]))
    }
}

/// Writer for VexLake Parquet files
pub struct ParquetWriter<'a> {
    #[allow(dead_code)]
    client: &'a StorageClient,
    dimension: usize,
}

impl<'a> ParquetWriter<'a> {
    /// Create a new Parquet writer
    pub fn new(client: &'a StorageClient, dimension: usize) -> Self {
        Self { client, dimension }
    }

    /// Create a RecordBatch from raw vector data
    pub fn create_batch(
        &self,
        ids: &[u64],
        vectors: &[Vec<f32>],
        metadata: &[Option<String>],
    ) -> Result<RecordBatch> {
        if ids.len() != vectors.len() || ids.len() != metadata.len() {
            return Err(Error::InvalidConfig("Input arrays must have same length".to_string()));
        }

        let id_array = UInt64Array::from(ids.to_vec());
        
        let mut flattened_vectors = Vec::with_capacity(vectors.len() * self.dimension);
        for v in vectors {
            if v.len() != self.dimension {
                return Err(Error::DimensionMismatch { expected: self.dimension, actual: v.len() });
            }
            flattened_vectors.extend_from_slice(v);
        }
        
        let values = Float32Array::from(flattened_vectors);
        let field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array = FixedSizeListArray::try_new(
            field,
            self.dimension as i32,
            Arc::new(values) as ArrayRef,
            None,
        )
        .map_err(|e| Error::Arrow(e))?;

        let metadata_array = StringArray::from(metadata.to_vec());

        RecordBatch::try_new(
            VexSchema::get(self.dimension),
            vec![
                Arc::new(id_array) as ArrayRef,
                Arc::new(vector_array) as ArrayRef,
                Arc::new(metadata_array) as ArrayRef,
            ],
        )
        .map_err(Error::Arrow)
    }

    /// Write a RecordBatch to storage in Parquet format
    pub async fn write_batch(&self, path: &str, batch: &RecordBatch) -> Result<()> {
        use parquet::arrow::AsyncArrowWriter;

        let mut buf = Vec::new();
        let mut writer = AsyncArrowWriter::try_new(&mut buf, batch.schema(), None)
            .map_err(|e| Error::Index(e.to_string()))?;
        
        writer.write(batch).await.map_err(|e| Error::Index(e.to_string()))?;
        writer.close().await.map_err(|e| Error::Index(e.to_string()))?;

        self.client.write(path, buf).await?;
        Ok(())
    }
}

use datafusion::prelude::*;
use datafusion::physical_plan::collect;

/// Reader for VexLake Parquet files using DataFusion
pub struct ParquetReader<'a> {
    client: &'a StorageClient,
}

impl<'a> ParquetReader<'a> {
    /// Create a new Parquet reader
    pub fn new(client: &'a StorageClient) -> Self {
        Self { client }
    }

    /// Read all vectors from a Parquet file
    pub async fn read_all(&self, path: &str) -> Result<Vec<RecordBatch>> {
        // DataFusion SessionContext
        let _ctx = SessionContext::new();
        
        // Since we are using OpenDAL, for now we might need to read the whole file 
        // into memory or implement an ObjectStore for DataFusion.
        // For simplicity in this phase, we'll read the file and use ctx.read_parquet with a local path
        // OR better, we use RecordBatchReader from the parquet crate directly for now 
        // until we have the full DataFusion ObjectStore integrated.
        
        let data = self.client.read(path).await?;
        let bytes = bytes::Bytes::from(data);
        
        use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
        
        let builder = ParquetRecordBatchReaderBuilder::try_new(bytes)
            .map_err(|e| Error::Index(e.to_string()))?;
        
        let reader = builder.build().map_err(|e| Error::Index(e.to_string()))?;
        
        let mut batches = Vec::new();
        for batch in reader {
            batches.push(batch.map_err(Error::Arrow)?);
        }
        
        Ok(batches)
    }

    /// Execute a query using DataFusion
    pub async fn query(&self, path: &str, sql: &str) -> Result<Vec<RecordBatch>> {
        let ctx = SessionContext::new();
        
        // We'll write to a temp file to allow DataFusion to read it
        // TODO: In Phase 4, we will register an ObjectStore for direct S3 reading
        let data = self.client.read(path).await?;
        let temp_dir = tempfile::tempdir().map_err(|e| Error::Storage(opendal::Error::new(opendal::ErrorKind::Unexpected, &e.to_string())))?;
        let file_path = temp_dir.path().join("data.parquet");
        std::fs::write(&file_path, data).map_err(|e| Error::Storage(opendal::Error::new(opendal::ErrorKind::Unexpected, &e.to_string())))?;

        ctx.register_parquet("vectors", file_path.to_str().unwrap(), ParquetReadOptions::default())
            .await
            .map_err(|e| Error::Index(e.to_string()))?;

        let df = ctx.sql(sql).await.map_err(|e| Error::Index(e.to_string()))?;
        let plan = df.create_physical_plan().await.map_err(|e| Error::Index(e.to_string()))?;
        let task_ctx = ctx.task_ctx();
        
        let result = collect(plan, task_ctx).await.map_err(|e| Error::Index(e.to_string()))?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parquet_roundtrip() {
        let client = StorageClient::memory().unwrap();
        let writer = ParquetWriter::new(&client, 3);
        let reader = ParquetReader::new(&client);

        let ids = vec![1, 2];
        let vectors = vec![
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
        ];
        let metadata = vec![
            Some("{\"tag\": \"a\"}".to_string()),
            None,
        ];

        let batch = writer.create_batch(&ids, &vectors, &metadata).unwrap();
        writer.write_batch("data/test.parquet", &batch).await.unwrap();

        assert!(client.exists("data/test.parquet").await.unwrap());

        // Test read_all
        let read_batches = reader.read_all("data/test.parquet").await.unwrap();
        assert_eq!(read_batches.len(), 1);
        assert_eq!(read_batches[0].num_rows(), 2);

        // Test query
        let query_results = reader.query("data/test.parquet", "SELECT id FROM vectors WHERE id = 1").await.unwrap();
        assert_eq!(query_results.len(), 1);
        assert_eq!(query_results[0].num_rows(), 1);
    }
}

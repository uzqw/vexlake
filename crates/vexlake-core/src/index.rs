//! Vector index implementations
//!
//! This module provides vector indexing algorithms:
//! - HNSW (Hierarchical Navigable Small World)
//! - IVF (Inverted File Index) - future
//!
//! Indexes are serializable and can be stored in S3.

use std::collections::HashMap;

use crate::{Error, Result};

/// Index configuration
#[derive(Debug, Clone)]
pub struct IndexConfig {
    /// Vector dimension
    pub dimension: usize,
    /// HNSW M parameter (number of connections per layer)
    pub m: usize,
    /// HNSW ef_construction parameter
    pub ef_construction: usize,
    /// HNSW ef_search parameter
    pub ef_search: usize,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            dimension: 128,
            m: 16,
            ef_construction: 200,
            ef_search: 50,
        }
    }
}

/// Simple in-memory vector index (placeholder for HNSW)
pub struct VectorIndex {
    config: IndexConfig,
    vectors: HashMap<u64, Vec<f32>>,
    next_id: u64,
}

impl VectorIndex {
    /// Create a new vector index
    pub fn new(config: IndexConfig) -> Self {
        Self {
            config,
            vectors: HashMap::new(),
            next_id: 0,
        }
    }

    /// Create with default configuration
    pub fn with_dimension(dimension: usize) -> Self {
        Self::new(IndexConfig {
            dimension,
            ..Default::default()
        })
    }

    /// Insert a vector into the index
    pub fn insert(&mut self, vector: Vec<f32>) -> Result<u64> {
        if vector.len() != self.config.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimension,
                actual: vector.len(),
            });
        }

        let id = self.next_id;
        self.next_id += 1;
        self.vectors.insert(id, vector);
        Ok(id)
    }

    /// Insert a vector with a specific ID
    pub fn insert_with_id(&mut self, id: u64, vector: Vec<f32>) -> Result<()> {
        if vector.len() != self.config.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimension,
                actual: vector.len(),
            });
        }

        self.vectors.insert(id, vector);
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        Ok(())
    }

    /// Get a vector by ID
    pub fn get(&self, id: u64) -> Option<&Vec<f32>> {
        self.vectors.get(&id)
    }

    /// Delete a vector by ID
    pub fn delete(&mut self, id: u64) -> bool {
        self.vectors.remove(&id).is_some()
    }

    /// Search for the top K most similar vectors
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<crate::vector::SearchResult>> {
        if query.len() != self.config.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimension,
                actual: query.len(),
            });
        }

        let vectors: Vec<(u64, Vec<f32>)> = self
            .vectors
            .iter()
            .map(|(id, v)| (*id, v.clone()))
            .collect();

        Ok(crate::vector::brute_force_topk(query, &vectors, k))
    }

    /// Get the number of vectors in the index
    pub fn len(&self) -> usize {
        self.vectors.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.vectors.is_empty()
    }

    /// Get the dimension of vectors in this index
    pub fn dimension(&self) -> usize {
        self.config.dimension
    }

    /// Clear all vectors from the index
    pub fn clear(&mut self) {
        self.vectors.clear();
        self.next_id = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_insert_and_get() {
        let mut index = VectorIndex::with_dimension(3);

        let id = index.insert(vec![1.0, 2.0, 3.0]).unwrap();
        assert_eq!(id, 0);

        let vec = index.get(id).unwrap();
        assert_eq!(vec, &vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_index_dimension_mismatch() {
        let mut index = VectorIndex::with_dimension(3);

        let result = index.insert(vec![1.0, 2.0]);
        assert!(result.is_err());
    }

    #[test]
    fn test_index_search() {
        let mut index = VectorIndex::with_dimension(3);

        index.insert(vec![1.0, 0.0, 0.0]).unwrap();
        index.insert(vec![0.0, 1.0, 0.0]).unwrap();
        index.insert(vec![0.5, 0.5, 0.0]).unwrap();

        let results = index.search(&[1.0, 0.0, 0.0], 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 0); // Most similar
    }

    #[test]
    fn test_index_delete() {
        let mut index = VectorIndex::with_dimension(3);

        let id = index.insert(vec![1.0, 2.0, 3.0]).unwrap();
        assert!(index.get(id).is_some());

        assert!(index.delete(id));
        assert!(index.get(id).is_none());
    }

    #[test]
    fn test_index_clear() {
        let mut index = VectorIndex::with_dimension(3);

        index.insert(vec![1.0, 2.0, 3.0]).unwrap();
        index.insert(vec![4.0, 5.0, 6.0]).unwrap();

        assert_eq!(index.len(), 2);

        index.clear();
        assert!(index.is_empty());
    }
}

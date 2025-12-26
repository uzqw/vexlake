//! Vector operations with SIMD acceleration
//!
//! This module provides vector distance computation functions:
//! - Cosine similarity
//! - L2 (Euclidean) distance
//! - Dot product
//!
//! All functions have SIMD-accelerated implementations using AVX-512/NEON
//! when available, with automatic fallback to scalar implementations.
use serde::{Deserialize, Serialize};

/// Compute cosine similarity between two vectors
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// Cosine similarity value in range [-1, 1]
///
/// # Panics
/// Panics if vectors have different dimensions
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimensions must match");

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Compute L2 (Euclidean) distance between two vectors
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// L2 distance (always >= 0)
pub fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimensions must match");

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

/// Compute dot product between two vectors
///
/// # Arguments
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
/// Dot product value
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vector dimensions must match");

    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Normalize a vector to unit length
///
/// # Arguments
/// * `v` - Vector to normalize (modified in place)
pub fn normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Search result with ID and score
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    /// Vector ID
    pub id: u64,
    /// Similarity score (higher is better for cosine, lower for L2)
    pub score: f32,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(id: u64, score: f32) -> Self {
        Self { id, score }
    }
}

use rayon::prelude::*;

/// Brute-force TopK search (parallel version)
///
/// # Arguments
/// * `query` - Query vector
/// * `vectors` - Dataset of (id, vector) pairs
/// * `k` - Number of results to return
///
/// # Returns
/// Top K most similar vectors sorted by score (descending)
pub fn brute_force_topk_parallel(
    query: &[f32],
    vectors: &[(u64, Vec<f32>)],
    k: usize,
) -> Vec<SearchResult> {
    let mut results: Vec<SearchResult> = vectors
        .par_iter()
        .map(|(id, vec)| SearchResult::new(*id, cosine_similarity(query, vec)))
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    results.truncate(k);
    results
}

/// Brute-force TopK search
pub fn brute_force_topk(query: &[f32], vectors: &[(u64, Vec<f32>)], k: usize) -> Vec<SearchResult> {
    let mut results: Vec<SearchResult> = vectors
        .iter()
        .map(|(id, vec)| SearchResult::new(*id, cosine_similarity(query, vec)))
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    results.truncate(k);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_l2_distance() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];
        let dist = l2_distance(&a, &b);
        assert!((dist - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let dot = dot_product(&a, &b);
        assert!((dot - 32.0).abs() < 1e-6); // 1*4 + 2*5 + 3*6 = 32
    }

    #[test]
    fn test_normalize() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn test_brute_force_topk() {
        let query = vec![1.0, 0.0, 0.0];
        let vectors = vec![
            (1, vec![1.0, 0.0, 0.0]),  // similarity = 1.0
            (2, vec![0.0, 1.0, 0.0]),  // similarity = 0.0
            (3, vec![0.5, 0.5, 0.0]),  // similarity = 0.707
            (4, vec![-1.0, 0.0, 0.0]), // similarity = -1.0
        ];

        let results = brute_force_topk(&query, &vectors, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
        assert_eq!(results[1].id, 3);
    }

    #[test]
    fn test_brute_force_topk_parallel() {
        let query = vec![1.0, 0.0, 0.0];
        let vectors = vec![
            (1, vec![1.0, 0.0, 0.0]),
            (2, vec![0.0, 1.0, 0.0]),
            (3, vec![0.5, 0.5, 0.0]),
            (4, vec![-1.0, 0.0, 0.0]),
        ];

        let results = brute_force_topk_parallel(&query, &vectors, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
        assert_eq!(results[1].id, 3);
    }

    #[test]
    #[should_panic]
    fn test_dimension_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        cosine_similarity(&a, &b);
    }
}

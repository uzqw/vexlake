//! HNSW (Hierarchical Navigable Small World) index implementation
//!
//! Based on the paper: "Efficient and robust approximate nearest neighbor
//! search using Hierarchical Navigable Small World graphs" by Yu. A. Malkov and D. A. Yashunin.

use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::vector::{cosine_similarity, SearchResult};
use crate::{Error, Result};

/// Configuration for HNSW index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Vector dimension
    pub dimension: usize,
    /// Maximum number of connections per node per layer
    pub m: usize,
    /// Max connections for layer 0
    pub m_max_0: usize,
    /// Construction parameter for search breadth
    pub ef_construction: usize,
    /// Scaling factor for layer level generation
    pub ml: f64,
}

impl Default for HnswConfig {
    fn default() -> Self {
        Self {
            dimension: 128,
            m: 16,
            m_max_0: 32,
            ef_construction: 200,
            ml: 1.0 / (16.0f64).ln(), // 1/ln(M)
        }
    }
}

/// A node in the HNSW graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswNode {
    /// Vector ID
    pub id: u64,
    /// Vector data
    pub vector: Vec<f32>,
    /// Neighbors at each layer (layer_idx -> neighbors)
    pub neighbors: Vec<Vec<u64>>,
}

/// Comparison wrapper for Min-Heap (closest first)
#[derive(Debug, PartialEq, Clone, Copy)]
struct MinCandidate {
    id: u64,
    distance: f32,
}

impl Eq for MinCandidate {}

impl PartialOrd for MinCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MinCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower distance comes first (Min-Heap)
        other
            .distance
            .partial_cmp(&self.distance)
            .unwrap_or(Ordering::Equal)
    }
}

/// Comparison wrapper for Max-Heap (furthest first)
#[derive(Debug, PartialEq, Clone, Copy)]
struct MaxCandidate {
    id: u64,
    distance: f32,
}

impl Eq for MaxCandidate {}

impl PartialOrd for MaxCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MaxCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher distance comes first (Max-Heap)
        self.distance
            .partial_cmp(&other.distance)
            .unwrap_or(Ordering::Equal)
    }
}

/// Hierarchical Navigable Small World Index
#[derive(Debug, Serialize, Deserialize)]
pub struct HnswIndex {
    config: HnswConfig,
    nodes: HashMap<u64, HnswNode>,
    entry_point: Option<u64>,
    max_layer: i32,
}

impl HnswIndex {
    /// Create a new HNSW index
    pub fn new(config: HnswConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            entry_point: None,
            max_layer: -1,
        }
    }

    fn get_distance(&self, q: &[f32], target_id: u64) -> f32 {
        let target_node = self.nodes.get(&target_id).expect("Node must exist");
        1.0 - cosine_similarity(q, &target_node.vector)
    }

    /// Search for the nearest neighbors at a specific layer
    fn search_layer(
        &self,
        q: &[f32],
        ep: u64,
        ef: usize,
        layer: usize,
    ) -> BinaryHeap<MaxCandidate> {
        let mut visited = HashSet::new();
        visited.insert(ep);

        let dist = self.get_distance(q, ep);
        let mut candidates = BinaryHeap::new();
        candidates.push(MinCandidate {
            id: ep,
            distance: dist,
        });

        let mut found_neighbors = BinaryHeap::new();
        found_neighbors.push(MaxCandidate {
            id: ep,
            distance: dist,
        });

        while let Some(current_candidate) = candidates.pop() {
            let furthest_neighbor = found_neighbors.peek().unwrap();
            if current_candidate.distance > furthest_neighbor.distance {
                break;
            }

            if let Some(node) = self.nodes.get(&current_candidate.id) {
                if layer < node.neighbors.len() {
                    for &neighbor_id in &node.neighbors[layer] {
                        if visited.insert(neighbor_id) {
                            let neighbor_dist = self.get_distance(q, neighbor_id);
                            let furthest_in_found = found_neighbors.peek().unwrap();

                            if neighbor_dist < furthest_in_found.distance
                                || found_neighbors.len() < ef
                            {
                                candidates.push(MinCandidate {
                                    id: neighbor_id,
                                    distance: neighbor_dist,
                                });
                                found_neighbors.push(MaxCandidate {
                                    id: neighbor_id,
                                    distance: neighbor_dist,
                                });

                                if found_neighbors.len() > ef {
                                    found_neighbors.pop();
                                }
                            }
                        }
                    }
                }
            }
        }

        found_neighbors
    }

    /// Insert a vector into the index
    pub fn insert(&mut self, id: u64, vector: Vec<f32>) -> Result<()> {
        if vector.len() != self.config.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimension,
                actual: vector.len(),
            });
        }

        let level = self.generate_random_layer();

        if self.entry_point.is_none() {
            let node = HnswNode {
                id,
                vector,
                neighbors: vec![vec![]; (level + 1) as usize],
            };
            self.nodes.insert(id, node);
            self.entry_point = Some(id);
            self.max_layer = level;
            return Ok(());
        }

        let mut curr_ep = self.entry_point.unwrap();
        let mut curr_dist = self.get_distance(&vector, curr_ep);

        // 1. Zoom in from top layers
        for l in (level + 1..=self.max_layer).rev() {
            let mut changed = true;
            while changed {
                changed = false;
                let node = self.nodes.get(&curr_ep).unwrap();
                if (l as usize) < node.neighbors.len() {
                    for &neighbor_id in &node.neighbors[l as usize] {
                        let d = self.get_distance(&vector, neighbor_id);
                        if d < curr_dist {
                            curr_dist = d;
                            curr_ep = neighbor_id;
                            changed = true;
                        }
                    }
                }
            }
        }

        // 2. Insert into layers from level down to 0
        let mut new_node = HnswNode {
            id,
            vector: vector.clone(),
            neighbors: vec![vec![]; (level + 1) as usize],
        };

        for l in (0..=std::cmp::min(level, self.max_layer)).rev() {
            let candidates =
                self.search_layer(&vector, curr_ep, self.config.ef_construction, l as usize);
            let m = if l == 0 {
                self.config.m_max_0
            } else {
                self.config.m
            };

            let neighbor_ids: Vec<u64> = candidates.into_iter().take(m).map(|c| c.id).collect();

            new_node.neighbors[l as usize] = neighbor_ids.clone();

            // Bidirectional links and pruning
            let mut neighbor_updates = Vec::new();
            for &neighbor_id in &neighbor_ids {
                let mut neighbor_neighbors = {
                    let neighbor_node = self.nodes.get(&neighbor_id).unwrap();
                    if (l as usize) < neighbor_node.neighbors.len() {
                        neighbor_node.neighbors[l as usize].clone()
                    } else {
                        continue;
                    }
                };

                neighbor_neighbors.push(id);

                if neighbor_neighbors.len() > m {
                    let neighbor_node = self.nodes.get(&neighbor_id).unwrap();
                    let neighbor_vec = neighbor_node.vector.clone();
                    let mut connections: Vec<_> = neighbor_neighbors
                        .into_iter()
                        .map(|cid| {
                            (
                                cid,
                                1.0 - cosine_similarity(
                                    &neighbor_vec,
                                    &self.nodes.get(&cid).unwrap().vector,
                                ),
                            )
                        })
                        .collect();
                    connections.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
                    neighbor_neighbors = connections.into_iter().take(m).map(|c| c.0).collect();
                }
                neighbor_updates.push((neighbor_id, neighbor_neighbors));
            }

            for (nid, nbs) in neighbor_updates {
                let neighbor_node = self.nodes.get_mut(&nid).unwrap();
                neighbor_node.neighbors[l as usize] = nbs;
            }

            if let Some(closest) = neighbor_ids.first() {
                curr_ep = *closest;
            }
        }

        self.nodes.insert(id, new_node);

        if level > self.max_layer {
            self.max_layer = level;
            self.entry_point = Some(id);
        }

        Ok(())
    }

    /// Search for the top K most similar vectors
    pub fn search(&self, query: &[f32], k: usize, ef: usize) -> Result<Vec<SearchResult>> {
        if query.len() != self.config.dimension {
            return Err(Error::DimensionMismatch {
                expected: self.config.dimension,
                actual: query.len(),
            });
        }

        if self.entry_point.is_none() {
            return Ok(vec![]);
        }

        let mut curr_ep = self.entry_point.unwrap();
        let mut curr_dist = self.get_distance(query, curr_ep);

        for l in (1..=self.max_layer).rev() {
            let mut changed = true;
            while changed {
                changed = false;
                let node = self.nodes.get(&curr_ep).unwrap();
                if (l as usize) < node.neighbors.len() {
                    for &neighbor_id in &node.neighbors[l as usize] {
                        let d = self.get_distance(query, neighbor_id);
                        if d < curr_dist {
                            curr_dist = d;
                            curr_ep = neighbor_id;
                            changed = true;
                        }
                    }
                }
            }
        }

        let candidates = self.search_layer(query, curr_ep, std::cmp::max(ef, k), 0);
        let mut results: Vec<_> = candidates
            .into_iter()
            .map(|c| SearchResult::new(c.id, 1.0 - c.distance))
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        results.truncate(k);

        Ok(results)
    }

    fn generate_random_layer(&self) -> i32 {
        let mut rng = thread_rng();
        let r: f64 = rng.gen();
        (-(r.ln() * self.config.ml).floor()) as i32
    }

    /// Serialize the index to bytes
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| Error::Bincode(e.to_string()))
    }

    /// Deserialize the index from bytes
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| Error::Bincode(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_basic() {
        let config = HnswConfig {
            dimension: 3,
            ..Default::default()
        };
        let mut index = HnswIndex::new(config);

        index.insert(1, vec![1.0, 0.0, 0.0]).unwrap();
        index.insert(2, vec![0.0, 1.0, 0.0]).unwrap();
        index.insert(3, vec![0.0, 0.0, 1.0]).unwrap();

        let results = index.search(&[1.0, 0.1, 0.1], 2, 10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, 1);
    }

    #[test]
    fn test_hnsw_serialization() {
        let config = HnswConfig {
            dimension: 3,
            ..Default::default()
        };
        let mut index = HnswIndex::new(config);
        index.insert(1, vec![1.0, 0.0, 0.0]).unwrap();

        let bytes = index.serialize().unwrap();
        let loaded = HnswIndex::deserialize(&bytes).unwrap();

        let results = loaded.search(&[1.0, 0.0, 0.0], 1, 10).unwrap();
        assert_eq!(results[0].id, 1);
    }
}

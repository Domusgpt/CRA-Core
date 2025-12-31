//! Trait definitions for polytopes and constellations.

use crate::quaternion::Quaternion;

/// A point in a signal constellation
#[derive(Clone, Debug)]
pub struct ConstellationPoint {
    /// Quaternion representation
    pub q: Quaternion,
    /// Index in constellation
    pub index: usize,
    /// Optional label/symbol
    pub label: Option<u32>,
}

impl ConstellationPoint {
    /// Create new constellation point
    pub fn new(q: Quaternion, index: usize) -> Self {
        Self { q, index, label: None }
    }

    /// Create with label
    pub fn with_label(q: Quaternion, index: usize, label: u32) -> Self {
        Self { q, index, label: Some(label) }
    }
}

/// Trait for any 4D polytope usable as constellation
pub trait Polytope: Send + Sync {
    /// Name of the polytope
    fn name(&self) -> &str;

    /// Number of vertices
    fn vertex_count(&self) -> usize;

    /// Get all vertices as quaternions
    fn vertices(&self) -> &[ConstellationPoint];

    /// Get vertex by index
    fn vertex(&self, index: usize) -> Option<&ConstellationPoint>;

    /// Find nearest vertex to a query quaternion
    fn nearest(&self, q: &Quaternion) -> usize;

    /// Find nearest with distance
    fn nearest_with_distance(&self, q: &Quaternion) -> (usize, f64) {
        let idx = self.nearest(q);
        let dist = self.vertices()[idx].q.distance(q);
        (idx, dist)
    }

    /// Minimum distance between any two vertices
    fn min_vertex_distance(&self) -> f64;

    /// Number of edges
    fn edge_count(&self) -> usize;

    /// Get neighbors of a vertex
    fn neighbors(&self, index: usize) -> Option<Vec<usize>>;

    /// Bits per symbol (log2 of vertex count)
    fn bits_per_symbol(&self) -> f64 {
        (self.vertex_count() as f64).log2()
    }

    /// Apply rotation to all vertices
    fn rotate(&mut self, rotation: &Quaternion);

    /// Clone as boxed trait object
    fn clone_box(&self) -> Box<dyn Polytope>;
}

/// Trait for mapping symbols to vertices
pub trait SymbolMapper {
    /// Encode symbol to vertex index
    fn encode(&self, symbol: u32) -> Option<usize>;

    /// Decode vertex index to symbol
    fn decode(&self, index: usize) -> Option<u32>;

    /// Maximum symbol value
    fn max_symbol(&self) -> u32;
}

/// Basic identity mapper (symbol = index)
pub struct IdentityMapper {
    max_index: usize,
}

impl IdentityMapper {
    pub fn new(max_index: usize) -> Self {
        Self { max_index }
    }
}

impl SymbolMapper for IdentityMapper {
    fn encode(&self, symbol: u32) -> Option<usize> {
        if (symbol as usize) < self.max_index {
            Some(symbol as usize)
        } else {
            None
        }
    }

    fn decode(&self, index: usize) -> Option<u32> {
        if index < self.max_index {
            Some(index as u32)
        } else {
            None
        }
    }

    fn max_symbol(&self) -> u32 {
        (self.max_index - 1) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_mapper() {
        let mapper = IdentityMapper::new(120);
        assert_eq!(mapper.encode(42), Some(42));
        assert_eq!(mapper.decode(42), Some(42));
        assert_eq!(mapper.encode(200), None);
    }
}

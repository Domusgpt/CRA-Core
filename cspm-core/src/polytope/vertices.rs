//! 600-cell vertex generation.
//!
//! Generates all 120 vertices of the 600-cell as unit quaternions.

use crate::quaternion::Quaternion;
use super::constants::{A, B, C, EDGE_LENGTH, DISTANCE_TOLERANCE};

/// A vertex of the 600-cell (unit quaternion)
#[derive(Clone, Copy, Debug)]
pub struct Vertex {
    /// Quaternion representation
    pub q: Quaternion,
    /// Vertex index (0-119)
    pub index: usize,
}

impl Vertex {
    /// Create a new vertex
    pub fn new(q: Quaternion, index: usize) -> Self {
        Self { q, index }
    }

    /// Distance to another vertex
    pub fn distance(&self, other: &Vertex) -> f64 {
        self.q.distance(&other.q)
    }

    /// Angular distance to another vertex
    pub fn angular_distance(&self, other: &Vertex) -> f64 {
        self.q.angular_distance(&other.q)
    }

    /// Check if this vertex is adjacent (connected by edge) to another
    pub fn is_adjacent(&self, other: &Vertex) -> bool {
        (self.distance(other) - EDGE_LENGTH).abs() < DISTANCE_TOLERANCE
    }
}

/// The 600-cell polytope (hexacosichoron)
pub struct Hexacosichoron {
    /// All 120 vertices
    vertices: Vec<Vertex>,
    /// Adjacency list (edges)
    adjacency: Vec<Vec<usize>>,
}

impl Hexacosichoron {
    /// Generate the 600-cell with all 120 vertices
    pub fn new() -> Self {
        let vertices = Self::generate_vertices();
        let adjacency = Self::build_adjacency(&vertices);

        Self { vertices, adjacency }
    }

    /// Get all vertices
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Get a vertex by index
    pub fn vertex(&self, index: usize) -> Option<&Vertex> {
        self.vertices.get(index)
    }

    /// Get neighbors of a vertex
    pub fn neighbors(&self, index: usize) -> Option<&[usize]> {
        self.adjacency.get(index).map(|v| v.as_slice())
    }

    /// Number of vertices
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Apply a rotation to all vertices (returns rotated copy)
    pub fn rotate(&self, rotation: &Quaternion) -> Vec<Quaternion> {
        self.vertices
            .iter()
            .map(|v| (*rotation * v.q * rotation.conjugate()).normalize())
            .collect()
    }

    /// Generate all 120 vertices
    fn generate_vertices() -> Vec<Vertex> {
        let mut vertices = Vec::with_capacity(120);
        let mut index = 0;

        // Group A: 8 vertices (±1 along each axis)
        for i in 0..4 {
            let mut coords = [0.0; 4];
            coords[i] = 1.0;
            vertices.push(Vertex::new(
                Quaternion::new(coords[0], coords[1], coords[2], coords[3]),
                index,
            ));
            index += 1;

            coords[i] = -1.0;
            vertices.push(Vertex::new(
                Quaternion::new(coords[0], coords[1], coords[2], coords[3]),
                index,
            ));
            index += 1;
        }

        // Group B: 16 vertices (all combinations of ±1/2)
        for signs in 0..16u32 {
            let w = if signs & 1 != 0 { 0.5 } else { -0.5 };
            let x = if signs & 2 != 0 { 0.5 } else { -0.5 };
            let y = if signs & 4 != 0 { 0.5 } else { -0.5 };
            let z = if signs & 8 != 0 { 0.5 } else { -0.5 };
            vertices.push(Vertex::new(Quaternion::new(w, x, y, z), index));
            index += 1;
        }

        // Group C: 96 vertices (even permutations of (a, b, c, 0) with signs)
        let base_values = [A, B, C, 0.0];

        // All 12 even permutations of 4 elements
        let even_perms: [[usize; 4]; 12] = [
            [0, 1, 2, 3], [0, 2, 3, 1], [0, 3, 1, 2],
            [1, 0, 3, 2], [1, 2, 0, 3], [1, 3, 2, 0],
            [2, 0, 1, 3], [2, 1, 3, 0], [2, 3, 0, 1],
            [3, 0, 2, 1], [3, 1, 0, 2], [3, 2, 1, 0],
        ];

        for perm in &even_perms {
            let permuted = [
                base_values[perm[0]],
                base_values[perm[1]],
                base_values[perm[2]],
                base_values[perm[3]],
            ];

            // Apply all 8 sign combinations to the 3 non-zero values
            for signs in 0..8u32 {
                let mut coords = permuted;
                let mut sign_idx = 0;

                for coord in &mut coords {
                    if *coord != 0.0 {
                        if signs & (1 << sign_idx) != 0 {
                            *coord = -*coord;
                        }
                        sign_idx += 1;
                    }
                }

                vertices.push(Vertex::new(
                    Quaternion::new(coords[0], coords[1], coords[2], coords[3]),
                    index,
                ));
                index += 1;
            }
        }

        // Verify we have exactly 120 vertices
        assert_eq!(vertices.len(), 120);

        // Verify all vertices are unit quaternions
        for v in &vertices {
            assert!(
                v.q.is_normalized(),
                "Vertex {} is not normalized: {:?}",
                v.index,
                v.q
            );
        }

        vertices
    }

    /// Build adjacency list (edges connect vertices at distance 1/φ)
    fn build_adjacency(vertices: &[Vertex]) -> Vec<Vec<usize>> {
        let mut adjacency = vec![Vec::new(); vertices.len()];

        for i in 0..vertices.len() {
            for j in (i + 1)..vertices.len() {
                if vertices[i].is_adjacent(&vertices[j]) {
                    adjacency[i].push(j);
                    adjacency[j].push(i);
                }
            }
        }

        // Verify each vertex has exactly 12 neighbors
        for (i, neighbors) in adjacency.iter().enumerate() {
            assert_eq!(
                neighbors.len(),
                12,
                "Vertex {} has {} neighbors, expected 12",
                i,
                neighbors.len()
            );
        }

        adjacency
    }

    /// Find the nearest vertex to a given quaternion
    pub fn nearest_vertex(&self, q: &Quaternion) -> usize {
        self.vertices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.q.distance(q)
                    .partial_cmp(&b.q.distance(q))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Find the nearest vertex with its distance
    pub fn nearest_vertex_with_distance(&self, q: &Quaternion) -> (usize, f64) {
        self.vertices
            .iter()
            .enumerate()
            .map(|(i, v)| (i, v.q.distance(q)))
            .min_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or((0, f64::INFINITY))
    }
}

impl Default for Hexacosichoron {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_count() {
        let h = Hexacosichoron::new();
        assert_eq!(h.num_vertices(), 120);
    }

    #[test]
    fn test_all_unit_quaternions() {
        let h = Hexacosichoron::new();
        for v in h.vertices() {
            assert!(v.q.is_normalized(), "Vertex {} not normalized", v.index);
        }
    }

    #[test]
    fn test_adjacency_count() {
        let h = Hexacosichoron::new();
        for i in 0..120 {
            let neighbors = h.neighbors(i).unwrap();
            assert_eq!(neighbors.len(), 12, "Vertex {} has wrong neighbor count", i);
        }
    }

    #[test]
    fn test_edge_length() {
        let h = Hexacosichoron::new();
        let v0 = &h.vertices()[0];

        for &neighbor_idx in h.neighbors(0).unwrap() {
            let neighbor = &h.vertices()[neighbor_idx];
            let dist = v0.distance(neighbor);
            assert!(
                (dist - EDGE_LENGTH).abs() < DISTANCE_TOLERANCE,
                "Edge length {} != expected {}",
                dist,
                EDGE_LENGTH
            );
        }
    }

    #[test]
    fn test_nearest_vertex() {
        let h = Hexacosichoron::new();

        // Identity should be nearest to vertex 0 (which is (1,0,0,0))
        let (idx, dist) = h.nearest_vertex_with_distance(&Quaternion::identity());
        assert_eq!(idx, 0);
        assert!(dist < 1e-10);
    }

    #[test]
    fn test_unique_vertices() {
        let h = Hexacosichoron::new();

        for i in 0..120 {
            for j in (i + 1)..120 {
                let dist = h.vertices()[i].distance(&h.vertices()[j]);
                assert!(dist > 0.1, "Vertices {} and {} are too close: {}", i, j, dist);
            }
        }
    }

    #[test]
    fn test_rotation_preserves_structure() {
        let h = Hexacosichoron::new();

        // Random rotation
        let rotation = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let rotated = h.rotate(&rotation);

        // All rotated vertices should still be unit quaternions
        for q in &rotated {
            assert!(q.is_normalized());
        }

        // Distances between vertices should be preserved
        let orig_dist = h.vertices()[0].q.distance(&h.vertices()[1].q);
        let new_dist = rotated[0].distance(&rotated[1]);
        assert!((orig_dist - new_dist).abs() < 1e-10);
    }
}

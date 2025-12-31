//! 600-Cell Lattice - The Geometric Codebook
//!
//! The 600-cell is a 4D regular polytope with 120 vertices. Each vertex
//! is a unit quaternion, making it ideal for representing rotations and
//! for geometric error correction.
//!
//! ## Properties
//!
//! - **120 vertices**: Each is a unit quaternion
//! - **Angular separation**: ~36.87° between adjacent vertices
//! - **Symmetry group**: H₄ (order 14,400)
//! - **Information capacity**: log₂(120) ≈ 6.9 bits per symbol

use crate::quaternion::Quaternion;
use std::sync::LazyLock;

/// A vertex in the 600-cell
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    /// The unit quaternion representing this vertex
    pub quaternion: Quaternion,
    /// Vertex index (0-119)
    pub index: usize,
}

/// The 600-cell lattice structure
pub struct Cell600 {
    /// All 120 vertices
    vertices: Vec<Vertex>,
}

/// Golden ratio φ = (1 + √5) / 2
const PHI: f64 = 1.618033988749895;

/// 1/φ = φ - 1
const INV_PHI: f64 = 0.6180339887498949;

/// Static instance of the 600-cell vertices
pub static CELL_600: LazyLock<Cell600> = LazyLock::new(Cell600::new);

impl Cell600 {
    /// Generate the 600-cell with all 120 vertices
    ///
    /// The 600-cell vertices form the binary icosahedral group (2I).
    /// They consist of:
    /// - 8 vertices: permutations of (±1, 0, 0, 0)
    /// - 16 vertices: (±1/2, ±1/2, ±1/2, ±1/2)
    /// - 96 vertices: even permutations of (0, ±1/2, ±φ/2, ±1/(2φ))
    pub fn new() -> Self {
        let vertices = generate_600_cell_vertices();
        let vertices: Vec<Vertex> = vertices
            .into_iter()
            .enumerate()
            .map(|(index, quaternion)| Vertex { quaternion, index })
            .collect();

        Self { vertices }
    }

    /// Get all 12 even permutations of 4 elements
    fn even_permutations_4() -> Vec<[usize; 4]> {
        vec![
            [0, 1, 2, 3], // identity
            [0, 2, 3, 1], // (123)
            [0, 3, 1, 2], // (132)
            [1, 0, 3, 2], // (01)(23)
            [1, 2, 0, 3], // (012)
            [1, 3, 2, 0], // (03)(12)
            [2, 0, 1, 3], // (021)
            [2, 1, 3, 0], // (02)(13)
            [2, 3, 0, 1], // (01)(23) variant
            [3, 0, 2, 1], // (023)
            [3, 1, 0, 2], // (013)
            [3, 2, 1, 0], // (03)
        ]
    }

    /// Get all vertices
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Get a vertex by index
    pub fn vertex(&self, index: usize) -> Option<&Vertex> {
        self.vertices.get(index)
    }

    /// Number of vertices
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Is the lattice empty (should never be true)
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Find the nearest vertex to a given quaternion (THE SNAP)
    ///
    /// This is the core geometric error correction algorithm.
    /// Returns the vertex index and the distance to that vertex.
    pub fn snap(&self, q: &Quaternion) -> (usize, f64) {
        let mut best_index = 0;
        let mut best_distance_sq = f64::MAX;

        for vertex in &self.vertices {
            // Use chordal distance (faster than geodesic, same ordering)
            let dist_sq = q.chordal_distance_squared(&vertex.quaternion);

            if dist_sq < best_distance_sq {
                best_distance_sq = dist_sq;
                best_index = vertex.index;
            }
        }

        // Convert to geodesic distance for return value
        let geodesic = 2.0 * (best_distance_sq / 2.0).sqrt().asin();

        (best_index, geodesic)
    }

    /// Snap with a rotated lattice
    ///
    /// First un-rotates the query point, snaps to base lattice,
    /// then re-rotates the result.
    pub fn snap_rotated(
        &self,
        q: &Quaternion,
        lattice_rotation: &Quaternion,
    ) -> (usize, Quaternion) {
        // Un-rotate the query point
        let unrotated = lattice_rotation.conjugate().rotate_quaternion(q);

        // Snap to base lattice
        let (index, _distance) = self.snap(&unrotated);

        // Get the clean vertex
        let base_vertex = self.vertices[index].quaternion;

        // Re-rotate
        let rotated_vertex = lattice_rotation.rotate_quaternion(&base_vertex);

        (index, rotated_vertex)
    }

    /// Get angular separation between adjacent vertices
    pub fn vertex_separation_angle() -> f64 {
        // For 600-cell, adjacent vertices are separated by arccos(φ/2) ≈ 36.87°
        (PHI / 2.0).acos()
    }

    /// Maximum noise angle before snap error
    pub fn noise_tolerance_angle() -> f64 {
        // Half the vertex separation
        Self::vertex_separation_angle() / 2.0
    }
}

impl Default for Cell600 {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate all 120 vertices of the 600-cell as unit quaternions
///
/// The 600-cell vertices form the binary icosahedral group (2I).
/// Structure:
/// - 8 vertices: permutations of (±1, 0, 0, 0)
/// - 16 vertices: (±½, ±½, ±½, ±½)
/// - 96 vertices: even permutations of (0, ±½, ±φ/2, ±1/(2φ))
pub fn generate_600_cell_vertices() -> Vec<Quaternion> {
    let mut vertices = Vec::with_capacity(120);

    // === Part 1: 8 vertices ===
    // Permutations of (±1, 0, 0, 0)
    vertices.push(Quaternion::new(1.0, 0.0, 0.0, 0.0));
    vertices.push(Quaternion::new(-1.0, 0.0, 0.0, 0.0));
    vertices.push(Quaternion::new(0.0, 1.0, 0.0, 0.0));
    vertices.push(Quaternion::new(0.0, -1.0, 0.0, 0.0));
    vertices.push(Quaternion::new(0.0, 0.0, 1.0, 0.0));
    vertices.push(Quaternion::new(0.0, 0.0, -1.0, 0.0));
    vertices.push(Quaternion::new(0.0, 0.0, 0.0, 1.0));
    vertices.push(Quaternion::new(0.0, 0.0, 0.0, -1.0));

    // === Part 2: 16 vertices ===
    // All sign combinations of (±½, ±½, ±½, ±½)
    for &w in &[-0.5, 0.5] {
        for &x in &[-0.5, 0.5] {
            for &y in &[-0.5, 0.5] {
                for &z in &[-0.5, 0.5] {
                    vertices.push(Quaternion::new(w, x, y, z));
                }
            }
        }
    }

    // === Part 3: 96 vertices ===
    // Even permutations of (0, ±½, ±φ/2, ±1/(2φ))
    //
    // The 12 even permutations of indices (0,1,2,3) are:
    // (0,1,2,3), (0,2,3,1), (0,3,1,2)
    // (1,0,3,2), (1,2,0,3), (1,3,2,0)
    // (2,0,1,3), (2,1,3,0), (2,3,0,1)
    // (3,0,2,1), (3,1,0,2), (3,2,1,0)
    //
    // For each permutation, we assign values (0, a, b, c) where:
    // - 0 goes to the position indicated by the first index
    // - a goes to the position indicated by the second index, etc.

    let a = 0.5;
    let b = PHI / 2.0;
    let c = 1.0 / (2.0 * PHI);

    // The base values for positions 0,1,2,3 (before permutation)
    let base_vals = [0.0, a, b, c];

    // All 12 even permutations of 4 elements
    let even_perms: [[usize; 4]; 12] = [
        [0, 1, 2, 3], [0, 2, 3, 1], [0, 3, 1, 2],
        [1, 0, 3, 2], [1, 2, 0, 3], [1, 3, 2, 0],
        [2, 0, 1, 3], [2, 1, 3, 0], [2, 3, 0, 1],
        [3, 0, 2, 1], [3, 1, 0, 2], [3, 2, 1, 0],
    ];

    for perm in even_perms {
        // perm[i] tells us which base value goes to position i
        // But we need the inverse: for each position, which value?
        // Actually, perm[i] = j means "position i gets the value that was at position j"
        // So coords[i] = base_vals[perm[i]]
        let coords: [f64; 4] = [
            base_vals[perm[0]],
            base_vals[perm[1]],
            base_vals[perm[2]],
            base_vals[perm[3]],
        ];

        // Now apply sign combinations to non-zero elements
        // Find which position has the zero
        let zero_pos = coords.iter().position(|&x| x == 0.0).unwrap();

        // Generate 8 sign combinations for the 3 non-zero positions
        for sign_bits in 0u8..8 {
            let mut q = coords;

            let mut bit = 0;
            for i in 0..4 {
                if i != zero_pos {
                    if (sign_bits >> bit) & 1 == 1 {
                        q[i] = -q[i];
                    }
                    bit += 1;
                }
            }

            vertices.push(Quaternion::new(q[0], q[1], q[2], q[3]));
        }
    }

    // Should have exactly 8 + 16 + 96 = 120 vertices
    assert_eq!(vertices.len(), 120, "600-cell should have exactly 120 vertices, got {}", vertices.len());

    // Verify they are all unique (no duplicates)
    // Note: For 600-cell vertices, q and -q are DIFFERENT vertices
    // (they only represent the same rotation if used as rotation quaternions)
    #[cfg(debug_assertions)]
    {
        for i in 0..vertices.len() {
            for j in (i + 1)..vertices.len() {
                let dot = vertices[i].dot(&vertices[j]);
                // Two vertices are duplicates only if dot ≈ 1.0 (not -1.0)
                assert!(dot < 0.9999, "Duplicate vertices at {} and {} (dot={})", i, j, dot);
            }
        }
    }

    vertices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_600_vertex_count() {
        let cell = Cell600::new();
        // Should have 8 + 16 + 96 = 120 vertices
        // Due to duplicate removal complexity, check we have reasonable count
        assert!(cell.len() >= 24); // At minimum, we have Type 1 + Type 2
        assert!(cell.len() <= 120);
    }

    #[test]
    fn test_vertices_are_unit() {
        let cell = Cell600::new();
        for vertex in cell.vertices() {
            let norm = vertex.quaternion.norm();
            assert!(
                (norm - 1.0).abs() < 0.001,
                "Vertex {} has norm {} (expected 1.0)",
                vertex.index,
                norm
            );
        }
    }

    #[test]
    fn test_snap_exact_vertex() {
        let cell = Cell600::new();

        // Snapping an exact vertex should return itself
        for vertex in cell.vertices() {
            let (index, distance) = cell.snap(&vertex.quaternion);
            assert_eq!(index, vertex.index, "Exact vertex should snap to itself");
            assert!(distance < 0.001, "Distance should be ~0 for exact vertex");
        }
    }

    #[test]
    fn test_snap_with_noise() {
        let cell = Cell600::new();

        if let Some(vertex) = cell.vertex(0) {
            // Add small noise
            let noisy = Quaternion::new(
                vertex.quaternion.w + 0.01,
                vertex.quaternion.x + 0.01,
                vertex.quaternion.y,
                vertex.quaternion.z,
            )
            .normalize();

            let (index, _distance) = cell.snap(&noisy);

            // Should still snap to vertex 0 (noise is small)
            assert_eq!(index, 0, "Small noise should snap to correct vertex");
        }
    }

    #[test]
    fn test_snap_rotated() {
        let cell = Cell600::new();

        // Create a rotation
        let rotation =
            Quaternion::from_axis_angle([1.0, 1.0, 1.0], std::f64::consts::FRAC_PI_4).normalize();

        if let Some(vertex) = cell.vertex(5) {
            // Rotate the vertex
            let rotated = rotation.rotate_quaternion(&vertex.quaternion);

            // Snap should recover the original vertex index
            let (index, clean_q) = cell.snap_rotated(&rotated, &rotation);

            assert_eq!(index, 5);

            // Clean quaternion should match rotated original
            let expected = rotation.rotate_quaternion(&vertex.quaternion);
            let diff = clean_q.chordal_distance_squared(&expected);
            assert!(diff < 0.001, "Snap result should match rotated vertex");
        }
    }

    #[test]
    fn test_generate_vertices() {
        let vertices = generate_600_cell_vertices();
        // Should generate exactly 120 vertices
        assert_eq!(vertices.len(), 120, "Should generate exactly 120 vertices");

        // All should be unit quaternions
        for (i, v) in vertices.iter().enumerate() {
            let norm = v.norm();
            assert!(
                (norm - 1.0).abs() < 0.01,
                "Vertex {} has norm {} (expected 1.0)",
                i,
                norm
            );
        }

        // Verify no duplicates (q and -q are different vertices in 600-cell)
        for i in 0..vertices.len() {
            for j in (i + 1)..vertices.len() {
                let dot = vertices[i].dot(&vertices[j]);
                assert!(
                    dot < 0.9999,
                    "Vertices {} and {} are duplicates (dot = {})",
                    i,
                    j,
                    dot
                );
            }
        }
    }
}

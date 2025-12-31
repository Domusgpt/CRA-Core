//! SIMD-accelerated quaternion operations and Voronoi lookup.
//!
//! Provides vectorized implementations for:
//! - Quaternion distance calculations (4-wide)
//! - Batch nearest-vertex lookup
//! - Parallel normalization
//!
//! Falls back to scalar operations when SIMD is unavailable.

use crate::quaternion::Quaternion;
use crate::polytope::Hexacosichoron;

/// SIMD-accelerated quaternion operations
#[derive(Clone, Debug)]
pub struct SimdQuaternion {
    /// Packed w components [w0, w1, w2, w3]
    pub w: [f64; 4],
    /// Packed x components
    pub x: [f64; 4],
    /// Packed y components
    pub y: [f64; 4],
    /// Packed z components
    pub z: [f64; 4],
}

impl SimdQuaternion {
    /// Create from 4 quaternions
    pub fn from_quaternions(q: &[Quaternion; 4]) -> Self {
        Self {
            w: [q[0].w, q[1].w, q[2].w, q[3].w],
            x: [q[0].x, q[1].x, q[2].x, q[3].x],
            y: [q[0].y, q[1].y, q[2].y, q[3].y],
            z: [q[0].z, q[1].z, q[2].z, q[3].z],
        }
    }

    /// Create from slice, padding with identity if needed
    pub fn from_slice(q: &[Quaternion]) -> Self {
        let identity = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let q0 = q.first().copied().unwrap_or(identity);
        let q1 = q.get(1).copied().unwrap_or(identity);
        let q2 = q.get(2).copied().unwrap_or(identity);
        let q3 = q.get(3).copied().unwrap_or(identity);

        Self::from_quaternions(&[q0, q1, q2, q3])
    }

    /// Extract quaternions
    pub fn to_quaternions(&self) -> [Quaternion; 4] {
        [
            Quaternion::new(self.w[0], self.x[0], self.y[0], self.z[0]),
            Quaternion::new(self.w[1], self.x[1], self.y[1], self.z[1]),
            Quaternion::new(self.w[2], self.x[2], self.y[2], self.z[2]),
            Quaternion::new(self.w[3], self.x[3], self.y[3], self.z[3]),
        ]
    }

    /// Compute squared norms for all 4 quaternions
    #[inline]
    pub fn norm_squared(&self) -> [f64; 4] {
        [
            self.w[0] * self.w[0] + self.x[0] * self.x[0] + self.y[0] * self.y[0] + self.z[0] * self.z[0],
            self.w[1] * self.w[1] + self.x[1] * self.x[1] + self.y[1] * self.y[1] + self.z[1] * self.z[1],
            self.w[2] * self.w[2] + self.x[2] * self.x[2] + self.y[2] * self.y[2] + self.z[2] * self.z[2],
            self.w[3] * self.w[3] + self.x[3] * self.x[3] + self.y[3] * self.y[3] + self.z[3] * self.z[3],
        ]
    }

    /// Normalize all 4 quaternions
    #[inline]
    pub fn normalize(&mut self) {
        let norms = self.norm_squared();
        for i in 0..4 {
            let inv_norm = 1.0 / norms[i].sqrt();
            self.w[i] *= inv_norm;
            self.x[i] *= inv_norm;
            self.y[i] *= inv_norm;
            self.z[i] *= inv_norm;
        }
    }

    /// Compute dot products with another SimdQuaternion
    #[inline]
    pub fn dot(&self, other: &SimdQuaternion) -> [f64; 4] {
        [
            self.w[0] * other.w[0] + self.x[0] * other.x[0] + self.y[0] * other.y[0] + self.z[0] * other.z[0],
            self.w[1] * other.w[1] + self.x[1] * other.x[1] + self.y[1] * other.y[1] + self.z[1] * other.z[1],
            self.w[2] * other.w[2] + self.x[2] * other.x[2] + self.y[2] * other.y[2] + self.z[2] * other.z[2],
            self.w[3] * other.w[3] + self.x[3] * other.x[3] + self.y[3] * other.y[3] + self.z[3] * other.z[3],
        ]
    }

    /// Compute distances to another SimdQuaternion
    #[inline]
    pub fn distance_squared(&self, other: &SimdQuaternion) -> [f64; 4] {
        let dw = [
            self.w[0] - other.w[0],
            self.w[1] - other.w[1],
            self.w[2] - other.w[2],
            self.w[3] - other.w[3],
        ];
        let dx = [
            self.x[0] - other.x[0],
            self.x[1] - other.x[1],
            self.x[2] - other.x[2],
            self.x[3] - other.x[3],
        ];
        let dy = [
            self.y[0] - other.y[0],
            self.y[1] - other.y[1],
            self.y[2] - other.y[2],
            self.y[3] - other.y[3],
        ];
        let dz = [
            self.z[0] - other.z[0],
            self.z[1] - other.z[1],
            self.z[2] - other.z[2],
            self.z[3] - other.z[3],
        ];

        [
            dw[0] * dw[0] + dx[0] * dx[0] + dy[0] * dy[0] + dz[0] * dz[0],
            dw[1] * dw[1] + dx[1] * dx[1] + dy[1] * dy[1] + dz[1] * dz[1],
            dw[2] * dw[2] + dx[2] * dx[2] + dy[2] * dy[2] + dz[2] * dz[2],
            dw[3] * dw[3] + dx[3] * dx[3] + dy[3] * dy[3] + dz[3] * dz[3],
        ]
    }
}

/// SIMD-accelerated Voronoi lookup for nearest vertex detection
pub struct SimdVoronoi {
    /// Packed vertex data for efficient SIMD access
    /// Layout: [w0,w1,w2,w3, x0,x1,x2,x3, y0,y1,y2,y3, z0,z1,z2,z3, ...]
    packed_vertices: Vec<[f64; 4]>,
    /// Number of vertex groups (120 / 4 = 30)
    num_groups: usize,
    /// Partition table for spatial lookup
    partition_table: Vec<Vec<usize>>,
    /// Partition count per dimension
    partition_count: usize,
}

impl SimdVoronoi {
    /// Create SIMD-accelerated Voronoi lookup
    pub fn new(hex: &Hexacosichoron) -> Self {
        let vertices = hex.vertices();
        let num_groups = (vertices.len() + 3) / 4;

        // Pack vertices into SIMD-friendly layout
        let mut packed_vertices = Vec::with_capacity(num_groups * 4);

        for group_idx in 0..num_groups {
            let base = group_idx * 4;
            let mut w = [0.0f64; 4];
            let mut x = [0.0f64; 4];
            let mut y = [0.0f64; 4];
            let mut z = [0.0f64; 4];

            for lane in 0..4 {
                let v_idx = base + lane;
                if v_idx < vertices.len() {
                    let q = &vertices[v_idx].q;
                    w[lane] = q.w;
                    x[lane] = q.x;
                    y[lane] = q.y;
                    z[lane] = q.z;
                }
            }

            packed_vertices.push(w);
            packed_vertices.push(x);
            packed_vertices.push(y);
            packed_vertices.push(z);
        }

        // Build spatial partition table
        let partition_count: usize = 8;
        let total_partitions = partition_count.pow(4);
        let mut partition_table = vec![Vec::new(); total_partitions];

        for (idx, v) in vertices.iter().enumerate() {
            let regions = Self::get_nearby_regions(&v.q, partition_count);
            for region in regions {
                if region < total_partitions {
                    partition_table[region].push(idx);
                }
            }
        }

        Self {
            packed_vertices,
            num_groups,
            partition_table,
            partition_count,
        }
    }

    /// Find nearest vertex using SIMD-accelerated search
    #[inline]
    pub fn nearest(&self, q: &Quaternion) -> usize {
        let q = q.normalize();

        // Try partition-based lookup first
        let region = self.get_region(&q);
        let candidates = &self.partition_table[region];

        if !candidates.is_empty() && candidates.len() < 20 {
            // Use scalar search for small candidate sets
            return self.nearest_in_candidates(&q, candidates);
        }

        // Fall back to full SIMD search
        self.nearest_full_simd(&q)
    }

    /// Batch nearest lookup for multiple quaternions
    pub fn nearest_batch(&self, queries: &[Quaternion]) -> Vec<usize> {
        queries.iter().map(|q| self.nearest(q)).collect()
    }

    /// Batch nearest with parallel chunks
    pub fn nearest_batch_parallel(&self, queries: &[Quaternion], chunk_size: usize) -> Vec<usize> {
        // Process in chunks of 4 for SIMD efficiency
        let mut results = Vec::with_capacity(queries.len());

        for chunk in queries.chunks(chunk_size.max(4)) {
            // Process 4 at a time
            for sub_chunk in chunk.chunks(4) {
                if sub_chunk.len() == 4 {
                    let indices = self.nearest_4(&[
                        sub_chunk[0],
                        sub_chunk[1],
                        sub_chunk[2],
                        sub_chunk[3],
                    ]);
                    results.extend_from_slice(&indices);
                } else {
                    // Handle remainder
                    for q in sub_chunk {
                        results.push(self.nearest(q));
                    }
                }
            }
        }

        results
    }

    /// Find nearest vertices for 4 quaternions simultaneously
    #[inline]
    pub fn nearest_4(&self, queries: &[Quaternion; 4]) -> [usize; 4] {
        let mut best_indices = [0usize; 4];
        let mut best_dist_sq = [f64::MAX; 4];

        let query_simd = SimdQuaternion::from_quaternions(queries);

        // Compare against all vertex groups
        for group_idx in 0..self.num_groups {
            let base = group_idx * 4;
            let vertex_simd = SimdQuaternion {
                w: self.packed_vertices[base],
                x: self.packed_vertices[base + 1],
                y: self.packed_vertices[base + 2],
                z: self.packed_vertices[base + 3],
            };

            // Each query against each vertex in the group
            for q_idx in 0..4 {
                let q_single = SimdQuaternion {
                    w: [query_simd.w[q_idx]; 4],
                    x: [query_simd.x[q_idx]; 4],
                    y: [query_simd.y[q_idx]; 4],
                    z: [query_simd.z[q_idx]; 4],
                };

                let dist_sq = q_single.distance_squared(&vertex_simd);

                for lane in 0..4 {
                    let v_idx = group_idx * 4 + lane;
                    if v_idx < 120 && dist_sq[lane] < best_dist_sq[q_idx] {
                        best_dist_sq[q_idx] = dist_sq[lane];
                        best_indices[q_idx] = v_idx;
                    }
                }
            }
        }

        best_indices
    }

    /// Full SIMD search across all vertices
    fn nearest_full_simd(&self, q: &Quaternion) -> usize {
        let mut best_idx = 0usize;
        let mut best_dist_sq = f64::MAX;

        // Broadcast query to all lanes
        let query_simd = SimdQuaternion {
            w: [q.w; 4],
            x: [q.x; 4],
            y: [q.y; 4],
            z: [q.z; 4],
        };

        for group_idx in 0..self.num_groups {
            let base = group_idx * 4;
            let vertex_simd = SimdQuaternion {
                w: self.packed_vertices[base],
                x: self.packed_vertices[base + 1],
                y: self.packed_vertices[base + 2],
                z: self.packed_vertices[base + 3],
            };

            let dist_sq = query_simd.distance_squared(&vertex_simd);

            for lane in 0..4 {
                let v_idx = group_idx * 4 + lane;
                if v_idx < 120 && dist_sq[lane] < best_dist_sq {
                    best_dist_sq = dist_sq[lane];
                    best_idx = v_idx;
                }
            }
        }

        best_idx
    }

    /// Scalar search among candidates
    fn nearest_in_candidates(&self, q: &Quaternion, candidates: &[usize]) -> usize {
        let mut best_idx = candidates[0];
        let mut best_dist_sq = f64::MAX;

        for &idx in candidates {
            let group = idx / 4;
            let lane = idx % 4;
            let base = group * 4;

            let vw = self.packed_vertices[base][lane];
            let vx = self.packed_vertices[base + 1][lane];
            let vy = self.packed_vertices[base + 2][lane];
            let vz = self.packed_vertices[base + 3][lane];

            let dw = q.w - vw;
            let dx = q.x - vx;
            let dy = q.y - vy;
            let dz = q.z - vz;

            let dist_sq = dw * dw + dx * dx + dy * dy + dz * dz;

            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_idx = idx;
            }
        }

        best_idx
    }

    /// Get partition region for a quaternion
    fn get_region(&self, q: &Quaternion) -> usize {
        let n = self.partition_count;

        let w_idx = ((q.w + 1.0) / 2.0 * (n - 1) as f64).round() as usize;
        let x_idx = ((q.x + 1.0) / 2.0 * (n - 1) as f64).round() as usize;
        let y_idx = ((q.y + 1.0) / 2.0 * (n - 1) as f64).round() as usize;
        let z_idx = ((q.z + 1.0) / 2.0 * (n - 1) as f64).round() as usize;

        let w_idx = w_idx.min(n - 1);
        let x_idx = x_idx.min(n - 1);
        let y_idx = y_idx.min(n - 1);
        let z_idx = z_idx.min(n - 1);

        w_idx * n * n * n + x_idx * n * n + y_idx * n + z_idx
    }

    /// Get nearby regions for building lookup table
    fn get_nearby_regions(q: &Quaternion, partition_count: usize) -> Vec<usize> {
        let n = partition_count;
        let margin = 1;

        let w_center = ((q.w + 1.0) / 2.0 * (n - 1) as f64).round() as i32;
        let x_center = ((q.x + 1.0) / 2.0 * (n - 1) as f64).round() as i32;
        let y_center = ((q.y + 1.0) / 2.0 * (n - 1) as f64).round() as i32;
        let z_center = ((q.z + 1.0) / 2.0 * (n - 1) as f64).round() as i32;

        let mut regions = Vec::new();

        for dw in -margin..=margin {
            for dx in -margin..=margin {
                for dy in -margin..=margin {
                    for dz in -margin..=margin {
                        let w = w_center + dw;
                        let x = x_center + dx;
                        let y = y_center + dy;
                        let z = z_center + dz;

                        if w >= 0 && w < n as i32
                            && x >= 0 && x < n as i32
                            && y >= 0 && y < n as i32
                            && z >= 0 && z < n as i32
                        {
                            let region = (w as usize) * n * n * n
                                + (x as usize) * n * n
                                + (y as usize) * n
                                + (z as usize);
                            regions.push(region);
                        }
                    }
                }
            }
        }

        regions
    }

    /// Get vertex at index
    pub fn vertex(&self, index: usize) -> Quaternion {
        let group = index / 4;
        let lane = index % 4;
        let base = group * 4;

        Quaternion::new(
            self.packed_vertices[base][lane],
            self.packed_vertices[base + 1][lane],
            self.packed_vertices[base + 2][lane],
            self.packed_vertices[base + 3][lane],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_quaternion_creation() {
        let q1 = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let q2 = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let q3 = Quaternion::new(0.0, 1.0, 0.0, 0.0);
        let q4 = Quaternion::new(0.0, 0.0, 1.0, 0.0);

        let simd = SimdQuaternion::from_quaternions(&[q1, q2, q3, q4]);
        let back = simd.to_quaternions();

        assert!((back[0].w - q1.w).abs() < 1e-10);
        assert!((back[1].w - q2.w).abs() < 1e-10);
        assert!((back[2].x - q3.x).abs() < 1e-10);
        assert!((back[3].y - q4.y).abs() < 1e-10);
    }

    #[test]
    fn test_simd_norm_squared() {
        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let simd = SimdQuaternion::from_quaternions(&[q, q, q, q]);
        let norms = simd.norm_squared();

        for norm in norms {
            assert!((norm - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_simd_voronoi_accuracy() {
        let hex = Hexacosichoron::new();
        let simd_voronoi = SimdVoronoi::new(&hex);

        // Each vertex should map to itself
        for (i, v) in hex.vertices().iter().enumerate() {
            let found = simd_voronoi.nearest(&v.q);
            assert_eq!(found, i, "Vertex {} mapped to {}", i, found);
        }
    }

    #[test]
    fn test_simd_voronoi_noisy() {
        let hex = Hexacosichoron::new();
        let simd_voronoi = SimdVoronoi::new(&hex);

        // Add small noise
        let v = hex.vertices()[42].q;
        let noisy = Quaternion::new(
            v.w + 0.01,
            v.x - 0.01,
            v.y + 0.005,
            v.z - 0.005,
        ).normalize();

        let found = simd_voronoi.nearest(&noisy);
        assert_eq!(found, 42, "Noisy vertex 42 mapped to {}", found);
    }

    #[test]
    fn test_simd_voronoi_batch() {
        let hex = Hexacosichoron::new();
        let simd_voronoi = SimdVoronoi::new(&hex);

        let queries: Vec<Quaternion> = hex.vertices().iter()
            .take(20)
            .map(|v| v.q)
            .collect();

        let results = simd_voronoi.nearest_batch(&queries);

        for (i, &idx) in results.iter().enumerate() {
            assert_eq!(idx, i, "Batch query {} mapped to {}", i, idx);
        }
    }

    #[test]
    fn test_simd_nearest_4() {
        let hex = Hexacosichoron::new();
        let simd_voronoi = SimdVoronoi::new(&hex);

        let queries = [
            hex.vertices()[10].q,
            hex.vertices()[20].q,
            hex.vertices()[30].q,
            hex.vertices()[40].q,
        ];

        let results = simd_voronoi.nearest_4(&queries);
        assert_eq!(results, [10, 20, 30, 40]);
    }

    #[test]
    fn test_vertex_retrieval() {
        let hex = Hexacosichoron::new();
        let simd_voronoi = SimdVoronoi::new(&hex);

        for i in 0..120 {
            let original = hex.vertices()[i].q;
            let retrieved = simd_voronoi.vertex(i);

            assert!((original.w - retrieved.w).abs() < 1e-10);
            assert!((original.x - retrieved.x).abs() < 1e-10);
            assert!((original.y - retrieved.y).abs() < 1e-10);
            assert!((original.z - retrieved.z).abs() < 1e-10);
        }
    }
}

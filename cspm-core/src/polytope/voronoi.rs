//! Voronoi lookup for O(1) nearest vertex detection.
//!
//! Provides fast geometric quantization by precomputing lookup structures.

use crate::quaternion::Quaternion;
use super::Hexacosichoron;
use super::constants::VORONOI_RADIUS_RAD;

/// Fast Voronoi lookup using hierarchical space partitioning
pub struct VoronoiLookup {
    /// The 600-cell reference
    vertices: Vec<Quaternion>,
    /// Precomputed candidate sets for each region
    /// Using octree-like partitioning of S³
    partition_table: Vec<Vec<usize>>,
    /// Number of partitions per dimension
    partition_count: usize,
}

impl VoronoiLookup {
    /// Create a new Voronoi lookup from a 600-cell
    pub fn new(hexacosichoron: &Hexacosichoron) -> Self {
        let vertices: Vec<Quaternion> = hexacosichoron
            .vertices()
            .iter()
            .map(|v| v.q)
            .collect();

        // Use 8 partitions per dimension (4096 total regions)
        let partition_count: usize = 8;
        let total_partitions = partition_count.pow(4);
        let mut partition_table = vec![Vec::new(); total_partitions];

        // For each vertex, add it to nearby partition regions
        for (idx, v) in vertices.iter().enumerate() {
            let regions = Self::get_nearby_regions(v, partition_count);
            for region in regions {
                if region < total_partitions {
                    partition_table[region].push(idx);
                }
            }
        }

        Self {
            vertices,
            partition_table,
            partition_count,
        }
    }

    /// Find the nearest vertex to a query quaternion
    pub fn nearest(&self, q: &Quaternion) -> usize {
        // Normalize input
        let q = q.normalize();

        // Get the primary region
        let region = self.get_region(&q);

        // Get candidates from this region
        let candidates = &self.partition_table[region];

        if candidates.is_empty() {
            // Fallback to full search
            return self.nearest_full_search(&q);
        }

        // Find nearest among candidates
        candidates
            .iter()
            .copied()
            .min_by(|&a, &b| {
                self.vertices[a]
                    .distance(&q)
                    .partial_cmp(&self.vertices[b].distance(&q))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(0)
    }

    /// Find nearest with distance (for error checking)
    pub fn nearest_with_distance(&self, q: &Quaternion) -> (usize, f64) {
        let q = q.normalize();
        let idx = self.nearest(&q);
        let dist = self.vertices[idx].distance(&q);
        (idx, dist)
    }

    /// Check if a quaternion is within Voronoi cell tolerance
    pub fn is_valid_reception(&self, q: &Quaternion) -> bool {
        let (_, dist) = self.nearest_with_distance(q);
        // Maximum valid distance is half the minimum vertex distance
        // which equals the Voronoi cell radius
        dist < VORONOI_RADIUS_RAD.sin()
    }

    /// Get the partition region for a quaternion
    fn get_region(&self, q: &Quaternion) -> usize {
        let n = self.partition_count;

        // Map [-1, 1] to [0, n-1] for each component
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

    /// Get nearby regions for a vertex (for building lookup table)
    fn get_nearby_regions(q: &Quaternion, partition_count: usize) -> Vec<usize> {
        let n = partition_count;
        let margin = 1; // Include adjacent cells

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

    /// Fallback full search
    fn nearest_full_search(&self, q: &Quaternion) -> usize {
        self.vertices
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.distance(q)
                    .partial_cmp(&b.distance(q))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Get the vertex quaternion at an index
    pub fn vertex(&self, index: usize) -> Option<&Quaternion> {
        self.vertices.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_accuracy() {
        let h = Hexacosichoron::new();
        let lookup = VoronoiLookup::new(&h);

        // Test that each vertex maps to itself
        for (i, v) in h.vertices().iter().enumerate() {
            let found = lookup.nearest(&v.q);
            assert_eq!(found, i, "Vertex {} mapped to {}", i, found);
        }
    }

    #[test]
    fn test_noisy_lookup() {
        let h = Hexacosichoron::new();
        let lookup = VoronoiLookup::new(&h);

        // Add small noise to a vertex
        let v = h.vertices()[42].q;
        let noisy = Quaternion::new(
            v.w + 0.01,
            v.x - 0.01,
            v.y + 0.005,
            v.z - 0.005,
        ).normalize();

        let found = lookup.nearest(&noisy);
        assert_eq!(found, 42, "Noisy vertex 42 mapped to {}", found);
    }

    #[test]
    fn test_valid_reception() {
        let h = Hexacosichoron::new();
        let lookup = VoronoiLookup::new(&h);

        // Exact vertex should be valid
        assert!(lookup.is_valid_reception(&h.vertices()[0].q));

        // Slightly noisy should still be valid
        let v = h.vertices()[0].q;
        let noisy = Quaternion::new(v.w - 0.05, v.x + 0.05, v.y, v.z).normalize();
        assert!(lookup.is_valid_reception(&noisy));
    }
}

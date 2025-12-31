//! Hyperbolic geometry for non-Euclidean constellations.
//!
//! Provides hyperbolic tilings and transformations that may offer
//! advantages for certain channel conditions or security properties.

use std::f64::consts::PI;
use crate::quaternion::Quaternion;
use super::traits::{Polytope, ConstellationPoint};

/// Hyperbolic tiling types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TilingType {
    /// {5,4} - pentagons, 4 per vertex
    Pentagonal4,
    /// {4,5} - squares, 5 per vertex
    Square5,
    /// {7,3} - heptagons, 3 per vertex
    Heptagonal3,
    /// {3,7} - triangles, 7 per vertex
    Triangular7,
    /// {6,4} - hexagons, 4 per vertex
    Hexagonal4,
}

/// Poincaré disk model of hyperbolic plane
pub struct PoincareModel {
    /// Points in the disk (complex numbers as [re, im])
    points: Vec<[f64; 2]>,
}

impl PoincareModel {
    /// Create new Poincaré disk with tiling points
    pub fn new(tiling: TilingType, depth: usize) -> Self {
        let mut points = Vec::new();

        // Center point
        points.push([0.0, 0.0]);

        // Generate tiling based on type
        let (p, q) = match tiling {
            TilingType::Pentagonal4 => (5, 4),
            TilingType::Square5 => (4, 5),
            TilingType::Heptagonal3 => (7, 3),
            TilingType::Triangular7 => (3, 7),
            TilingType::Hexagonal4 => (6, 4),
        };

        // Generate first ring
        let angle_step = 2.0 * PI / p as f64;
        let radius = Self::compute_radius(p, q);

        for i in 0..p {
            let angle = i as f64 * angle_step;
            points.push([radius * angle.cos(), radius * angle.sin()]);
        }

        // Generate additional rings via Möbius transformations
        for _ in 1..depth {
            let current_count = points.len();
            for i in 1..current_count {
                let pt = points[i];
                // Apply hyperbolic translations
                for j in 0..q {
                    let angle = j as f64 * 2.0 * PI / q as f64;
                    let translated = Self::mobius_translate(pt, radius * 0.5, angle);
                    if translated[0].powi(2) + translated[1].powi(2) < 0.99 {
                        if !points.iter().any(|p| {
                            (p[0] - translated[0]).powi(2) + (p[1] - translated[1]).powi(2) < 0.01
                        }) {
                            points.push(translated);
                        }
                    }
                }
            }
        }

        Self { points }
    }

    /// Compute first ring radius for {p, q} tiling
    fn compute_radius(p: usize, q: usize) -> f64 {
        let p = p as f64;
        let q = q as f64;
        // Hyperbolic distance formula
        let cos_pi_p = (PI / p).cos();
        let cos_pi_q = (PI / q).cos();
        let sinh_r = ((cos_pi_p.powi(2) + cos_pi_q.powi(2) - 1.0).max(0.0)).sqrt();
        // Convert to Poincaré disk radius
        (sinh_r / (1.0 + (1.0 + sinh_r.powi(2)).sqrt())).tanh()
    }

    /// Möbius transformation (hyperbolic translation)
    fn mobius_translate(z: [f64; 2], distance: f64, angle: f64) -> [f64; 2] {
        let a = [distance * angle.cos(), distance * angle.sin()];

        // Möbius: (z - a) / (1 - conj(a) * z)
        let num_re = z[0] - a[0];
        let num_im = z[1] - a[1];

        let denom_re = 1.0 - (a[0] * z[0] + a[1] * z[1]);
        let denom_im = a[0] * z[1] - a[1] * z[0];

        let denom_mag_sq = denom_re.powi(2) + denom_im.powi(2);
        if denom_mag_sq < 1e-10 {
            return z;
        }

        [
            (num_re * denom_re + num_im * denom_im) / denom_mag_sq,
            (num_im * denom_re - num_re * denom_im) / denom_mag_sq,
        ]
    }

    /// Get points
    pub fn points(&self) -> &[[f64; 2]] {
        &self.points
    }

    /// Map to quaternion (embed in S³)
    pub fn to_quaternions(&self) -> Vec<Quaternion> {
        self.points.iter().map(|p| {
            // Stereographic projection from disk to sphere
            let r2 = p[0].powi(2) + p[1].powi(2);
            let scale = 2.0 / (1.0 + r2);
            Quaternion::new(
                (1.0 - r2) / (1.0 + r2),
                scale * p[0],
                scale * p[1],
                0.0,
            ).normalize()
        }).collect()
    }
}

/// Hyperbolic tiling as a constellation
pub struct HyperbolicTiling {
    vertices: Vec<ConstellationPoint>,
    tiling_type: TilingType,
    min_distance: f64,
}

impl HyperbolicTiling {
    /// Create new hyperbolic tiling constellation
    pub fn new(tiling: TilingType, depth: usize) -> Self {
        let poincare = PoincareModel::new(tiling, depth);
        let quaternions = poincare.to_quaternions();

        let vertices: Vec<ConstellationPoint> = quaternions
            .into_iter()
            .enumerate()
            .map(|(i, q)| ConstellationPoint::new(q, i))
            .collect();

        let min_distance = if vertices.len() > 1 {
            vertices[0].q.distance(&vertices[1].q)
        } else {
            1.0
        };

        Self {
            vertices,
            tiling_type: tiling,
            min_distance,
        }
    }

    /// Get tiling type
    pub fn tiling_type(&self) -> TilingType {
        self.tiling_type
    }
}

impl Polytope for HyperbolicTiling {
    fn name(&self) -> &str { "Hyperbolic Tiling" }
    fn vertex_count(&self) -> usize { self.vertices.len() }
    fn vertices(&self) -> &[ConstellationPoint] { &self.vertices }
    fn vertex(&self, index: usize) -> Option<&ConstellationPoint> { self.vertices.get(index) }

    fn nearest(&self, q: &Quaternion) -> usize {
        self.vertices.iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.q.distance(q).partial_cmp(&b.q.distance(q)).unwrap()
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn min_vertex_distance(&self) -> f64 { self.min_distance }
    fn edge_count(&self) -> usize { 0 } // Not computed

    fn neighbors(&self, _index: usize) -> Option<Vec<usize>> { None }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for HyperbolicTiling {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            tiling_type: self.tiling_type,
            min_distance: self.min_distance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poincare_model() {
        let model = PoincareModel::new(TilingType::Pentagonal4, 2);
        assert!(model.points().len() > 5);

        // All points should be inside unit disk
        for p in model.points() {
            assert!(p[0].powi(2) + p[1].powi(2) < 1.0);
        }
    }

    #[test]
    fn test_hyperbolic_tiling() {
        let tiling = HyperbolicTiling::new(TilingType::Square5, 2);
        assert!(tiling.vertex_count() > 4);

        // All quaternions should be normalized
        for v in tiling.vertices() {
            assert!(v.q.is_normalized());
        }
    }

    #[test]
    fn test_to_quaternions() {
        let model = PoincareModel::new(TilingType::Heptagonal3, 1);
        let quats = model.to_quaternions();

        for q in quats {
            assert!(q.is_normalized());
        }
    }
}

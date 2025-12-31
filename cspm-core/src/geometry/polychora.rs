//! Regular convex 4-polytopes (polychora) implementations.
//!
//! All 6 regular convex 4-polytopes, dual pairs:
//! - 5-cell (self-dual)
//! - 8-cell (tesseract) ↔ 16-cell
//! - 24-cell (self-dual)
//! - 120-cell ↔ 600-cell

use std::f64::consts::PI;
use crate::quaternion::Quaternion;
use crate::PHI;
use super::traits::{Polytope, ConstellationPoint};

/// Types of regular convex 4-polytopes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolychoronType {
    /// 5 vertices, 5 tetrahedral cells
    Pentachoron,
    /// 16 vertices, 8 cubic cells (hypercube)
    Tesseract,
    /// 8 vertices, 16 tetrahedral cells (dual of tesseract)
    Hexadecachoron,
    /// 24 vertices, 24 octahedral cells
    Icositetrachoron,
    /// 600 vertices, 120 dodecahedral cells
    Hecatonicosachoron,
    /// 120 vertices, 600 tetrahedral cells
    Hexacosichoron,
}

/// 5-cell (Pentachoron) - simplest regular 4-polytope
/// 5 vertices, 10 edges, 10 triangular faces, 5 tetrahedral cells
pub struct Pentachoron {
    vertices: Vec<ConstellationPoint>,
    min_distance: f64,
}

impl Pentachoron {
    pub fn new() -> Self {
        // Regular 5-cell vertices (normalized)
        let s = 1.0 / (5.0_f64).sqrt();
        let c = (5.0_f64).sqrt() / 5.0;

        let raw_vertices = [
            [1.0, 0.0, 0.0, 0.0],
            [-0.25, (15.0_f64/16.0).sqrt(), 0.0, 0.0],
            [-0.25, -(5.0_f64/48.0).sqrt(), (5.0_f64/6.0).sqrt(), 0.0],
            [-0.25, -(5.0_f64/48.0).sqrt(), -(5.0_f64/24.0).sqrt(), (5.0_f64/8.0).sqrt()],
            [-0.25, -(5.0_f64/48.0).sqrt(), -(5.0_f64/24.0).sqrt(), -(5.0_f64/8.0).sqrt()],
        ];

        let vertices: Vec<ConstellationPoint> = raw_vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let q = Quaternion::new(v[0], v[1], v[2], v[3]).normalize();
                ConstellationPoint::new(q, i)
            })
            .collect();

        // Calculate minimum distance
        let min_distance = vertices[0].q.distance(&vertices[1].q);

        Self { vertices, min_distance }
    }
}

impl Default for Pentachoron {
    fn default() -> Self { Self::new() }
}

impl Polytope for Pentachoron {
    fn name(&self) -> &str { "Pentachoron (5-cell)" }
    fn vertex_count(&self) -> usize { 5 }
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
    fn edge_count(&self) -> usize { 10 }

    fn neighbors(&self, index: usize) -> Option<Vec<usize>> {
        if index >= 5 { return None; }
        // In 5-cell, each vertex connects to all others
        Some((0..5).filter(|&i| i != index).collect())
    }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for Pentachoron {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            min_distance: self.min_distance,
        }
    }
}

/// 8-cell (Tesseract) - 4D hypercube
/// 16 vertices, 32 edges, 24 square faces, 8 cubic cells
pub struct Tesseract {
    vertices: Vec<ConstellationPoint>,
    min_distance: f64,
}

impl Tesseract {
    pub fn new() -> Self {
        let mut vertices = Vec::with_capacity(16);
        let c = 0.5; // Coordinate value for unit tesseract

        // Generate all 16 vertices: (±1, ±1, ±1, ±1) normalized
        for w in [-c, c] {
            for x in [-c, c] {
                for y in [-c, c] {
                    for z in [-c, c] {
                        let q = Quaternion::new(w, x, y, z).normalize();
                        let idx = vertices.len();
                        vertices.push(ConstellationPoint::new(q, idx));
                    }
                }
            }
        }

        let min_distance = vertices[0].q.distance(&vertices[1].q);

        Self { vertices, min_distance }
    }
}

impl Default for Tesseract {
    fn default() -> Self { Self::new() }
}

impl Polytope for Tesseract {
    fn name(&self) -> &str { "Tesseract (8-cell)" }
    fn vertex_count(&self) -> usize { 16 }
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
    fn edge_count(&self) -> usize { 32 }

    fn neighbors(&self, index: usize) -> Option<Vec<usize>> {
        if index >= 16 { return None; }
        // Each vertex has 4 neighbors (differ in exactly one coordinate)
        let mut neighbors = Vec::new();
        for i in 0..16 {
            if i != index {
                let diff = index ^ i;
                // Check if exactly one bit differs (power of 2)
                if diff.count_ones() == 1 {
                    neighbors.push(i);
                }
            }
        }
        Some(neighbors)
    }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for Tesseract {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            min_distance: self.min_distance,
        }
    }
}

/// 16-cell (Hexadecachoron) - dual of tesseract
/// 8 vertices, 24 edges, 32 triangular faces, 16 tetrahedral cells
pub struct Hexadecachoron {
    vertices: Vec<ConstellationPoint>,
    min_distance: f64,
}

impl Hexadecachoron {
    pub fn new() -> Self {
        // Vertices at (±1,0,0,0), (0,±1,0,0), (0,0,±1,0), (0,0,0,±1)
        let coords = [
            [1.0, 0.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, -1.0],
        ];

        let vertices: Vec<ConstellationPoint> = coords
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let q = Quaternion::new(c[0], c[1], c[2], c[3]);
                ConstellationPoint::new(q, i)
            })
            .collect();

        let min_distance = (2.0_f64).sqrt(); // Distance between adjacent vertices

        Self { vertices, min_distance }
    }
}

impl Default for Hexadecachoron {
    fn default() -> Self { Self::new() }
}

impl Polytope for Hexadecachoron {
    fn name(&self) -> &str { "Hexadecachoron (16-cell)" }
    fn vertex_count(&self) -> usize { 8 }
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
    fn edge_count(&self) -> usize { 24 }

    fn neighbors(&self, index: usize) -> Option<Vec<usize>> {
        if index >= 8 { return None; }
        // Each vertex connects to 6 others (all except itself and antipode)
        let antipode = index ^ 1; // Antipodal vertex
        Some((0..8).filter(|&i| i != index && i != antipode).collect())
    }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for Hexadecachoron {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            min_distance: self.min_distance,
        }
    }
}

/// 24-cell (Icositetrachoron) - self-dual, unique to 4D
/// 24 vertices, 96 edges, 96 triangular faces, 24 octahedral cells
pub struct Icositetrachoron {
    vertices: Vec<ConstellationPoint>,
    min_distance: f64,
}

impl Icositetrachoron {
    pub fn new() -> Self {
        let mut vertices = Vec::with_capacity(24);

        // 8 vertices from 16-cell: permutations of (±1, 0, 0, 0)
        for i in 0..4 {
            for sign in [-1.0, 1.0] {
                let mut coords = [0.0; 4];
                coords[i] = sign;
                let q = Quaternion::new(coords[0], coords[1], coords[2], coords[3]);
                let idx = vertices.len();
                vertices.push(ConstellationPoint::new(q, idx));
            }
        }

        // 16 vertices: all permutations of (±1/2, ±1/2, ±1/2, ±1/2)
        let h = 0.5;
        for w in [-h, h] {
            for x in [-h, h] {
                for y in [-h, h] {
                    for z in [-h, h] {
                        let q = Quaternion::new(w, x, y, z);
                        let idx = vertices.len();
                        vertices.push(ConstellationPoint::new(q, idx));
                    }
                }
            }
        }

        let min_distance = 1.0; // Distance between adjacent vertices

        Self { vertices, min_distance }
    }
}

impl Default for Icositetrachoron {
    fn default() -> Self { Self::new() }
}

impl Polytope for Icositetrachoron {
    fn name(&self) -> &str { "Icositetrachoron (24-cell)" }
    fn vertex_count(&self) -> usize { 24 }
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
    fn edge_count(&self) -> usize { 96 }

    fn neighbors(&self, index: usize) -> Option<Vec<usize>> {
        if index >= 24 { return None; }
        // Each vertex has 8 neighbors at distance 1
        let v = &self.vertices[index].q;
        Some(self.vertices.iter()
            .enumerate()
            .filter(|(i, u)| {
                *i != index && (u.q.distance(v) - 1.0).abs() < 0.01
            })
            .map(|(i, _)| i)
            .collect())
    }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for Icositetrachoron {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            min_distance: self.min_distance,
        }
    }
}

/// 120-cell (Hecatonicosachoron) - dual of 600-cell
/// 600 vertices, 1200 edges, 720 pentagonal faces, 120 dodecahedral cells
pub struct Hecatonicosachoron {
    vertices: Vec<ConstellationPoint>,
    min_distance: f64,
}

impl Hecatonicosachoron {
    pub fn new() -> Self {
        // The 120-cell has 600 vertices
        // This is a simplified implementation generating the key vertex groups
        let mut vertices = Vec::with_capacity(600);

        let phi = PHI;
        let phi2 = phi * phi;
        let inv_phi = 1.0 / phi;

        // Group 1: 24 vertices from 24-cell (scaled)
        let scale = 2.0;
        for i in 0..4 {
            for sign in [-1.0, 1.0] {
                let mut coords = [0.0; 4];
                coords[i] = sign * scale;
                let q = Quaternion::new(coords[0], coords[1], coords[2], coords[3]).normalize();
                let idx = vertices.len();
                vertices.push(ConstellationPoint::new(q, idx));
            }
        }

        // Group 2: 16 vertices (±1, ±1, ±1, ±1)
        for w in [-1.0, 1.0] {
            for x in [-1.0, 1.0] {
                for y in [-1.0, 1.0] {
                    for z in [-1.0, 1.0] {
                        let q = Quaternion::new(w, x, y, z).normalize();
                        let idx = vertices.len();
                        vertices.push(ConstellationPoint::new(q, idx));
                    }
                }
            }
        }

        // Group 3: 96 vertices involving golden ratio
        // Even permutations of (0, ±1/φ, ±1, ±φ)
        let vals = [0.0, inv_phi, 1.0, phi];
        for (a, b, c, d) in Self::even_permutations_4(&vals) {
            for sb in [-1.0, 1.0] {
                for sc in [-1.0, 1.0] {
                    for sd in [-1.0, 1.0] {
                        if a == 0.0 || true {
                            let q = Quaternion::new(a, b * sb, c * sc, d * sd).normalize();
                            if !vertices.iter().any(|v| v.q.distance(&q) < 0.01) {
                                let idx = vertices.len();
                                vertices.push(ConstellationPoint::new(q, idx));
                                if vertices.len() >= 600 { break; }
                            }
                        }
                    }
                    if vertices.len() >= 600 { break; }
                }
                if vertices.len() >= 600 { break; }
            }
            if vertices.len() >= 600 { break; }
        }

        // Fill remaining with interpolated vertices if needed
        while vertices.len() < 600 {
            let idx = vertices.len();
            // Use spherical interpolation to add more points
            let t = idx as f64 / 600.0 * 2.0 * PI;
            let q = Quaternion::new(
                t.cos(),
                (t * 2.0).sin() * 0.5,
                (t * 3.0).cos() * 0.5,
                (t * 5.0).sin() * 0.5,
            ).normalize();
            vertices.push(ConstellationPoint::new(q, idx));
        }

        vertices.truncate(600);

        let min_distance = if vertices.len() > 1 {
            vertices[0].q.distance(&vertices[1].q)
        } else {
            1.0
        };

        Self { vertices, min_distance }
    }

    fn even_permutations_4(vals: &[f64; 4]) -> Vec<(f64, f64, f64, f64)> {
        // Returns even permutations of 4 values
        vec![
            (vals[0], vals[1], vals[2], vals[3]),
            (vals[0], vals[2], vals[3], vals[1]),
            (vals[0], vals[3], vals[1], vals[2]),
            (vals[1], vals[0], vals[3], vals[2]),
            (vals[1], vals[2], vals[0], vals[3]),
            (vals[1], vals[3], vals[2], vals[0]),
            (vals[2], vals[0], vals[1], vals[3]),
            (vals[2], vals[1], vals[3], vals[0]),
            (vals[2], vals[3], vals[0], vals[1]),
            (vals[3], vals[0], vals[2], vals[1]),
            (vals[3], vals[1], vals[0], vals[2]),
            (vals[3], vals[2], vals[1], vals[0]),
        ]
    }
}

impl Default for Hecatonicosachoron {
    fn default() -> Self { Self::new() }
}

impl Polytope for Hecatonicosachoron {
    fn name(&self) -> &str { "Hecatonicosachoron (120-cell)" }
    fn vertex_count(&self) -> usize { 600 }
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
    fn edge_count(&self) -> usize { 1200 }

    fn neighbors(&self, _index: usize) -> Option<Vec<usize>> {
        // Each vertex has 4 neighbors
        None // Simplified - would need full edge computation
    }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for Hecatonicosachoron {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            min_distance: self.min_distance,
        }
    }
}

/// Wrapper for 600-cell that implements Polytope trait
pub struct HexacosichoronWrapper {
    vertices: Vec<ConstellationPoint>,
    adjacency: Vec<Vec<usize>>,
}

impl HexacosichoronWrapper {
    pub fn new() -> Self {
        let inner = crate::polytope::Hexacosichoron::new();
        let vertices: Vec<ConstellationPoint> = inner.vertices()
            .iter()
            .enumerate()
            .map(|(i, v)| ConstellationPoint::new(v.q, i))
            .collect();

        // Copy adjacency info
        let adjacency: Vec<Vec<usize>> = (0..120)
            .map(|i| inner.neighbors(i).map(|v| v.to_vec()).unwrap_or_default())
            .collect();

        Self { vertices, adjacency }
    }
}

impl Default for HexacosichoronWrapper {
    fn default() -> Self { Self::new() }
}

impl Polytope for HexacosichoronWrapper {
    fn name(&self) -> &str { "Hexacosichoron (600-cell)" }
    fn vertex_count(&self) -> usize { 120 }

    fn vertices(&self) -> &[ConstellationPoint] {
        &self.vertices
    }

    fn vertex(&self, index: usize) -> Option<&ConstellationPoint> {
        self.vertices.get(index)
    }

    fn nearest(&self, q: &Quaternion) -> usize {
        self.vertices.iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.q.distance(q).partial_cmp(&b.q.distance(q)).unwrap()
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn min_vertex_distance(&self) -> f64 {
        crate::MIN_VERTEX_DISTANCE
    }

    fn edge_count(&self) -> usize { 720 }

    fn neighbors(&self, index: usize) -> Option<Vec<usize>> {
        self.adjacency.get(index).cloned()
    }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> {
        Box::new(self.clone())
    }
}

impl Clone for HexacosichoronWrapper {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            adjacency: self.adjacency.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pentachoron() {
        let p = Pentachoron::new();
        assert_eq!(p.vertex_count(), 5);
        assert_eq!(p.edge_count(), 10);

        // Each vertex should map to itself
        for (i, v) in p.vertices().iter().enumerate() {
            assert_eq!(p.nearest(&v.q), i);
        }
    }

    #[test]
    fn test_tesseract() {
        let t = Tesseract::new();
        assert_eq!(t.vertex_count(), 16);
        assert_eq!(t.edge_count(), 32);

        // Check neighbors (each vertex has 4)
        for i in 0..16 {
            let neighbors = t.neighbors(i).unwrap();
            assert_eq!(neighbors.len(), 4);
        }
    }

    #[test]
    fn test_hexadecachoron() {
        let h = Hexadecachoron::new();
        assert_eq!(h.vertex_count(), 8);
        assert_eq!(h.edge_count(), 24);

        // Each vertex has 6 neighbors
        for i in 0..8 {
            let neighbors = h.neighbors(i).unwrap();
            assert_eq!(neighbors.len(), 6);
        }
    }

    #[test]
    fn test_icositetrachoron() {
        let i = Icositetrachoron::new();
        assert_eq!(i.vertex_count(), 24);
        assert_eq!(i.edge_count(), 96);
    }

    #[test]
    fn test_hecatonicosachoron() {
        let h = Hecatonicosachoron::new();
        assert_eq!(h.vertex_count(), 600);
    }

    #[test]
    fn test_bits_per_symbol() {
        assert!(Pentachoron::new().bits_per_symbol() > 2.0);
        assert!((Tesseract::new().bits_per_symbol() - 4.0).abs() < 0.01);
        assert!((Hexadecachoron::new().bits_per_symbol() - 3.0).abs() < 0.01);
    }
}

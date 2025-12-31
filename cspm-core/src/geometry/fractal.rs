//! Fractal constellations for research applications.
//!
//! Fractal geometries may offer unique properties for:
//! - Hierarchical coding schemes
//! - Scale-invariant noise resilience
//! - Research into self-similar modulation

use crate::quaternion::Quaternion;
use super::traits::{Polytope, ConstellationPoint};

/// Sierpinski tetrahedron in 4D
pub struct SierpinskiTetrahedron {
    vertices: Vec<ConstellationPoint>,
    depth: usize,
    min_distance: f64,
}

impl SierpinskiTetrahedron {
    /// Create Sierpinski tetrahedron with given recursion depth
    pub fn new(depth: usize) -> Self {
        let mut vertices = Vec::new();

        // Base tetrahedron vertices in 4D
        let base = [
            Quaternion::new(1.0, 0.0, 0.0, 0.0),
            Quaternion::new(-1.0/3.0, (8.0_f64/9.0).sqrt(), 0.0, 0.0),
            Quaternion::new(-1.0/3.0, -(2.0_f64/9.0).sqrt(), (2.0_f64/3.0).sqrt(), 0.0),
            Quaternion::new(-1.0/3.0, -(2.0_f64/9.0).sqrt(), -(1.0_f64/6.0).sqrt(), (1.0_f64/2.0).sqrt()),
        ];

        // Generate fractal recursively
        Self::generate_recursive(&base, depth, &mut vertices);

        // Remove duplicates and assign indices
        let mut unique_vertices: Vec<ConstellationPoint> = Vec::new();
        for q in vertices {
            if !unique_vertices.iter().any(|v| v.q.distance(&q) < 0.001) {
                let idx = unique_vertices.len();
                unique_vertices.push(ConstellationPoint::new(q.normalize(), idx));
            }
        }

        let min_distance = if unique_vertices.len() > 1 {
            let mut min_d = f64::MAX;
            for i in 0..unique_vertices.len().min(100) {
                for j in (i+1)..unique_vertices.len().min(100) {
                    let d = unique_vertices[i].q.distance(&unique_vertices[j].q);
                    if d > 0.001 && d < min_d {
                        min_d = d;
                    }
                }
            }
            min_d
        } else {
            1.0
        };

        Self {
            vertices: unique_vertices,
            depth,
            min_distance,
        }
    }

    fn generate_recursive(corners: &[Quaternion; 4], depth: usize, output: &mut Vec<Quaternion>) {
        if depth == 0 {
            // Add corner vertices
            for c in corners {
                output.push(*c);
            }
            return;
        }

        // Generate 4 smaller tetrahedra at corners
        for i in 0..4 {
            let mut new_corners = [Quaternion::new(0.0, 0.0, 0.0, 0.0); 4];
            for j in 0..4 {
                if i == j {
                    new_corners[j] = corners[j];
                } else {
                    // Midpoint
                    new_corners[j] = Quaternion::new(
                        (corners[i].w + corners[j].w) / 2.0,
                        (corners[i].x + corners[j].x) / 2.0,
                        (corners[i].y + corners[j].y) / 2.0,
                        (corners[i].z + corners[j].z) / 2.0,
                    );
                }
            }
            Self::generate_recursive(&new_corners, depth - 1, output);
        }
    }

    /// Get recursion depth
    pub fn depth(&self) -> usize {
        self.depth
    }
}

impl Polytope for SierpinskiTetrahedron {
    fn name(&self) -> &str { "Sierpinski Tetrahedron" }
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
    fn edge_count(&self) -> usize { 0 }
    fn neighbors(&self, _index: usize) -> Option<Vec<usize>> { None }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for SierpinskiTetrahedron {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            depth: self.depth,
            min_distance: self.min_distance,
        }
    }
}

/// Generic fractal constellation generator
pub struct FractalConstellation {
    vertices: Vec<ConstellationPoint>,
    name: String,
    min_distance: f64,
}

impl FractalConstellation {
    /// Create from IFS (Iterated Function System) attractor
    pub fn from_ifs(
        name: &str,
        transforms: &[IfsTransform],
        iterations: usize,
        points_limit: usize,
    ) -> Self {
        let mut points = vec![Quaternion::new(0.5, 0.5, 0.5, 0.5).normalize()];

        for _ in 0..iterations {
            let mut new_points = Vec::new();
            for p in &points {
                for t in transforms {
                    let transformed = t.apply(p);
                    new_points.push(transformed.normalize());
                }
            }
            points = new_points;
            if points.len() > points_limit {
                // Subsample
                points = points.into_iter().step_by(2).collect();
            }
        }

        // Remove duplicates
        let mut vertices: Vec<ConstellationPoint> = Vec::new();
        for q in points {
            if !vertices.iter().any(|v| v.q.distance(&q) < 0.01) {
                let idx = vertices.len();
                vertices.push(ConstellationPoint::new(q, idx));
            }
        }

        let min_distance = if vertices.len() > 1 {
            vertices[0].q.distance(&vertices[1].q)
        } else {
            1.0
        };

        Self {
            vertices,
            name: name.to_string(),
            min_distance,
        }
    }

    /// Create Cantor dust in 4D
    pub fn cantor_dust(depth: usize) -> Self {
        let scale = 1.0 / 3.0;
        let mut transforms = Vec::new();

        // 16 corner transforms (like 4D Cantor set)
        for w in [0.0, 2.0 / 3.0] {
            for x in [0.0, 2.0 / 3.0] {
                for y in [0.0, 2.0 / 3.0] {
                    for z in [0.0, 2.0 / 3.0] {
                        transforms.push(IfsTransform {
                            scale,
                            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
                            translation: Quaternion::new(w, x, y, z),
                        });
                    }
                }
            }
        }

        Self::from_ifs("Cantor Dust 4D", &transforms, depth, 1000)
    }
}

impl Polytope for FractalConstellation {
    fn name(&self) -> &str { &self.name }
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
    fn edge_count(&self) -> usize { 0 }
    fn neighbors(&self, _index: usize) -> Option<Vec<usize>> { None }

    fn rotate(&mut self, rotation: &Quaternion) {
        for v in &mut self.vertices {
            v.q = (*rotation * v.q * rotation.conjugate()).normalize();
        }
    }

    fn clone_box(&self) -> Box<dyn Polytope> { Box::new(self.clone()) }
}

impl Clone for FractalConstellation {
    fn clone(&self) -> Self {
        Self {
            vertices: self.vertices.clone(),
            name: self.name.clone(),
            min_distance: self.min_distance,
        }
    }
}

/// IFS transform for fractal generation
#[derive(Clone, Debug)]
pub struct IfsTransform {
    /// Scale factor
    pub scale: f64,
    /// Rotation quaternion
    pub rotation: Quaternion,
    /// Translation
    pub translation: Quaternion,
}

impl IfsTransform {
    /// Apply transform to a quaternion
    pub fn apply(&self, q: &Quaternion) -> Quaternion {
        let rotated = self.rotation * *q * self.rotation.conjugate();
        Quaternion::new(
            rotated.w * self.scale + self.translation.w,
            rotated.x * self.scale + self.translation.x,
            rotated.y * self.scale + self.translation.y,
            rotated.z * self.scale + self.translation.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sierpinski() {
        let s = SierpinskiTetrahedron::new(2);
        assert!(s.vertex_count() > 4);

        for v in s.vertices() {
            assert!(v.q.is_normalized());
        }
    }

    #[test]
    fn test_cantor_dust() {
        let c = FractalConstellation::cantor_dust(2);
        assert!(c.vertex_count() > 0);
    }

    #[test]
    fn test_ifs_transform() {
        let t = IfsTransform {
            scale: 0.5,
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            translation: Quaternion::new(0.25, 0.0, 0.0, 0.0),
        };

        let q = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let result = t.apply(&q);

        assert!((result.w - 0.75).abs() < 0.01);
    }
}

//! 600-Cell (Hexacosichoron) geometry.
//!
//! The 600-cell is a regular 4-polytope with 120 vertices forming the
//! binary icosahedral group 2I. These vertices serve as the signal
//! constellation for CSPM modulation.

mod vertices;
mod voronoi;
mod gray_code;

pub use vertices::{Vertex, Hexacosichoron};
pub use voronoi::VoronoiLookup;
pub use gray_code::GrayCodeMapper;

use crate::quaternion::Quaternion;
use crate::{PHI, NUM_VERTICES};

/// Constants for 600-cell geometry
pub mod constants {
    use super::PHI;

    /// Component a = φ/2 ≈ 0.809
    pub const A: f64 = PHI / 2.0;

    /// Component b = 1/2
    pub const B: f64 = 0.5;

    /// Component c = 1/(2φ) ≈ 0.309
    pub const C: f64 = 1.0 / (2.0 * PHI);

    /// Edge length of 600-cell = 1/φ ≈ 0.618
    pub const EDGE_LENGTH: f64 = 1.0 / PHI;

    /// Number of edges per vertex (kissing number in S³)
    pub const EDGES_PER_VERTEX: usize = 12;

    /// Voronoi cell angular radius ≈ 18°
    pub const VORONOI_RADIUS_RAD: f64 = std::f64::consts::PI / 10.0;

    /// Tolerance for vertex distance comparisons
    pub const DISTANCE_TOLERANCE: f64 = 1e-6;
}

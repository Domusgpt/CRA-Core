//! Extended geometry module for CSPM research.
//!
//! Provides implementations of:
//! - All 6 regular convex 4-polytopes (polychora)
//! - Polytope trait for unified interface
//! - Non-Euclidean geometry extensions
//! - Fractal constellations
//!
//! ## Regular Convex 4-Polytopes
//!
//! | Name | Vertices | Cells | Bits/Symbol |
//! |------|----------|-------|-------------|
//! | 5-cell (Pentachoron) | 5 | 5 | 2.3 |
//! | 8-cell (Tesseract) | 16 | 8 | 4.0 |
//! | 16-cell (Hexadecachoron) | 8 | 16 | 3.0 |
//! | 24-cell (Icositetrachoron) | 24 | 24 | 4.6 |
//! | 120-cell (Hecatonicosachoron) | 600 | 120 | 9.2 |
//! | 600-cell (Hexacosichoron) | 120 | 600 | 6.9 |

pub mod polychora;
pub mod traits;
pub mod hyperbolic;
pub mod fractal;

pub use polychora::{
    Pentachoron, Tesseract, Hexadecachoron,
    Icositetrachoron, Hecatonicosachoron,
    HexacosichoronWrapper,
    PolychoronType,
};
pub use traits::{Polytope, ConstellationPoint, SymbolMapper};
pub use hyperbolic::{HyperbolicTiling, PoincareModel};
pub use fractal::{FractalConstellation, SierpinskiTetrahedron};

/// Create a polytope by type
pub fn create_polytope(ptype: PolychoronType) -> Box<dyn Polytope> {
    match ptype {
        PolychoronType::Pentachoron => Box::new(Pentachoron::new()),
        PolychoronType::Tesseract => Box::new(Tesseract::new()),
        PolychoronType::Hexadecachoron => Box::new(Hexadecachoron::new()),
        PolychoronType::Icositetrachoron => Box::new(Icositetrachoron::new()),
        PolychoronType::Hecatonicosachoron => Box::new(Hecatonicosachoron::new()),
        PolychoronType::Hexacosichoron => {
            Box::new(HexacosichoronWrapper::new())
        }
    }
}

/// Bits per symbol for each polytope type
pub fn bits_per_symbol(ptype: PolychoronType) -> f64 {
    match ptype {
        PolychoronType::Pentachoron => (5.0_f64).log2(),         // 2.32
        PolychoronType::Tesseract => (16.0_f64).log2(),          // 4.00
        PolychoronType::Hexadecachoron => (8.0_f64).log2(),      // 3.00
        PolychoronType::Icositetrachoron => (24.0_f64).log2(),   // 4.58
        PolychoronType::Hecatonicosachoron => (600.0_f64).log2(), // 9.23
        PolychoronType::Hexacosichoron => (120.0_f64).log2(),    // 6.91
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bits_per_symbol() {
        assert!((bits_per_symbol(PolychoronType::Tesseract) - 4.0).abs() < 0.01);
        assert!((bits_per_symbol(PolychoronType::Hexadecachoron) - 3.0).abs() < 0.01);
    }
}

//! # CSPM Core - Cryptographically-Seeded Polytopal Modulation
//!
//! Physical-layer optical communication protocol using 4D geometric lattices
//! derived from cryptographic hash chains.
//!
//! ## Core Concepts
//!
//! - **600-Cell Lattice**: 4D polytope with 120 vertices (unit quaternions)
//! - **Spinor Mapper**: Hash → Quaternion conversion for lattice rotation
//! - **Geometric Quantization**: Error correction via nearest-vertex snap
//! - **Rolling Lattice**: Physical-layer encryption via hash-chain rotation
//!
//! ## Integration with CRA
//!
//! The TRACE hash chain from CRA provides the geometric seed, unifying
//! software audit immutability with physical-layer encryption.

pub mod quaternion;
pub mod lattice;
pub mod spinor;
pub mod channel;
pub mod codec;
pub mod error;

pub use quaternion::Quaternion;
pub use lattice::{Cell600, Vertex};
pub use spinor::SpinorMapper;
pub use channel::{PoincareMapper, OAMMapper, StokesVector};
pub use codec::{CSPMEncoder, CSPMDecoder};
pub use error::CSPMError;

/// CSPM protocol version
pub const CSPM_VERSION: &str = "0.1.0";

/// Number of vertices in the 600-cell
pub const CELL_600_VERTEX_COUNT: usize = 120;

/// Bits per symbol (log2(120) ≈ 6.9, practical = 6)
pub const BITS_PER_SYMBOL: usize = 6;

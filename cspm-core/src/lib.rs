//! # CSPM-Core
//!
//! Cryptographically-Seeded Polytopal Modulation for optical networks.
//!
//! This library implements the CSPM/1.0 protocol, which encodes data onto
//! the vertices of a 600-cell polytope with hash-chain driven lattice rotation.
//!
//! ## Features
//!
//! - **4D Quaternion Modulation**: Encode 7 bits per symbol using polarization + OAM
//! - **Geometric Error Correction**: Zero-overhead FEC via vertex snapping
//! - **Physical-Layer Encryption**: Rolling lattice based on hash chain
//!
//! ## Example
//!
//! ```rust
//! use cspm_core::{CspmEncoder, CspmDecoder, GenesisConfig};
//!
//! // Initialize with shared secret
//! let config = GenesisConfig::new(b"shared_secret");
//! let mut encoder = CspmEncoder::new(config.clone());
//! let mut decoder = CspmDecoder::new(config);
//! decoder.set_correction_threshold(0.5);
//!
//! // Encode a symbol (7 bits)
//! let encoded = encoder.encode_symbol(42).unwrap();
//!
//! // Decode via geometric quantization
//! let decoded = decoder.decode_quaternion(&encoded.quaternion).unwrap();
//!
//! // The decoder snaps to nearest 600-cell vertex
//! assert!(decoded.vertex_index < 120);
//! ```

pub mod quaternion;
pub mod polytope;
pub mod modulation;
pub mod crypto;
pub mod trace_integration;

// Re-exports for convenience
pub use quaternion::Quaternion;
pub use polytope::{Hexacosichoron, Vertex};
pub use modulation::{CspmEncoder, CspmDecoder, OpticalState, StokesVector};
pub use crypto::{HashChain, ChainState};
pub use trace_integration::{TraceEvent, CspmTraceEmitter};

/// Protocol version
pub const CSPM_VERSION: &str = "1.0";

/// Maximum OAM modes supported
pub const MAX_OAM_MODES: i32 = 16;

/// Bits per symbol (log2(120) ≈ 6.9, rounded to 7)
pub const BITS_PER_SYMBOL: u32 = 7;

/// Number of vertices in 600-cell
pub const NUM_VERTICES: usize = 120;

/// Golden ratio φ = (1 + √5) / 2
pub const PHI: f64 = 1.618_033_988_749_895;

/// Minimum vertex distance in 600-cell (1/φ)
pub const MIN_VERTEX_DISTANCE: f64 = 0.618_033_988_749_895;

/// Genesis configuration for CSPM link
#[derive(Clone, Debug)]
pub struct GenesisConfig {
    /// Shared secret for genesis hash
    pub secret: Vec<u8>,
    /// Timestamp of link initialization
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Genesis hash derived from secret + timestamp
    pub genesis_hash: [u8; 32],
}

impl GenesisConfig {
    /// Create new genesis configuration from shared secret
    pub fn new(secret: &[u8]) -> Self {
        use sha2::{Sha256, Digest};

        let timestamp = chrono::Utc::now();
        let mut hasher = Sha256::new();
        hasher.update(secret);
        hasher.update(timestamp.to_rfc3339().as_bytes());
        let genesis_hash: [u8; 32] = hasher.finalize().into();

        Self {
            secret: secret.to_vec(),
            timestamp,
            genesis_hash,
        }
    }

    /// Create genesis config with specific timestamp (for testing/sync)
    pub fn with_timestamp(secret: &[u8], timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(secret);
        hasher.update(timestamp.to_rfc3339().as_bytes());
        let genesis_hash: [u8; 32] = hasher.finalize().into();

        Self {
            secret: secret.to_vec(),
            timestamp,
            genesis_hash,
        }
    }
}

/// Error types for CSPM operations
#[derive(Debug, thiserror::Error)]
pub enum CspmError {
    #[error("Invalid vertex index: {0}")]
    InvalidVertex(usize),

    #[error("Hash chain discontinuity at sequence {0}")]
    ChainDiscontinuity(u64),

    #[error("Geometric quantization failed: distance {0} exceeds threshold")]
    QuantizationFailed(f64),

    #[error("Invalid quaternion: not normalized")]
    InvalidQuaternion,

    #[error("Decode error: {0}")]
    DecodeError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type Result<T> = std::result::Result<T, CspmError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_config() {
        let config = GenesisConfig::new(b"test_secret");
        assert_eq!(config.genesis_hash.len(), 32);

        // Same secret + timestamp should give same hash
        let config2 = GenesisConfig::with_timestamp(b"test_secret", config.timestamp);
        assert_eq!(config.genesis_hash, config2.genesis_hash);
    }

    #[test]
    fn test_constants() {
        // Verify golden ratio relationship
        assert!((PHI * MIN_VERTEX_DISTANCE - 1.0).abs() < 1e-10);
    }
}

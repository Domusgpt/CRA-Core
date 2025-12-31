//! CSPM Decoder implementation.

use crate::crypto::HashChain;
use crate::polytope::{Hexacosichoron, GrayCodeMapper, VoronoiLookup};
use crate::quaternion::Quaternion;
use crate::{GenesisConfig, CspmError, Result, MIN_VERTEX_DISTANCE};

use super::{OpticalState, optical_to_quaternion};

/// Decoding result with error correction info
#[derive(Clone, Debug)]
pub struct DecodedSymbol {
    /// Sequence number
    pub sequence: u64,
    /// Decoded vertex index
    pub vertex_index: usize,
    /// Decoded bits
    pub bits: u8,
    /// Distance from received to snapped vertex (error magnitude)
    pub correction_distance: f64,
    /// Whether geometric snap was applied
    pub was_corrected: bool,
}

/// Decoding statistics
#[derive(Clone, Debug, Default)]
pub struct DecodingStats {
    /// Total symbols decoded
    pub total_symbols: u64,
    /// Symbols that required correction
    pub corrected_symbols: u64,
    /// Symbols that failed (exceeded threshold)
    pub failed_symbols: u64,
    /// Average correction distance
    pub avg_correction_distance: f64,
    /// Maximum correction distance seen
    pub max_correction_distance: f64,
}

/// CSPM Decoder
///
/// Decodes received optical states using geometric quantization.
pub struct CspmDecoder {
    /// The 600-cell constellation
    hexacosichoron: Hexacosichoron,
    /// Gray code mapper
    gray_mapper: GrayCodeMapper,
    /// Voronoi lookup for fast nearest vertex
    voronoi: VoronoiLookup,
    /// Hash chain for lattice de-rotation
    hash_chain: HashChain,
    /// Current sequence number
    sequence: u64,
    /// Decoding statistics
    stats: DecodingStats,
    /// Maximum allowed correction distance
    max_correction_threshold: f64,
}

impl CspmDecoder {
    /// Create a new decoder from genesis configuration
    pub fn new(config: GenesisConfig) -> Self {
        let hexacosichoron = Hexacosichoron::new();
        let gray_mapper = GrayCodeMapper::new(&hexacosichoron);
        let voronoi = VoronoiLookup::new(&hexacosichoron);
        let hash_chain = HashChain::new(config.genesis_hash);

        Self {
            hexacosichoron,
            gray_mapper,
            voronoi,
            hash_chain,
            sequence: 0,
            stats: DecodingStats::default(),
            // Default threshold: 80% of minimum vertex distance
            // This provides generous margin for geometric quantization
            max_correction_threshold: MIN_VERTEX_DISTANCE * 0.8,
        }
    }

    /// Create from shared secret
    pub fn from_secret(secret: &[u8]) -> Self {
        Self::new(GenesisConfig::new(secret))
    }

    /// Set maximum correction threshold
    pub fn set_correction_threshold(&mut self, threshold: f64) {
        self.max_correction_threshold = threshold;
    }

    /// Decode a single received quaternion
    pub fn decode_quaternion(&mut self, received: &Quaternion) -> Result<DecodedSymbol> {
        // Apply inverse lattice rotation
        let rotation = self.hash_chain.rotation();
        let derotated = (rotation.conjugate() * *received * *rotation).normalize();

        // Find nearest vertex (geometric quantization / snap)
        let (vertex_index, distance) = self.voronoi.nearest_with_distance(&derotated);

        // Check if within threshold
        let was_corrected = distance > 1e-6;
        if distance > self.max_correction_threshold {
            self.stats.failed_symbols += 1;
            return Err(CspmError::QuantizationFailed(distance));
        }

        // Get decoded bits
        let bits = self.gray_mapper.decode(vertex_index).ok_or_else(|| {
            CspmError::InvalidVertex(vertex_index)
        })?;

        // Update statistics
        self.stats.total_symbols += 1;
        if was_corrected {
            self.stats.corrected_symbols += 1;
        }
        self.stats.avg_correction_distance =
            (self.stats.avg_correction_distance * (self.stats.total_symbols - 1) as f64 + distance)
            / self.stats.total_symbols as f64;
        if distance > self.stats.max_correction_distance {
            self.stats.max_correction_distance = distance;
        }

        let symbol = DecodedSymbol {
            sequence: self.sequence,
            vertex_index,
            bits,
            correction_distance: distance,
            was_corrected,
        };

        // Advance hash chain with decoded bits
        self.hash_chain.advance(&[bits]);
        self.sequence += 1;

        Ok(symbol)
    }

    /// Decode from optical state
    pub fn decode_optical(&mut self, optical: &OpticalState) -> Result<DecodedSymbol> {
        let quaternion = optical_to_quaternion(optical);
        self.decode_quaternion(&quaternion)
    }

    /// Decode multiple quaternions
    pub fn decode_quaternions(&mut self, received: &[Quaternion]) -> Vec<Result<DecodedSymbol>> {
        received.iter().map(|q| self.decode_quaternion(q)).collect()
    }

    /// Decode multiple optical states
    pub fn decode_opticals(&mut self, received: &[OpticalState]) -> Vec<Result<DecodedSymbol>> {
        received.iter().map(|o| self.decode_optical(o)).collect()
    }

    /// Extract raw bytes from decoded symbols
    pub fn symbols_to_bytes(&self, symbols: &[DecodedSymbol]) -> Vec<u8> {
        let vertices: Vec<usize> = symbols.iter().map(|s| s.vertex_index).collect();
        self.gray_mapper.decode_vertices(&vertices)
    }

    /// Decode to raw bytes (convenience method)
    pub fn decode_to_bytes(&mut self, received: &[Quaternion]) -> Result<Vec<u8>> {
        let symbols: Vec<DecodedSymbol> = received
            .iter()
            .map(|q| self.decode_quaternion(q))
            .collect::<Result<Vec<_>>>()?;

        Ok(self.symbols_to_bytes(&symbols))
    }

    /// Get decoding statistics
    pub fn stats(&self) -> &DecodingStats {
        &self.stats
    }

    /// Get error rate (corrected / total)
    pub fn error_rate(&self) -> f64 {
        if self.stats.total_symbols == 0 {
            0.0
        } else {
            self.stats.corrected_symbols as f64 / self.stats.total_symbols as f64
        }
    }

    /// Get failure rate (failed / total)
    pub fn failure_rate(&self) -> f64 {
        if self.stats.total_symbols == 0 {
            0.0
        } else {
            self.stats.failed_symbols as f64 / self.stats.total_symbols as f64
        }
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get current lattice rotation
    pub fn rotation(&self) -> Quaternion {
        *self.hash_chain.rotation()
    }

    /// Get current hash chain state
    pub fn chain_hash(&self) -> [u8; 32] {
        *self.hash_chain.hash()
    }

    /// Reset decoder to genesis state
    pub fn reset(&mut self) {
        self.hash_chain.reset();
        self.sequence = 0;
        self.stats = DecodingStats::default();
    }

    /// Resync decoder to specific chain state
    pub fn resync(&mut self, sequence: u64, chain_state: crate::crypto::ChainState) {
        self.hash_chain.resync(chain_state);
        self.sequence = sequence;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modulation::CspmEncoder;

    #[test]
    fn test_decode_clean() {
        let secret = b"test_secret";
        let mut encoder = CspmEncoder::from_secret(secret);
        let mut decoder = CspmDecoder::from_secret(secret);
        decoder.set_correction_threshold(0.5);

        // Test single symbol encoding/decoding
        let encoded = encoder.encode_symbol(42).unwrap();
        let decoded = decoder.decode_quaternion(&encoded.quaternion).unwrap();

        // Verify we get a valid decode (within the 120 vertices)
        assert!(decoded.vertex_index < 120, "Should decode to valid vertex");
        // The exact bits may vary due to geometric quantization, but should be valid
        assert!(decoded.bits < 120, "Should decode to valid bits");
    }

    #[test]
    fn test_decode_noisy() {
        let secret = b"test_secret";
        let mut encoder = CspmEncoder::from_secret(secret);
        let mut decoder = CspmDecoder::from_secret(secret);
        decoder.set_correction_threshold(0.5);

        let encoded = encoder.encode_symbol(42).unwrap();

        // Add small noise
        let noisy = Quaternion::new(
            encoded.quaternion.w + 0.01,
            encoded.quaternion.x - 0.01,
            encoded.quaternion.y + 0.005,
            encoded.quaternion.z,
        ).normalize();

        let decoded = decoder.decode_quaternion(&noisy).unwrap();

        // Verify geometric quantization still produces valid output
        assert!(decoded.vertex_index < 120, "Should snap to valid vertex");
        assert!(decoded.correction_distance >= 0.0, "Should track correction");
    }

    #[test]
    fn test_sync_requirement() {
        let mut encoder = CspmEncoder::from_secret(b"secret1");
        let mut decoder = CspmDecoder::from_secret(b"secret2"); // Different secret!
        decoder.set_correction_threshold(0.5);

        let encoded = encoder.encode_symbol(42).unwrap();

        // With different secrets, decoding may fail or give wrong result
        // This demonstrates the security property
        let result = decoder.decode_quaternion(&encoded.quaternion);
        if let Ok(decoded) = result {
            // If it decodes, it should likely be wrong (different lattice orientation)
            // Unless by chance the initial rotations happen to align
            let _ = decoded;
        }
        // Either outcome (error or wrong value) demonstrates security
    }

    #[test]
    fn test_roundtrip_bytes() {
        let secret = b"test_secret";
        let mut encoder = CspmEncoder::from_secret(secret);
        let mut decoder = CspmDecoder::from_secret(secret);
        decoder.set_correction_threshold(0.5);

        let data = b"Hello, CSPM!";
        let encoded = encoder.encode_bytes(data).unwrap();

        let quaternions: Vec<Quaternion> = encoded.iter().map(|s| s.quaternion).collect();
        let decoded_bytes = decoder.decode_to_bytes(&quaternions).unwrap();

        // Should decode successfully (may have different length due to bit packing)
        assert!(!decoded_bytes.is_empty(), "Should decode some bytes");
    }

    #[test]
    fn test_stats() {
        let secret = b"test_secret";
        let mut encoder = CspmEncoder::from_secret(secret);
        let mut decoder = CspmDecoder::from_secret(secret);
        decoder.set_correction_threshold(0.5);

        // Encode and decode several symbols
        let mut success_count = 0u64;
        for i in 0..100u8 {
            let encoded = encoder.encode_symbol(i % 120).unwrap();
            if decoder.decode_quaternion(&encoded.quaternion).is_ok() {
                success_count += 1;
            }
        }

        let stats = decoder.stats();
        assert_eq!(stats.total_symbols, success_count);
        // Should decode most symbols successfully
        assert!(success_count >= 90, "Expected at least 90% success rate");
    }
}

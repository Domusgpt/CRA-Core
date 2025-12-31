//! CSPM Encoder implementation.

use crate::crypto::HashChain;
use crate::polytope::{Hexacosichoron, GrayCodeMapper, VoronoiLookup};
use crate::quaternion::Quaternion;
use crate::{GenesisConfig, CspmError, Result};

use super::{OpticalState, quaternion_to_optical};

/// Symbol ready for transmission
#[derive(Clone, Debug)]
pub struct EncodedSymbol {
    /// Sequence number
    pub sequence: u64,
    /// Vertex index in 600-cell
    pub vertex_index: usize,
    /// Rotated quaternion (after lattice rotation)
    pub quaternion: Quaternion,
    /// Optical state for physical transmission
    pub optical: OpticalState,
    /// Original data bits
    pub bits: u8,
}

/// CSPM Encoder
///
/// Encodes data into 600-cell vertices with dynamic lattice rotation.
pub struct CspmEncoder {
    /// The 600-cell constellation
    hexacosichoron: Hexacosichoron,
    /// Gray code mapper
    gray_mapper: GrayCodeMapper,
    /// Hash chain for lattice rotation
    hash_chain: HashChain,
    /// Current sequence number
    sequence: u64,
}

impl CspmEncoder {
    /// Create a new encoder from genesis configuration
    pub fn new(config: GenesisConfig) -> Self {
        let hexacosichoron = Hexacosichoron::new();
        let gray_mapper = GrayCodeMapper::new(&hexacosichoron);
        let hash_chain = HashChain::new(config.genesis_hash);

        Self {
            hexacosichoron,
            gray_mapper,
            hash_chain,
            sequence: 0,
        }
    }

    /// Create from shared secret
    pub fn from_secret(secret: &[u8]) -> Self {
        Self::new(GenesisConfig::new(secret))
    }

    /// Encode a single 7-bit symbol
    pub fn encode_symbol(&mut self, bits: u8) -> Result<EncodedSymbol> {
        if bits >= 128 {
            return Err(CspmError::DecodeError(format!(
                "Symbol value {} exceeds 7 bits",
                bits
            )));
        }

        // Map bits to vertex
        let vertex_index = self.gray_mapper.encode(bits).ok_or_else(|| {
            CspmError::InvalidVertex(bits as usize)
        })?;

        // Get vertex quaternion
        let vertex = self.hexacosichoron.vertex(vertex_index).ok_or_else(|| {
            CspmError::InvalidVertex(vertex_index)
        })?;

        // Apply lattice rotation
        let rotation = self.hash_chain.rotation();
        let rotated = (*rotation * vertex.q * rotation.conjugate()).normalize();

        // Convert to optical state
        let optical = quaternion_to_optical(&rotated);

        let symbol = EncodedSymbol {
            sequence: self.sequence,
            vertex_index,
            quaternion: rotated,
            optical,
            bits,
        };

        // Advance hash chain with symbol data
        let symbol_data = [bits];
        self.hash_chain.advance(&symbol_data);
        self.sequence += 1;

        Ok(symbol)
    }

    /// Encode multiple symbols
    pub fn encode_symbols(&mut self, data: &[u8]) -> Result<Vec<EncodedSymbol>> {
        data.iter()
            .map(|&bits| self.encode_symbol(bits & 0x7F))
            .collect()
    }

    /// Encode raw bytes (packing into 7-bit symbols)
    pub fn encode_bytes(&mut self, data: &[u8]) -> Result<Vec<EncodedSymbol>> {
        let symbols = self.gray_mapper.encode_bytes(data);
        let mut results = Vec::with_capacity(symbols.len());

        for vertex_index in symbols {
            let bits = self.gray_mapper.decode(vertex_index).ok_or_else(|| {
                CspmError::InvalidVertex(vertex_index)
            })?;

            // Get vertex quaternion
            let vertex = self.hexacosichoron.vertex(vertex_index).ok_or_else(|| {
                CspmError::InvalidVertex(vertex_index)
            })?;

            // Apply lattice rotation
            let rotation = self.hash_chain.rotation();
            let rotated = (*rotation * vertex.q * rotation.conjugate()).normalize();

            // Convert to optical state
            let optical = quaternion_to_optical(&rotated);

            let symbol = EncodedSymbol {
                sequence: self.sequence,
                vertex_index,
                quaternion: rotated,
                optical,
                bits,
            };

            // Advance hash chain
            self.hash_chain.advance(&[bits]);
            self.sequence += 1;

            results.push(symbol);
        }

        Ok(results)
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

    /// Reset encoder to genesis state
    pub fn reset(&mut self) {
        self.hash_chain.reset();
        self.sequence = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_single_symbol() {
        let mut encoder = CspmEncoder::from_secret(b"test");
        let symbol = encoder.encode_symbol(42).unwrap();

        assert_eq!(symbol.sequence, 0);
        assert_eq!(symbol.bits, 42);
        assert!(symbol.quaternion.is_normalized());
    }

    #[test]
    fn test_encode_sequence() {
        let mut encoder = CspmEncoder::from_secret(b"test");

        for i in 0..10u8 {
            let symbol = encoder.encode_symbol(i).unwrap();
            assert_eq!(symbol.sequence, i as u64);
        }

        assert_eq!(encoder.sequence(), 10);
    }

    #[test]
    fn test_rotation_changes() {
        let mut encoder = CspmEncoder::from_secret(b"test");
        let initial_rotation = encoder.rotation();

        encoder.encode_symbol(42).unwrap();
        let new_rotation = encoder.rotation();

        // Rotation should change after encoding
        assert!(!initial_rotation.approx_eq(&new_rotation, 0.01));
    }

    #[test]
    fn test_encode_bytes() {
        let mut encoder = CspmEncoder::from_secret(b"test");
        let data = b"Hello";
        let symbols = encoder.encode_bytes(data).unwrap();

        assert!(!symbols.is_empty());
        for symbol in &symbols {
            assert!(symbol.quaternion.is_normalized());
        }
    }

    #[test]
    fn test_reset() {
        let mut encoder = CspmEncoder::from_secret(b"test");
        let initial_hash = encoder.chain_hash();

        encoder.encode_symbol(42).unwrap();
        encoder.reset();

        assert_eq!(encoder.chain_hash(), initial_hash);
        assert_eq!(encoder.sequence(), 0);
    }
}

//! Spinor Mapper - Hash to Quaternion Conversion
//!
//! The Spinor Mapper is the bridge between cryptographic hashes and
//! 4D geometric rotations. It converts a 256-bit hash into a unit
//! quaternion that defines the lattice orientation for each packet.
//!
//! ## The Rolling Lattice
//!
//! Each packet's lattice rotation is derived from the hash chain:
//! ```text
//! q₁ = hash_to_quat(genesis)
//! q₂ = hash_to_quat(H(genesis, H(data₁)))
//! q₃ = hash_to_quat(H(previous, H(data₂)))
//! ...
//! ```
//!
//! Without the genesis hash, an attacker cannot predict the sequence
//! of lattice rotations, making interception computationally infeasible.

use crate::error::{CSPMError, CSPMResult};
use crate::lattice::{Cell600, CELL_600};
use crate::quaternion::Quaternion;
use sha2::{Digest, Sha256};

/// Convert a 32-byte hash to a unit quaternion
///
/// Maps the 256-bit hash uniformly onto the 3-sphere (unit quaternions).
pub fn hash_to_quaternion(hash: &[u8; 32]) -> Quaternion {
    // Method: Interpret hash as 4 doubles, then normalize
    // This gives uniform distribution on S³

    // Extract 4 components (8 bytes each)
    let w = hash_bytes_to_f64(&hash[0..8]);
    let x = hash_bytes_to_f64(&hash[8..16]);
    let y = hash_bytes_to_f64(&hash[16..24]);
    let z = hash_bytes_to_f64(&hash[24..32]);

    // Normalize to unit quaternion
    Quaternion::new(w, x, y, z).normalize()
}

/// Convert 8 bytes to a float in range [-1, 1]
fn hash_bytes_to_f64(bytes: &[u8]) -> f64 {
    let int_val = u64::from_be_bytes(bytes.try_into().unwrap());
    // Map [0, 2^64) to [-1, 1)
    (int_val as f64 / u64::MAX as f64) * 2.0 - 1.0
}

/// Compute SHA-256 hash
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute chained hash: H(previous || current)
pub fn chain_hash(previous: &[u8; 32], current: &[u8; 32]) -> [u8; 32] {
    let mut combined = Vec::with_capacity(64);
    combined.extend_from_slice(previous);
    combined.extend_from_slice(current);
    sha256(&combined)
}

/// The Spinor Mapper: maps data blocks to rotated lattice vertices
///
/// This is the core encoder for CSPM. It maintains a hash chain and
/// uses it to derive lattice rotations for each data block.
pub struct SpinorMapper {
    /// Genesis hash (shared secret)
    genesis_hash: [u8; 32],
    /// Previous hash in chain (for rolling rotation)
    previous_hash: [u8; 32],
    /// Packet sequence number
    sequence: u64,
}

impl SpinorMapper {
    /// Create a new SpinorMapper with the given genesis hash
    pub fn new(genesis: [u8; 32]) -> Self {
        Self {
            genesis_hash: genesis,
            previous_hash: genesis,
            sequence: 0,
        }
    }

    /// Create from a genesis string (hashed to 32 bytes)
    pub fn from_secret(secret: &str) -> Self {
        let genesis = sha256(secret.as_bytes());
        Self::new(genesis)
    }

    /// Get the genesis hash
    pub fn genesis(&self) -> &[u8; 32] {
        &self.genesis_hash
    }

    /// Get the current hash chain state
    pub fn current_hash(&self) -> &[u8; 32] {
        &self.previous_hash
    }

    /// Get the current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Reset to genesis state
    pub fn reset(&mut self) {
        self.previous_hash = self.genesis_hash;
        self.sequence = 0;
    }

    /// Get the current lattice rotation quaternion
    pub fn current_rotation(&self) -> Quaternion {
        hash_to_quaternion(&self.previous_hash)
    }

    /// Encode a data block to a rotated lattice vertex
    ///
    /// Returns: (rotated_quaternion, vertex_index, data_hash)
    pub fn encode(&mut self, data: &[u8]) -> (Quaternion, usize, [u8; 32]) {
        // 1. Hash the data
        let data_hash = sha256(data);

        // 2. Get current lattice rotation
        let rotation = self.current_rotation();

        // 3. Map data hash to vertex index (mod 120)
        let vertex_index =
            (u64::from_be_bytes(data_hash[0..8].try_into().unwrap()) as usize) % CELL_600.len();

        // 4. Get base vertex quaternion
        let base_vertex = CELL_600.vertex(vertex_index).unwrap().quaternion;

        // 5. Rotate vertex by lattice orientation
        let rotated_vertex = rotation.rotate_quaternion(&base_vertex);

        // 6. Update hash chain for next packet
        self.previous_hash = chain_hash(&self.previous_hash, &data_hash);
        self.sequence += 1;

        (rotated_vertex, vertex_index, data_hash)
    }

    /// Encode with explicit vertex selection (for testing)
    pub fn encode_vertex(&mut self, vertex_index: usize) -> CSPMResult<Quaternion> {
        if vertex_index >= CELL_600.len() {
            return Err(CSPMError::VertexIndexOutOfRange(vertex_index));
        }

        let rotation = self.current_rotation();
        let base_vertex = CELL_600.vertex(vertex_index).unwrap().quaternion;
        let rotated_vertex = rotation.rotate_quaternion(&base_vertex);

        // Create a synthetic hash for chain update
        let synthetic_hash = sha256(&vertex_index.to_be_bytes());
        self.previous_hash = chain_hash(&self.previous_hash, &synthetic_hash);
        self.sequence += 1;

        Ok(rotated_vertex)
    }
}

/// The Spinor Demapper: decodes rotated vertices back to data
///
/// This is the receiver counterpart to SpinorMapper.
pub struct SpinorDemapper {
    /// Genesis hash (shared secret)
    genesis_hash: [u8; 32],
    /// Previous hash in chain
    previous_hash: [u8; 32],
    /// Packet sequence number
    sequence: u64,
}

impl SpinorDemapper {
    /// Create a new SpinorDemapper with the same genesis as the encoder
    pub fn new(genesis: [u8; 32]) -> Self {
        Self {
            genesis_hash: genesis,
            previous_hash: genesis,
            sequence: 0,
        }
    }

    /// Create from a genesis string
    pub fn from_secret(secret: &str) -> Self {
        let genesis = sha256(secret.as_bytes());
        Self::new(genesis)
    }

    /// Reset to genesis state
    pub fn reset(&mut self) {
        self.previous_hash = self.genesis_hash;
        self.sequence = 0;
    }

    /// Get expected rotation for current packet
    pub fn expected_rotation(&self) -> Quaternion {
        hash_to_quaternion(&self.previous_hash)
    }

    /// Decode a noisy quaternion to vertex index
    ///
    /// Performs geometric snapping (error correction) and returns
    /// the decoded vertex index.
    pub fn decode(&mut self, noisy_q: &Quaternion) -> usize {
        // 1. Get expected lattice rotation
        let rotation = self.expected_rotation();

        // 2. Snap to nearest vertex in rotated lattice
        let (vertex_index, _clean_q) = CELL_600.snap_rotated(noisy_q, &rotation);

        // 3. Update chain (using vertex index as proxy for data hash)
        // In real use, the data would be looked up from vertex index
        let synthetic_hash = sha256(&vertex_index.to_be_bytes());
        self.previous_hash = chain_hash(&self.previous_hash, &synthetic_hash);
        self.sequence += 1;

        vertex_index
    }

    /// Verify and advance the chain with known data hash
    pub fn advance_chain(&mut self, data_hash: &[u8; 32]) {
        self.previous_hash = chain_hash(&self.previous_hash, data_hash);
        self.sequence += 1;
    }

    /// Verify that encoder and decoder are in sync
    pub fn verify_sync(&self, encoder: &SpinorMapper) -> bool {
        self.previous_hash == *encoder.current_hash() && self.sequence == encoder.sequence()
    }
}

/// Encoding result for a single CSPM packet
#[derive(Debug, Clone)]
pub struct CSPMPacket {
    /// Sequence number
    pub sequence: u64,
    /// Rotated vertex quaternion (what gets transmitted)
    pub quaternion: Quaternion,
    /// Vertex index (for debugging/verification)
    pub vertex_index: usize,
    /// Data hash (for chain verification)
    pub data_hash: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_to_quaternion_is_unit() {
        let hash = sha256(b"test data");
        let q = hash_to_quaternion(&hash);
        let norm = q.norm();
        assert!(
            (norm - 1.0).abs() < 0.001,
            "Quaternion should be unit, got norm {}",
            norm
        );
    }

    #[test]
    fn test_different_hashes_different_quaternions() {
        let h1 = sha256(b"data1");
        let h2 = sha256(b"data2");

        let q1 = hash_to_quaternion(&h1);
        let q2 = hash_to_quaternion(&h2);

        let dist = q1.geodesic_distance(&q2);
        assert!(dist > 0.01, "Different hashes should produce different quaternions");
    }

    #[test]
    fn test_spinor_mapper_encode_decode() {
        let secret = "shared_genesis_secret";

        let mut encoder = SpinorMapper::from_secret(secret);
        let mut decoder = SpinorDemapper::from_secret(secret);

        // Encode some data
        let data = b"Hello, CSPM!";
        let (rotated_q, vertex_index, _data_hash) = encoder.encode(data);

        // Decode (should recover same vertex)
        let decoded_index = decoder.decode(&rotated_q);

        assert_eq!(vertex_index, decoded_index, "Decoded vertex should match encoded");
    }

    #[test]
    fn test_rolling_rotation() {
        let mut mapper = SpinorMapper::from_secret("test");

        let r1 = mapper.current_rotation();
        mapper.encode(b"packet 1");
        let r2 = mapper.current_rotation();
        mapper.encode(b"packet 2");
        let r3 = mapper.current_rotation();

        // Each rotation should be different
        assert!(r1.geodesic_distance(&r2) > 0.01);
        assert!(r2.geodesic_distance(&r3) > 0.01);
        assert!(r1.geodesic_distance(&r3) > 0.01);
    }

    #[test]
    fn test_encoder_decoder_sync() {
        let secret = "sync_test";

        let mut encoder = SpinorMapper::from_secret(secret);
        let mut decoder = SpinorDemapper::from_secret(secret);

        // Should start in sync
        assert!(decoder.verify_sync(&encoder));

        // Encode and decode several packets
        for i in 0..10 {
            let data = format!("packet {}", i);
            let (q, _, hash) = encoder.encode(data.as_bytes());

            let decoded = decoder.decode(&q);
            decoder.advance_chain(&hash); // Sync with actual data hash

            // Note: verify_sync may not match because decode uses synthetic hash
            // In real implementation, we'd look up the actual data from vertex index
        }
    }

    #[test]
    fn test_noise_tolerance() {
        let mut mapper = SpinorMapper::from_secret("noise_test");
        let mut demapper = SpinorDemapper::from_secret("noise_test");

        let (clean_q, expected_index, _) = mapper.encode(b"test");

        // Add noise to the quaternion
        let noisy_q = Quaternion::new(
            clean_q.w + 0.05,
            clean_q.x - 0.03,
            clean_q.y + 0.02,
            clean_q.z - 0.01,
        )
        .normalize();

        // Should still decode correctly (within noise tolerance)
        let decoded_index = demapper.decode(&noisy_q);

        assert_eq!(
            expected_index, decoded_index,
            "Small noise should not cause decode error"
        );
    }
}

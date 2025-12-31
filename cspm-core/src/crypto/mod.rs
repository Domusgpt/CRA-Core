//! Cryptographic primitives for CSPM.
//!
//! Provides hash chain management and hash-to-quaternion conversion
//! for dynamic lattice rotation.

use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

use crate::quaternion::Quaternion;

/// State of the hash chain at a given point
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainState {
    /// Current accumulated hash
    pub hash: [u8; 32],
    /// Current lattice rotation quaternion
    pub rotation: Quaternion,
    /// Chain depth (number of events processed)
    pub depth: u64,
}

impl ChainState {
    /// Create a new chain state from genesis hash
    pub fn genesis(genesis_hash: &[u8; 32]) -> Self {
        let rotation = hash_to_quaternion(genesis_hash);
        Self {
            hash: *genesis_hash,
            rotation,
            depth: 0,
        }
    }

    /// Advance the chain with a new event hash
    pub fn advance(&self, event_data: &[u8]) -> Self {
        // Hash the event data
        let event_hash = sha256(event_data);

        // Chain the hashes: H(current || event)
        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(&self.hash);
        combined.extend_from_slice(&event_hash);
        let new_hash = sha256(&combined);

        // Compute rotation delta from event hash
        let delta_rotation = hash_to_quaternion(&event_hash);

        // Accumulate rotation: R_new = R_old * delta
        let new_rotation = (self.rotation * delta_rotation).normalize();

        Self {
            hash: new_hash,
            rotation: new_rotation,
            depth: self.depth + 1,
        }
    }

    /// Verify that a chain state correctly follows from a previous state
    pub fn verify_follows(&self, previous: &ChainState, event_data: &[u8]) -> bool {
        let expected = previous.advance(event_data);
        self.hash == expected.hash && self.depth == expected.depth
    }
}

/// Hash chain for managing CSPM lattice rotations
pub struct HashChain {
    /// Current chain state
    state: ChainState,
    /// Genesis hash for verification
    genesis_hash: [u8; 32],
}

impl HashChain {
    /// Create a new hash chain from genesis
    pub fn new(genesis_hash: [u8; 32]) -> Self {
        Self {
            state: ChainState::genesis(&genesis_hash),
            genesis_hash,
        }
    }

    /// Create from shared secret
    pub fn from_secret(secret: &[u8]) -> Self {
        let genesis_hash = sha256(secret);
        Self::new(genesis_hash)
    }

    /// Get current chain state
    pub fn state(&self) -> &ChainState {
        &self.state
    }

    /// Get current rotation
    pub fn rotation(&self) -> &Quaternion {
        &self.state.rotation
    }

    /// Get current hash
    pub fn hash(&self) -> &[u8; 32] {
        &self.state.hash
    }

    /// Get chain depth
    pub fn depth(&self) -> u64 {
        self.state.depth
    }

    /// Advance the chain with packet data
    pub fn advance(&mut self, packet_data: &[u8]) {
        self.state = self.state.advance(packet_data);
    }

    /// Get genesis hash
    pub fn genesis_hash(&self) -> &[u8; 32] {
        &self.genesis_hash
    }

    /// Reset chain to genesis
    pub fn reset(&mut self) {
        self.state = ChainState::genesis(&self.genesis_hash);
    }

    /// Resync to a specific state
    pub fn resync(&mut self, state: ChainState) {
        self.state = state;
    }
}

/// Convert a 32-byte hash to a unit quaternion
pub fn hash_to_quaternion(hash: &[u8; 32]) -> Quaternion {
    // Use first 16 bytes to create 4 f32 values, then normalize
    // This provides uniform distribution on S³

    // Extract 4 floats from first 16 bytes
    let w = bytes_to_f64(&hash[0..4]);
    let x = bytes_to_f64(&hash[4..8]);
    let y = bytes_to_f64(&hash[8..12]);
    let z = bytes_to_f64(&hash[12..16]);

    // Normalize to unit quaternion
    Quaternion::new(w, x, y, z).normalize()
}

/// Convert 4 bytes to f64 in range [-1, 1]
fn bytes_to_f64(bytes: &[u8]) -> f64 {
    let value = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    // Map [0, u32::MAX] to [-1, 1]
    (value as f64 / u32::MAX as f64) * 2.0 - 1.0
}

/// Compute SHA-256 hash
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute SHA-256 hash and return as hex string
pub fn sha256_hex(data: &[u8]) -> String {
    hex::encode(sha256(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_to_quaternion_normalized() {
        let hash = sha256(b"test data");
        let q = hash_to_quaternion(&hash);
        assert!(q.is_normalized());
    }

    #[test]
    fn test_hash_to_quaternion_deterministic() {
        let hash = sha256(b"test data");
        let q1 = hash_to_quaternion(&hash);
        let q2 = hash_to_quaternion(&hash);
        assert!(q1.approx_eq(&q2, 1e-10));
    }

    #[test]
    fn test_hash_to_quaternion_different_inputs() {
        let hash1 = sha256(b"test data 1");
        let hash2 = sha256(b"test data 2");
        let q1 = hash_to_quaternion(&hash1);
        let q2 = hash_to_quaternion(&hash2);

        // Different inputs should give different quaternions
        assert!(!q1.approx_eq(&q2, 0.1));
    }

    #[test]
    fn test_chain_advance() {
        let mut chain = HashChain::from_secret(b"secret");

        let initial_hash = chain.hash().clone();
        let initial_rotation = *chain.rotation();

        chain.advance(b"packet 1");

        // Hash and rotation should change
        assert_ne!(chain.hash(), &initial_hash);
        assert!(!chain.rotation().approx_eq(&initial_rotation, 0.1));
        assert_eq!(chain.depth(), 1);
    }

    #[test]
    fn test_chain_deterministic() {
        let mut chain1 = HashChain::from_secret(b"secret");
        let mut chain2 = HashChain::from_secret(b"secret");

        chain1.advance(b"packet 1");
        chain2.advance(b"packet 1");

        assert_eq!(chain1.hash(), chain2.hash());
        assert!(chain1.rotation().approx_eq(chain2.rotation(), 1e-10));
    }

    #[test]
    fn test_chain_order_matters() {
        let mut chain1 = HashChain::from_secret(b"secret");
        let mut chain2 = HashChain::from_secret(b"secret");

        chain1.advance(b"packet A");
        chain1.advance(b"packet B");

        chain2.advance(b"packet B");
        chain2.advance(b"packet A");

        // Different order should give different results
        assert_ne!(chain1.hash(), chain2.hash());
    }

    #[test]
    fn test_state_verify() {
        let chain = HashChain::from_secret(b"secret");
        let state1 = chain.state().clone();

        let state2 = state1.advance(b"packet data");

        assert!(state2.verify_follows(&state1, b"packet data"));
        assert!(!state2.verify_follows(&state1, b"wrong data"));
    }

    #[test]
    fn test_chain_reset() {
        let mut chain = HashChain::from_secret(b"secret");
        let genesis = chain.state().clone();

        chain.advance(b"packet 1");
        chain.advance(b"packet 2");

        chain.reset();

        assert_eq!(*chain.hash(), genesis.hash);
        assert_eq!(chain.depth(), 0);
    }
}

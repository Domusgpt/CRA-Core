//! CSPM Codec - Complete Encode/Decode Pipeline
//!
//! Combines all components for end-to-end CSPM communication:
//! - Data → Hash chain → Lattice rotation → Physical encoding
//! - Physical decoding → Geometric snap → Data recovery

use crate::channel::{ChannelNoiseModel, PhysicalDecoder, PhysicalEncoder, PhysicalState};
use crate::error::{CSPMError, CSPMResult};
use crate::lattice::CELL_600;
use crate::quaternion::Quaternion;
use crate::spinor::{sha256, SpinorDemapper, SpinorMapper};

/// Complete CSPM encoder
///
/// Combines hash chain, lattice rotation, and physical encoding.
pub struct CSPMEncoder {
    spinor_mapper: SpinorMapper,
    physical_encoder: PhysicalEncoder,
}

impl CSPMEncoder {
    /// Create a new encoder with the given genesis secret
    pub fn new(genesis_secret: &str) -> Self {
        Self {
            spinor_mapper: SpinorMapper::from_secret(genesis_secret),
            physical_encoder: PhysicalEncoder::default_encoder(),
        }
    }

    /// Create from raw genesis hash
    pub fn from_genesis(genesis: [u8; 32]) -> Self {
        Self {
            spinor_mapper: SpinorMapper::new(genesis),
            physical_encoder: PhysicalEncoder::default_encoder(),
        }
    }

    /// Get the genesis hash
    pub fn genesis(&self) -> &[u8; 32] {
        self.spinor_mapper.genesis()
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.spinor_mapper.sequence()
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.spinor_mapper.reset();
    }

    /// Encode data to physical state
    ///
    /// Returns the physical state for transmission and metadata for verification.
    pub fn encode(&mut self, data: &[u8]) -> EncodedPacket {
        let (quaternion, vertex_index, data_hash) = self.spinor_mapper.encode(data);
        let physical_state = self.physical_encoder.encode(&quaternion);

        EncodedPacket {
            sequence: self.spinor_mapper.sequence() - 1,
            quaternion,
            vertex_index,
            data_hash,
            physical_state,
        }
    }

    /// Encode raw bytes with known length
    pub fn encode_bytes(&mut self, bytes: &[u8]) -> EncodedPacket {
        self.encode(bytes)
    }
}

/// Complete CSPM decoder
///
/// Combines physical decoding, geometric snap, and hash chain verification.
pub struct CSPMDecoder {
    spinor_demapper: SpinorDemapper,
    physical_decoder: PhysicalDecoder,
}

impl CSPMDecoder {
    /// Create a new decoder with the same genesis as the encoder
    pub fn new(genesis_secret: &str) -> Self {
        Self {
            spinor_demapper: SpinorDemapper::from_secret(genesis_secret),
            physical_decoder: PhysicalDecoder::default_decoder(),
        }
    }

    /// Create from raw genesis hash
    pub fn from_genesis(genesis: [u8; 32]) -> Self {
        Self {
            spinor_demapper: SpinorDemapper::new(genesis),
            physical_decoder: PhysicalDecoder::default_decoder(),
        }
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.spinor_demapper.reset();
    }

    /// Decode a physical state
    ///
    /// Performs geometric error correction (snap) and returns the vertex index.
    pub fn decode(&mut self, physical_state: &PhysicalState) -> DecodedPacket {
        // 1. Recover noisy quaternion from physical state
        let noisy_q = self.physical_decoder.decode(physical_state);

        // 2. Get expected rotation from hash chain
        let expected_rotation = self.spinor_demapper.expected_rotation();

        // 3. Snap to nearest vertex (geometric error correction)
        let (vertex_index, clean_q) = CELL_600.snap_rotated(&noisy_q, &expected_rotation);

        // 4. Compute geodesic distance (quality metric)
        let snap_distance = noisy_q.geodesic_distance(&clean_q);

        // 5. Advance chain (using vertex index as proxy)
        let synthetic_hash = sha256(&vertex_index.to_be_bytes());
        self.spinor_demapper.advance_chain(&synthetic_hash);

        DecodedPacket {
            vertex_index,
            clean_quaternion: clean_q,
            noisy_quaternion: noisy_q,
            snap_distance,
        }
    }

    /// Advance chain with known data hash (for sync with encoder)
    pub fn sync_chain(&mut self, data_hash: &[u8; 32]) {
        self.spinor_demapper.advance_chain(data_hash);
    }
}

/// Encoded packet metadata
#[derive(Debug, Clone)]
pub struct EncodedPacket {
    /// Sequence number
    pub sequence: u64,
    /// Rotated vertex quaternion
    pub quaternion: Quaternion,
    /// Vertex index (0-119)
    pub vertex_index: usize,
    /// SHA-256 hash of data
    pub data_hash: [u8; 32],
    /// Physical encoding for transmission
    pub physical_state: PhysicalState,
}

/// Decoded packet result
#[derive(Debug, Clone)]
pub struct DecodedPacket {
    /// Decoded vertex index
    pub vertex_index: usize,
    /// Clean (snapped) quaternion
    pub clean_quaternion: Quaternion,
    /// Original noisy quaternion
    pub noisy_quaternion: Quaternion,
    /// Geodesic distance (quality metric)
    pub snap_distance: f64,
}

impl DecodedPacket {
    /// Check if decode was likely correct (low snap distance)
    pub fn is_high_confidence(&self) -> bool {
        // If snap distance is less than half the vertex separation, high confidence
        self.snap_distance < 0.32 // ~18.4° in radians
    }
}

/// End-to-end CSPM simulation
pub struct CSPMSimulation {
    encoder: CSPMEncoder,
    decoder: CSPMDecoder,
    noise_model: ChannelNoiseModel,
}

impl CSPMSimulation {
    /// Create a new simulation with shared secret
    pub fn new(genesis_secret: &str, noise_model: ChannelNoiseModel) -> Self {
        Self {
            encoder: CSPMEncoder::new(genesis_secret),
            decoder: CSPMDecoder::new(genesis_secret),
            noise_model,
        }
    }

    /// Run simulation for a single packet
    pub fn simulate_packet(&mut self, data: &[u8]) -> SimulationResult {
        let mut rng = rand::thread_rng();

        // Encode
        let encoded = self.encoder.encode(data);

        // Apply channel noise
        let noisy_state = self.noise_model.apply(&encoded.physical_state, &mut rng);

        // Decode
        let decoded = self.decoder.decode(&noisy_state);

        // Sync decoder chain with actual data hash
        self.decoder.sync_chain(&encoded.data_hash);

        // Check if correct
        let is_correct = decoded.vertex_index == encoded.vertex_index;

        SimulationResult {
            encoded,
            decoded,
            noisy_state,
            is_correct,
        }
    }

    /// Run simulation for many packets, return bit error rate
    pub fn run_ber_simulation(&mut self, num_packets: usize) -> BERResult {
        let mut errors = 0;
        let mut total_snap_distance = 0.0;
        let mut rng = rand::thread_rng();

        for i in 0..num_packets {
            // Generate random data
            let data = format!("packet_{}", i);
            let result = self.simulate_packet(data.as_bytes());

            if !result.is_correct {
                errors += 1;
            }
            total_snap_distance += result.decoded.snap_distance;
        }

        BERResult {
            total_packets: num_packets,
            errors,
            bit_error_rate: errors as f64 / num_packets as f64,
            average_snap_distance: total_snap_distance / num_packets as f64,
        }
    }

    /// Reset simulation
    pub fn reset(&mut self) {
        self.encoder.reset();
        self.decoder.reset();
    }
}

/// Single packet simulation result
#[derive(Debug)]
pub struct SimulationResult {
    pub encoded: EncodedPacket,
    pub decoded: DecodedPacket,
    pub noisy_state: PhysicalState,
    pub is_correct: bool,
}

/// Bit error rate simulation result
#[derive(Debug)]
pub struct BERResult {
    pub total_packets: usize,
    pub errors: usize,
    pub bit_error_rate: f64,
    pub average_snap_distance: f64,
}

impl std::fmt::Display for BERResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BER: {:.2e} ({} errors in {} packets), avg snap distance: {:.4} rad",
            self.bit_error_rate, self.errors, self.total_packets, self.average_snap_distance
        )
    }
}

/// Lookup table for vertex → data mapping
///
/// In a real system, this maps vertex indices to data blocks.
pub struct VertexCodebook {
    /// Data blocks indexed by vertex
    data: Vec<Vec<u8>>,
}

impl VertexCodebook {
    /// Create an empty codebook
    pub fn new() -> Self {
        Self {
            data: vec![Vec::new(); 120],
        }
    }

    /// Set data for a vertex
    pub fn set(&mut self, vertex_index: usize, data: Vec<u8>) -> CSPMResult<()> {
        if vertex_index >= 120 {
            return Err(CSPMError::VertexIndexOutOfRange(vertex_index));
        }
        self.data[vertex_index] = data;
        Ok(())
    }

    /// Get data for a vertex
    pub fn get(&self, vertex_index: usize) -> Option<&[u8]> {
        self.data.get(vertex_index).map(|v| v.as_slice())
    }
}

impl Default for VertexCodebook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_no_noise() {
        let secret = "test_secret";
        let mut encoder = CSPMEncoder::new(secret);
        let mut decoder = CSPMDecoder::new(secret);

        let data = b"Hello, CSPM!";
        let encoded = encoder.encode(data);

        // Decode without noise
        let decoded = decoder.decode(&encoded.physical_state);

        assert_eq!(
            encoded.vertex_index, decoded.vertex_index,
            "Clean decode should match"
        );
    }

    #[test]
    fn test_simulation_low_noise() {
        let mut sim = CSPMSimulation::new("sim_test", ChannelNoiseModel::low_noise());

        let result = sim.run_ber_simulation(100);

        println!("{}", result);

        // Low noise should have very low BER
        assert!(
            result.bit_error_rate < 0.1,
            "Low noise BER should be <10%: {}",
            result.bit_error_rate
        );
    }

    #[test]
    fn test_simulation_medium_noise() {
        let mut sim = CSPMSimulation::new("sim_test", ChannelNoiseModel::medium_noise());

        let result = sim.run_ber_simulation(100);

        println!("{}", result);

        // Medium noise will have some errors, but geometric correction helps
        // This is informational - actual BER depends on noise model tuning
    }

    #[test]
    fn test_sequence_sync() {
        let secret = "sync_test";
        let mut encoder = CSPMEncoder::new(secret);
        let mut decoder = CSPMDecoder::new(secret);

        // Encode and decode several packets
        for i in 0..10 {
            let data = format!("packet {}", i);
            let encoded = encoder.encode(data.as_bytes());
            let decoded = decoder.decode(&encoded.physical_state);
            decoder.sync_chain(&encoded.data_hash);

            assert_eq!(
                encoded.vertex_index, decoded.vertex_index,
                "Packet {} decode failed",
                i
            );
        }
    }

    #[test]
    fn test_codebook() {
        let mut codebook = VertexCodebook::new();

        codebook.set(0, b"data_for_vertex_0".to_vec()).unwrap();
        codebook.set(42, b"data_for_vertex_42".to_vec()).unwrap();

        assert_eq!(codebook.get(0), Some(b"data_for_vertex_0".as_slice()));
        assert_eq!(codebook.get(42), Some(b"data_for_vertex_42".as_slice()));
        assert!(codebook.get(1).unwrap().is_empty());
    }
}

//! Frame structure with sequence numbers.
//!
//! Defines the CSPM frame format for reliable transmission:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                          CSPM Frame                              │
//! ├──────────┬─────────────┬─────────────────┬───────────┬──────────┤
//! │ Preamble │   Header    │     Payload     │ Checkpoint│  Guard   │
//! │ 8 symb.  │  4 symbols  │   N symbols     │ 2 symbols │ 2 symb.  │
//! └──────────┴─────────────┴─────────────────┴───────────┴──────────┘
//!
//! Header (4 symbols = 28 bits):
//! ┌─────────────────┬─────────────────┬──────────────────┐
//! │  Sequence (14b) │  Length (8b)    │  Flags (6b)      │
//! └─────────────────┴─────────────────┴──────────────────┘
//! ```

use crate::quaternion::Quaternion;
use crate::polytope::Hexacosichoron;
use crate::crypto::ChainState;
use super::preamble::Preamble;

/// Frame header containing metadata
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameHeader {
    /// Sequence number (14 bits, wraps at 16384)
    pub sequence: u16,
    /// Payload length in symbols (8 bits, max 255)
    pub length: u8,
    /// Frame flags
    pub flags: FrameFlags,
}

/// Frame flags (6 bits)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FrameFlags {
    /// Frame contains checkpoint data
    pub has_checkpoint: bool,
    /// Frame is a retransmission
    pub is_retransmit: bool,
    /// Frame requires acknowledgment
    pub requires_ack: bool,
    /// More fragments follow
    pub more_fragments: bool,
    /// Frame contains priority data
    pub priority: bool,
    /// Reserved for future use
    pub reserved: bool,
}

impl FrameFlags {
    /// Encode flags to 6-bit value
    pub fn encode(&self) -> u8 {
        let mut value = 0u8;
        if self.has_checkpoint { value |= 0x01; }
        if self.is_retransmit { value |= 0x02; }
        if self.requires_ack { value |= 0x04; }
        if self.more_fragments { value |= 0x08; }
        if self.priority { value |= 0x10; }
        if self.reserved { value |= 0x20; }
        value
    }

    /// Decode flags from 6-bit value
    pub fn decode(value: u8) -> Self {
        Self {
            has_checkpoint: (value & 0x01) != 0,
            is_retransmit: (value & 0x02) != 0,
            requires_ack: (value & 0x04) != 0,
            more_fragments: (value & 0x08) != 0,
            priority: (value & 0x10) != 0,
            reserved: (value & 0x20) != 0,
        }
    }
}

impl FrameHeader {
    /// Create new header
    pub fn new(sequence: u16, length: u8) -> Self {
        Self {
            sequence: sequence & 0x3FFF, // 14 bits
            length,
            flags: FrameFlags::default(),
        }
    }

    /// Create header with flags
    pub fn with_flags(sequence: u16, length: u8, flags: FrameFlags) -> Self {
        Self {
            sequence: sequence & 0x3FFF,
            length,
            flags,
        }
    }

    /// Encode header to 28 bits (4 symbols × 7 bits)
    pub fn encode(&self) -> u32 {
        let flags = self.flags.encode() as u32;
        ((self.sequence as u32) << 14) | ((self.length as u32) << 6) | flags
    }

    /// Decode header from 28 bits
    pub fn decode(value: u32) -> Self {
        let sequence = ((value >> 14) & 0x3FFF) as u16;
        let length = ((value >> 6) & 0xFF) as u8;
        let flags = FrameFlags::decode((value & 0x3F) as u8);
        Self { sequence, length, flags }
    }

    /// Encode header to quaternion symbols
    pub fn to_symbols(&self, hex: &Hexacosichoron) -> Vec<Quaternion> {
        let encoded = self.encode();
        let vertices = hex.vertices();

        // Extract 4 × 7-bit values
        let indices = [
            ((encoded >> 21) & 0x7F) as usize,
            ((encoded >> 14) & 0x7F) as usize,
            ((encoded >> 7) & 0x7F) as usize,
            (encoded & 0x7F) as usize,
        ];

        indices
            .iter()
            .map(|&idx| vertices[idx.min(119)].q)
            .collect()
    }

    /// Decode header from quaternion symbols
    pub fn from_symbols(symbols: &[Quaternion], hex: &Hexacosichoron) -> Option<Self> {
        if symbols.len() < 4 {
            return None;
        }

        let vertices = hex.vertices();
        let mut encoded = 0u32;

        for (i, q) in symbols.iter().take(4).enumerate() {
            // Find nearest vertex
            let mut min_dist = f64::MAX;
            let mut nearest = 0usize;
            for (j, v) in vertices.iter().enumerate() {
                let dist = q.distance(&v.q);
                if dist < min_dist {
                    min_dist = dist;
                    nearest = j;
                }
            }
            // Pack into encoded value
            let shift = 21 - (i * 7);
            encoded |= ((nearest as u32) & 0x7F) << shift;
        }

        Some(Self::decode(encoded))
    }
}

/// Complete CSPM frame
#[derive(Clone, Debug)]
pub struct Frame {
    /// Frame header
    pub header: FrameHeader,
    /// Payload symbols
    pub payload: Vec<Quaternion>,
    /// Optional checkpoint (if flags.has_checkpoint)
    pub checkpoint: Option<FrameCheckpoint>,
}

/// Embedded checkpoint in frame
#[derive(Clone, Debug)]
pub struct FrameCheckpoint {
    /// Truncated hash (16 bits) for verification
    pub hash_prefix: u16,
    /// Chain depth at checkpoint
    pub depth_low: u8,
}

impl FrameCheckpoint {
    /// Create from chain state
    pub fn from_chain_state(state: &ChainState) -> Self {
        let hash_prefix = u16::from_le_bytes([state.hash[0], state.hash[1]]);
        let depth_low = (state.depth & 0xFF) as u8;
        Self { hash_prefix, depth_low }
    }

    /// Encode to 2 symbols (14 bits)
    pub fn encode(&self) -> u16 {
        // 16-bit hash truncated to 14 bits
        self.hash_prefix & 0x3FFF
    }

    /// Decode from value
    pub fn decode(value: u16, depth_hint: u64) -> Self {
        Self {
            hash_prefix: value & 0x3FFF,
            depth_low: (depth_hint & 0xFF) as u8,
        }
    }

    /// Verify against chain state
    pub fn verify(&self, state: &ChainState) -> bool {
        let expected_prefix = u16::from_le_bytes([state.hash[0], state.hash[1]]);
        (expected_prefix & 0x3FFF) == (self.hash_prefix & 0x3FFF)
    }
}

impl Frame {
    /// Create new frame
    pub fn new(header: FrameHeader, payload: Vec<Quaternion>) -> Self {
        Self {
            header,
            payload,
            checkpoint: None,
        }
    }

    /// Create frame with checkpoint
    pub fn with_checkpoint(
        header: FrameHeader,
        payload: Vec<Quaternion>,
        chain_state: &ChainState,
    ) -> Self {
        let mut header = header;
        header.flags.has_checkpoint = true;

        Self {
            header,
            payload,
            checkpoint: Some(FrameCheckpoint::from_chain_state(chain_state)),
        }
    }

    /// Get total frame length in symbols
    pub fn total_length(&self) -> usize {
        let preamble_len = 8;
        let header_len = 4;
        let checkpoint_len = if self.checkpoint.is_some() { 2 } else { 0 };
        let guard_len = 2;

        preamble_len + header_len + self.payload.len() + checkpoint_len + guard_len
    }
}

/// Frame builder for constructing frames
pub struct FrameBuilder {
    hex: Hexacosichoron,
    preamble: Preamble,
    sequence: u16,
}

impl FrameBuilder {
    /// Create new frame builder
    pub fn new() -> Self {
        Self {
            hex: Hexacosichoron::new(),
            preamble: Preamble::default(),
            sequence: 0,
        }
    }

    /// Create frame builder with initial sequence
    pub fn with_sequence(sequence: u16) -> Self {
        Self {
            hex: Hexacosichoron::new(),
            preamble: Preamble::default(),
            sequence,
        }
    }

    /// Build frame from payload
    pub fn build(&mut self, payload: Vec<Quaternion>) -> Frame {
        let header = FrameHeader::new(self.sequence, payload.len().min(255) as u8);
        self.sequence = self.sequence.wrapping_add(1) & 0x3FFF;
        Frame::new(header, payload)
    }

    /// Build frame with checkpoint
    pub fn build_with_checkpoint(
        &mut self,
        payload: Vec<Quaternion>,
        chain_state: &ChainState,
    ) -> Frame {
        let header = FrameHeader::new(self.sequence, payload.len().min(255) as u8);
        self.sequence = self.sequence.wrapping_add(1) & 0x3FFF;
        Frame::with_checkpoint(header, payload, chain_state)
    }

    /// Serialize frame to symbol stream
    pub fn serialize(&self, frame: &Frame) -> Vec<Quaternion> {
        let mut symbols = Vec::with_capacity(frame.total_length());

        // Preamble
        symbols.extend(self.preamble.sequence());

        // Header
        symbols.extend(frame.header.to_symbols(&self.hex));

        // Payload
        symbols.extend(&frame.payload);

        // Checkpoint (if present)
        if let Some(cp) = &frame.checkpoint {
            let cp_val = cp.encode();
            // Encode as 2 symbols (7 bits each = 14 bits)
            symbols.push(self.hex.vertices()[(cp_val >> 7) as usize & 0x7F].q);
            symbols.push(self.hex.vertices()[(cp_val & 0x7F) as usize].q);
        }

        // Guard symbols (identity quaternions for easy detection)
        let guard = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        symbols.push(guard);
        symbols.push(guard);

        symbols
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u16 {
        self.sequence
    }

    /// Set sequence number
    pub fn set_sequence(&mut self, seq: u16) {
        self.sequence = seq & 0x3FFF;
    }
}

impl Default for FrameBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame parser for receiving frames
pub struct FrameParser {
    hex: Hexacosichoron,
    preamble: Preamble,
    expected_sequence: u16,
}

impl FrameParser {
    /// Create new frame parser
    pub fn new() -> Self {
        Self {
            hex: Hexacosichoron::new(),
            preamble: Preamble::default(),
            expected_sequence: 0,
        }
    }

    /// Parse frame from symbol stream (after preamble detection)
    pub fn parse(&mut self, symbols: &[Quaternion]) -> Result<Frame, FrameParseError> {
        if symbols.len() < 4 {
            return Err(FrameParseError::TooShort);
        }

        // Parse header (first 4 symbols)
        let header = FrameHeader::from_symbols(&symbols[0..4], &self.hex)
            .ok_or(FrameParseError::InvalidHeader)?;

        // Validate length
        let payload_start = 4;
        let payload_end = payload_start + header.length as usize;

        if symbols.len() < payload_end {
            return Err(FrameParseError::PayloadTruncated);
        }

        let payload = symbols[payload_start..payload_end].to_vec();

        // Parse checkpoint if present
        let checkpoint = if header.flags.has_checkpoint {
            let cp_start = payload_end;
            if symbols.len() < cp_start + 2 {
                return Err(FrameParseError::CheckpointMissing);
            }

            // Decode checkpoint from 2 symbols
            let cp_hi = self.nearest_vertex(&symbols[cp_start]) & 0x7F;
            let cp_lo = self.nearest_vertex(&symbols[cp_start + 1]) & 0x7F;
            let cp_val = ((cp_hi as u16) << 7) | (cp_lo as u16);

            Some(FrameCheckpoint::decode(cp_val, 0))
        } else {
            None
        };

        // Check sequence
        let sequence_gap = if header.sequence >= self.expected_sequence {
            header.sequence - self.expected_sequence
        } else {
            // Wrapped
            (0x4000 - self.expected_sequence) + header.sequence
        };

        if sequence_gap > 100 {
            return Err(FrameParseError::SequenceGap {
                expected: self.expected_sequence,
                received: header.sequence,
            });
        }

        self.expected_sequence = header.sequence.wrapping_add(1) & 0x3FFF;

        Ok(Frame {
            header,
            payload,
            checkpoint,
        })
    }

    /// Find nearest vertex index for a quaternion
    fn nearest_vertex(&self, q: &Quaternion) -> usize {
        let vertices = self.hex.vertices();
        let mut min_dist = f64::MAX;
        let mut nearest = 0usize;

        for (i, v) in vertices.iter().enumerate() {
            let dist = q.distance(&v.q);
            if dist < min_dist {
                min_dist = dist;
                nearest = i;
            }
        }
        nearest
    }

    /// Reset expected sequence
    pub fn reset(&mut self) {
        self.expected_sequence = 0;
    }

    /// Set expected sequence
    pub fn set_expected_sequence(&mut self, seq: u16) {
        self.expected_sequence = seq & 0x3FFF;
    }
}

impl Default for FrameParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame parsing error
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameParseError {
    /// Frame too short
    TooShort,
    /// Invalid header encoding
    InvalidHeader,
    /// Payload truncated
    PayloadTruncated,
    /// Checkpoint expected but missing
    CheckpointMissing,
    /// Sequence number gap
    SequenceGap { expected: u16, received: u16 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_flags_roundtrip() {
        let flags = FrameFlags {
            has_checkpoint: true,
            is_retransmit: false,
            requires_ack: true,
            more_fragments: false,
            priority: true,
            reserved: false,
        };

        let encoded = flags.encode();
        let decoded = FrameFlags::decode(encoded);
        assert_eq!(flags, decoded);
    }

    #[test]
    fn test_frame_header_roundtrip() {
        let header = FrameHeader::with_flags(
            1234,
            128,
            FrameFlags {
                has_checkpoint: true,
                requires_ack: true,
                ..Default::default()
            },
        );

        let encoded = header.encode();
        let decoded = FrameHeader::decode(encoded);

        assert_eq!(header.sequence, decoded.sequence);
        assert_eq!(header.length, decoded.length);
        assert_eq!(header.flags, decoded.flags);
    }

    #[test]
    fn test_header_symbols_roundtrip() {
        let hex = Hexacosichoron::new();
        let header = FrameHeader::new(100, 64);

        let symbols = header.to_symbols(&hex);
        assert_eq!(symbols.len(), 4);

        let recovered = FrameHeader::from_symbols(&symbols, &hex).unwrap();
        assert_eq!(header.sequence, recovered.sequence);
        assert_eq!(header.length, recovered.length);
    }

    #[test]
    fn test_frame_builder() {
        let mut builder = FrameBuilder::new();
        let hex = Hexacosichoron::new();

        let payload: Vec<Quaternion> = (0..10)
            .map(|i| hex.vertices()[i * 5].q)
            .collect();

        let frame = builder.build(payload.clone());

        assert_eq!(frame.header.sequence, 0);
        assert_eq!(frame.header.length, 10);
        assert_eq!(frame.payload.len(), 10);

        // Next frame should have sequence 1
        let frame2 = builder.build(payload);
        assert_eq!(frame2.header.sequence, 1);
    }

    #[test]
    fn test_frame_serialization() {
        let mut builder = FrameBuilder::new();
        let hex = Hexacosichoron::new();

        let payload: Vec<Quaternion> = (0..10)
            .map(|i| hex.vertices()[i * 5].q)
            .collect();

        let frame = builder.build(payload);
        let serialized = builder.serialize(&frame);

        // Preamble(8) + Header(4) + Payload(10) + Guard(2) = 24
        assert_eq!(serialized.len(), 24);
    }

    #[test]
    fn test_frame_parse_roundtrip() {
        let mut builder = FrameBuilder::new();
        let mut parser = FrameParser::new();
        let hex = Hexacosichoron::new();

        let payload: Vec<Quaternion> = (0..10)
            .map(|i| hex.vertices()[i * 5].q)
            .collect();

        let frame = builder.build(payload.clone());
        let serialized = builder.serialize(&frame);

        // Skip preamble for parsing
        let parsed = parser.parse(&serialized[8..]).unwrap();

        assert_eq!(parsed.header.sequence, 0);
        assert_eq!(parsed.header.length, 10);
        assert_eq!(parsed.payload.len(), 10);
    }

    #[test]
    fn test_checkpoint_in_frame() {
        use crate::crypto::sha256;

        let mut builder = FrameBuilder::new();
        let hex = Hexacosichoron::new();

        let chain_state = ChainState::genesis(&sha256(b"test"));

        let payload: Vec<Quaternion> = (0..10)
            .map(|i| hex.vertices()[i * 5].q)
            .collect();

        let frame = builder.build_with_checkpoint(payload, &chain_state);

        assert!(frame.header.flags.has_checkpoint);
        assert!(frame.checkpoint.is_some());

        let cp = frame.checkpoint.unwrap();
        assert!(cp.verify(&chain_state));
    }

    #[test]
    fn test_sequence_gap_detection() {
        let mut parser = FrameParser::new();
        let hex = Hexacosichoron::new();

        // Create header with sequence 50 (gap from expected 0)
        let header = FrameHeader::new(50, 5);
        let mut symbols = header.to_symbols(&hex);
        symbols.extend((0..5).map(|_| Quaternion::new(1.0, 0.0, 0.0, 0.0)));

        let result = parser.parse(&symbols);
        assert!(result.is_ok()); // Gap of 50 is within tolerance

        // Very large gap should fail
        parser.reset();
        let header2 = FrameHeader::new(500, 5);
        let mut symbols2 = header2.to_symbols(&hex);
        symbols2.extend((0..5).map(|_| Quaternion::new(1.0, 0.0, 0.0, 0.0)));

        let result2 = parser.parse(&symbols2);
        assert!(matches!(result2, Err(FrameParseError::SequenceGap { .. })));
    }
}

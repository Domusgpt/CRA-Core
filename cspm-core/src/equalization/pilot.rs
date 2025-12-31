//! Pilot symbol insertion and extraction for channel estimation.
//!
//! Pilots are known quaternion symbols inserted at regular intervals
//! that allow the receiver to estimate and track channel variations.

use crate::quaternion::Quaternion;
use crate::polytope::Hexacosichoron;

/// Defines the pattern of pilot symbol insertion
#[derive(Clone, Debug)]
pub struct PilotPattern {
    /// Spacing between pilot symbols
    pub spacing: usize,
    /// Pilot sequence (indices into constellation)
    pub sequence: Vec<usize>,
    /// Current position in sequence
    sequence_pos: usize,
}

impl PilotPattern {
    /// Create uniform pilot pattern (one pilot every N symbols)
    pub fn uniform(spacing: usize) -> Self {
        // Use vertices that are maximally separated for good estimation
        // These are from different groups of the 600-cell
        let sequence = vec![0, 60, 30, 90, 15, 75, 45, 105];

        Self {
            spacing,
            sequence,
            sequence_pos: 0,
        }
    }

    /// Create pilot pattern for high-speed tracking
    pub fn fast_tracking() -> Self {
        Self::uniform(8) // Pilot every 8 symbols
    }

    /// Create pilot pattern for stable channels
    pub fn slow_tracking() -> Self {
        Self::uniform(64) // Pilot every 64 symbols
    }

    /// Check if position is a pilot
    pub fn is_pilot_position(&self, position: usize) -> bool {
        position % self.spacing == 0
    }

    /// Get next pilot vertex index
    pub fn next_pilot_index(&mut self) -> usize {
        let idx = self.sequence[self.sequence_pos];
        self.sequence_pos = (self.sequence_pos + 1) % self.sequence.len();
        idx
    }

    /// Get pilot index for specific position
    pub fn pilot_index_at(&self, position: usize) -> usize {
        let seq_pos = (position / self.spacing) % self.sequence.len();
        self.sequence[seq_pos]
    }

    /// Reset sequence position
    pub fn reset(&mut self) {
        self.sequence_pos = 0;
    }

    /// Calculate overhead percentage
    pub fn overhead_percent(&self) -> f64 {
        100.0 / self.spacing as f64
    }
}

/// Inserts pilot symbols into a data stream
pub struct PilotInserter {
    pattern: PilotPattern,
    hexacosichoron: Hexacosichoron,
    position: usize,
}

impl PilotInserter {
    /// Create new pilot inserter
    pub fn new(pattern: PilotPattern) -> Self {
        Self {
            pattern,
            hexacosichoron: Hexacosichoron::new(),
            position: 0,
        }
    }

    /// Insert pilots into a sequence of data quaternions
    pub fn insert(&mut self, data: &[Quaternion]) -> Vec<PilotOrData> {
        let mut output = Vec::new();
        let mut data_idx = 0;

        while data_idx < data.len() {
            if self.pattern.is_pilot_position(self.position) {
                // Insert pilot
                let pilot_idx = self.pattern.pilot_index_at(self.position);
                let pilot_q = self.hexacosichoron.vertices()[pilot_idx].q;
                output.push(PilotOrData::Pilot {
                    quaternion: pilot_q,
                    vertex_index: pilot_idx,
                    position: self.position,
                });
            } else {
                // Insert data
                output.push(PilotOrData::Data {
                    quaternion: data[data_idx],
                    position: self.position,
                });
                data_idx += 1;
            }
            self.position += 1;
        }

        // Ensure we end on a non-pilot if there's remaining data
        while data_idx < data.len() {
            if !self.pattern.is_pilot_position(self.position) {
                output.push(PilotOrData::Data {
                    quaternion: data[data_idx],
                    position: self.position,
                });
                data_idx += 1;
            } else {
                let pilot_idx = self.pattern.pilot_index_at(self.position);
                let pilot_q = self.hexacosichoron.vertices()[pilot_idx].q;
                output.push(PilotOrData::Pilot {
                    quaternion: pilot_q,
                    vertex_index: pilot_idx,
                    position: self.position,
                });
            }
            self.position += 1;
        }

        output
    }

    /// Reset inserter state
    pub fn reset(&mut self) {
        self.pattern.reset();
        self.position = 0;
    }
}

/// Pilot or data symbol
#[derive(Clone, Debug)]
pub enum PilotOrData {
    Pilot {
        quaternion: Quaternion,
        vertex_index: usize,
        position: usize,
    },
    Data {
        quaternion: Quaternion,
        position: usize,
    },
}

impl PilotOrData {
    pub fn quaternion(&self) -> &Quaternion {
        match self {
            PilotOrData::Pilot { quaternion, .. } => quaternion,
            PilotOrData::Data { quaternion, .. } => quaternion,
        }
    }

    pub fn position(&self) -> usize {
        match self {
            PilotOrData::Pilot { position, .. } => *position,
            PilotOrData::Data { position, .. } => *position,
        }
    }

    pub fn is_pilot(&self) -> bool {
        matches!(self, PilotOrData::Pilot { .. })
    }
}

/// Extracts pilot symbols from received stream
pub struct PilotExtractor {
    pattern: PilotPattern,
    hexacosichoron: Hexacosichoron,
}

impl PilotExtractor {
    /// Create new pilot extractor
    pub fn new(pattern: PilotPattern) -> Self {
        Self {
            pattern,
            hexacosichoron: Hexacosichoron::new(),
        }
    }

    /// Extract pilots and data from received symbols
    pub fn extract(&self, received: &[Quaternion]) -> ExtractionResult {
        let mut pilots = Vec::new();
        let mut data = Vec::new();

        for (pos, q) in received.iter().enumerate() {
            if self.pattern.is_pilot_position(pos) {
                let expected_idx = self.pattern.pilot_index_at(pos);
                let expected_q = self.hexacosichoron.vertices()[expected_idx].q;

                pilots.push(PilotMeasurement {
                    position: pos,
                    expected: expected_q,
                    received: *q,
                    vertex_index: expected_idx,
                });
            } else {
                data.push(DataSymbol {
                    position: pos,
                    quaternion: *q,
                });
            }
        }

        ExtractionResult { pilots, data }
    }

    /// Get the known pilot quaternion for a position
    pub fn expected_pilot(&self, position: usize) -> Option<Quaternion> {
        if self.pattern.is_pilot_position(position) {
            let idx = self.pattern.pilot_index_at(position);
            Some(self.hexacosichoron.vertices()[idx].q)
        } else {
            None
        }
    }
}

/// Result of pilot extraction
#[derive(Clone, Debug)]
pub struct ExtractionResult {
    /// Extracted pilot measurements
    pub pilots: Vec<PilotMeasurement>,
    /// Extracted data symbols
    pub data: Vec<DataSymbol>,
}

/// A pilot measurement
#[derive(Clone, Debug)]
pub struct PilotMeasurement {
    /// Position in stream
    pub position: usize,
    /// Expected (transmitted) quaternion
    pub expected: Quaternion,
    /// Received quaternion
    pub received: Quaternion,
    /// Vertex index
    pub vertex_index: usize,
}

impl PilotMeasurement {
    /// Estimate channel rotation from this pilot
    pub fn estimate_channel(&self) -> super::ChannelRotation {
        // Find rotation R such that: received ≈ R * expected * R†
        // This is: R = received * expected† (simplified for small rotations)
        let rotation = self.received * self.expected.conjugate();

        // Calculate confidence based on deviation from unit quaternion
        // (perfect channel gives |rotation| = 1)
        let norm = rotation.norm();
        let confidence = 1.0 - (norm - 1.0).abs().min(1.0);

        super::ChannelRotation {
            rotation: rotation.normalize(),
            confidence,
            timestamp: self.position as u64,
        }
    }

    /// Error magnitude (distance between expected and received after compensation)
    pub fn error_magnitude(&self, channel: &super::ChannelRotation) -> f64 {
        let compensated = channel.compensate(&self.received);
        self.expected.distance(&compensated)
    }
}

/// A data symbol
#[derive(Clone, Debug)]
pub struct DataSymbol {
    /// Position in stream
    pub position: usize,
    /// Received quaternion
    pub quaternion: Quaternion,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pilot_pattern_uniform() {
        let pattern = PilotPattern::uniform(16);

        assert!(pattern.is_pilot_position(0));
        assert!(!pattern.is_pilot_position(1));
        assert!(pattern.is_pilot_position(16));
        assert!(pattern.is_pilot_position(32));
    }

    #[test]
    fn test_pilot_overhead() {
        let pattern = PilotPattern::uniform(16);
        assert!((pattern.overhead_percent() - 6.25).abs() < 0.01);
    }

    #[test]
    fn test_pilot_inserter() {
        let pattern = PilotPattern::uniform(4);
        let mut inserter = PilotInserter::new(pattern);

        // Need 7 data items to get past second pilot (at position 4)
        let data = vec![
            Quaternion::new(0.5, 0.5, 0.5, 0.5),
            Quaternion::new(0.5, -0.5, 0.5, -0.5),
            Quaternion::new(0.5, 0.5, -0.5, -0.5),
            Quaternion::new(0.5, 0.5, 0.5, -0.5),
            Quaternion::new(-0.5, 0.5, 0.5, 0.5),
            Quaternion::new(0.5, -0.5, -0.5, 0.5),
            Quaternion::new(-0.5, -0.5, 0.5, 0.5),
        ];

        let output = inserter.insert(&data);

        // First should be pilot (position 0)
        assert!(output[0].is_pilot());

        // Then data at positions 1, 2, 3
        assert!(!output[1].is_pilot());
        assert!(!output[2].is_pilot());
        assert!(!output[3].is_pilot());

        // Position 4 should be pilot again
        assert!(output[4].is_pilot());

        // Verify we have enough output
        assert!(output.len() >= 5);
    }

    #[test]
    fn test_pilot_extraction() {
        let pattern = PilotPattern::uniform(4);
        let extractor = PilotExtractor::new(pattern.clone());

        // Create a sequence with pilots at positions 0, 4, 8
        let mut inserter = PilotInserter::new(pattern);
        let data = vec![
            Quaternion::new(0.5, 0.5, 0.5, 0.5),
            Quaternion::new(0.5, -0.5, 0.5, -0.5),
            Quaternion::new(0.5, 0.5, -0.5, -0.5),
        ];

        let inserted = inserter.insert(&data);
        let received: Vec<Quaternion> = inserted.iter().map(|p| *p.quaternion()).collect();

        let result = extractor.extract(&received);

        // Should have pilots at 0, 4
        assert!(result.pilots.len() >= 1);
        assert_eq!(result.pilots[0].position, 0);
    }

    #[test]
    fn test_channel_estimation_from_pilot() {
        let hex = Hexacosichoron::new();
        let expected = hex.vertices()[0].q;

        // Simulate channel rotation
        let channel = Quaternion::from_axis_angle([0.0, 0.0, 1.0], 0.3);
        let received = channel * expected * channel.conjugate();

        let measurement = PilotMeasurement {
            position: 0,
            expected,
            received: received.normalize(),
            vertex_index: 0,
        };

        let estimated = measurement.estimate_channel();

        // Should recover approximately the original channel
        // Note: there's a sign ambiguity in quaternion rotations
        let error1 = estimated.rotation.distance(&channel);
        let neg_channel = Quaternion::new(-channel.w, -channel.x, -channel.y, -channel.z);
        let error2 = estimated.rotation.distance(&neg_channel);
        let min_error = error1.min(error2);

        assert!(min_error < 0.2, "Channel estimation error: {}", min_error);
    }
}

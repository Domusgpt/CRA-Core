//! Preamble design for frame synchronization.
//!
//! The preamble is a unique quaternion sequence that marks frame boundaries.
//! It is designed for:
//! - Low autocorrelation sidelobes (unique detection)
//! - Robustness to channel rotation
//! - Minimal false positive rate
//!
//! ## Design
//!
//! Uses maximally separated vertices from the 600-cell to create a
//! sequence with good correlation properties. The preamble exploits
//! the geometric structure: vertices are chosen from different
//! "rings" of the polytope.

use crate::quaternion::Quaternion;
use crate::polytope::Hexacosichoron;

/// Preamble configuration
#[derive(Clone, Debug)]
pub struct PreambleConfig {
    /// Length of preamble in symbols
    pub length: usize,
    /// Correlation threshold for detection (0-1)
    pub detection_threshold: f64,
    /// Minimum gap between preamble detections
    pub min_gap: usize,
}

impl Default for PreambleConfig {
    fn default() -> Self {
        Self {
            length: 8,
            detection_threshold: 0.85,
            min_gap: 16,
        }
    }
}

/// Preamble sequence generator and matcher
#[derive(Clone, Debug)]
pub struct Preamble {
    /// Preamble quaternion sequence
    sequence: Vec<Quaternion>,
    /// Vertex indices used (for reference)
    vertex_indices: Vec<usize>,
    /// Configuration
    config: PreambleConfig,
}

impl Preamble {
    /// Create standard CSPM preamble
    pub fn new(config: PreambleConfig) -> Self {
        let hex = Hexacosichoron::new();
        let vertices = hex.vertices();

        // Select maximally separated vertices for preamble
        // These indices are chosen to maximize angular separation
        // and minimize autocorrelation sidelobes
        let vertex_indices = Self::select_preamble_vertices(config.length);

        let sequence: Vec<Quaternion> = vertex_indices
            .iter()
            .map(|&idx| vertices[idx].q)
            .collect();

        Self {
            sequence,
            vertex_indices,
            config,
        }
    }

    /// Get preamble sequence
    pub fn sequence(&self) -> &[Quaternion] {
        &self.sequence
    }

    /// Get vertex indices
    pub fn vertex_indices(&self) -> &[usize] {
        &self.vertex_indices
    }

    /// Get preamble length
    pub fn length(&self) -> usize {
        self.config.length
    }

    /// Compute correlation with received sequence
    pub fn correlate(&self, received: &[Quaternion]) -> f64 {
        if received.len() < self.sequence.len() {
            return 0.0;
        }

        // Use quaternion inner product for correlation
        // For unit quaternions, |q1 · q2| measures alignment
        let mut correlation = 0.0;
        for (i, preamble_q) in self.sequence.iter().enumerate() {
            let received_q = &received[i];
            // Inner product: w1*w2 + x1*x2 + y1*y2 + z1*z2
            let dot = preamble_q.w * received_q.w
                + preamble_q.x * received_q.x
                + preamble_q.y * received_q.y
                + preamble_q.z * received_q.z;
            // Use absolute value due to quaternion double-cover
            correlation += dot.abs();
        }

        correlation / self.sequence.len() as f64
    }

    /// Compute correlation with channel compensation
    pub fn correlate_with_channel(&self, received: &[Quaternion], channel: &Quaternion) -> f64 {
        if received.len() < self.sequence.len() {
            return 0.0;
        }

        let channel_inv = channel.conjugate();
        let mut correlation = 0.0;

        for (i, preamble_q) in self.sequence.iter().enumerate() {
            // Compensate for channel: recovered = channel_inv * received * channel
            let compensated = channel_inv * received[i] * *channel;
            let dot = preamble_q.w * compensated.w
                + preamble_q.x * compensated.x
                + preamble_q.y * compensated.y
                + preamble_q.z * compensated.z;
            correlation += dot.abs();
        }

        correlation / self.sequence.len() as f64
    }

    /// Select vertex indices for preamble with good correlation properties
    fn select_preamble_vertices(length: usize) -> Vec<usize> {
        // These vertices are selected from different "shells" of the 600-cell
        // to maximize separation and minimize autocorrelation sidelobes
        //
        // The 120 vertices of the 600-cell can be grouped:
        // - 8 vertices: permutations of (±1, 0, 0, 0)
        // - 16 vertices: (±1/2, ±1/2, ±1/2, ±1/2)
        // - 96 vertices: even permutations of (±φ/2, ±1/2, ±1/(2φ), 0)
        //
        // We select from each group for diversity

        let base_indices = [
            0,   // (1, 0, 0, 0)
            16,  // From second group
            24,  // From third group
            48,  // Different orientation
            72,  // Another shell
            96,  // Fourth shell
            108, // Antipodal region
            116, // Final region
        ];

        base_indices.iter().take(length).copied().collect()
    }

    /// Generate rotated preamble (for testing channel estimation)
    pub fn rotated(&self, channel: &Quaternion) -> Vec<Quaternion> {
        self.sequence
            .iter()
            .map(|q| (*channel * *q * channel.conjugate()).normalize())
            .collect()
    }
}

impl Default for Preamble {
    fn default() -> Self {
        Self::new(PreambleConfig::default())
    }
}

/// Preamble detector for incoming symbol stream
pub struct PreambleDetector {
    preamble: Preamble,
    config: PreambleConfig,
    /// Buffer for incoming symbols
    buffer: Vec<Quaternion>,
    /// Last detection position
    last_detection: Option<usize>,
    /// Total symbols processed
    symbols_processed: usize,
}

impl PreambleDetector {
    /// Create new detector
    pub fn new(config: PreambleConfig) -> Self {
        Self {
            preamble: Preamble::new(config.clone()),
            buffer: Vec::with_capacity(config.length * 2),
            last_detection: None,
            symbols_processed: 0,
            config,
        }
    }

    /// Process symbols and detect preamble
    pub fn detect(&mut self, symbols: &[Quaternion]) -> Option<usize> {
        for (i, q) in symbols.iter().enumerate() {
            self.buffer.push(*q);
            self.symbols_processed += 1;

            // Keep buffer at appropriate size
            if self.buffer.len() > self.config.length * 2 {
                self.buffer.remove(0);
            }

            // Check for preamble when buffer is full enough
            if self.buffer.len() >= self.config.length {
                let start = self.buffer.len() - self.config.length;
                let window = &self.buffer[start..];
                let correlation = self.preamble.correlate(window);

                if correlation >= self.config.detection_threshold {
                    // Check minimum gap
                    let global_pos = self.symbols_processed - self.config.length;
                    if let Some(last) = self.last_detection {
                        if global_pos - last < self.config.min_gap {
                            continue;
                        }
                    }

                    self.last_detection = Some(global_pos);
                    return Some(i + 1 - self.config.length);
                }
            }
        }

        None
    }

    /// Sliding window correlation across entire buffer
    pub fn correlation_profile(&self, symbols: &[Quaternion]) -> Vec<f64> {
        if symbols.len() < self.config.length {
            return vec![];
        }

        (0..=symbols.len() - self.config.length)
            .map(|i| self.preamble.correlate(&symbols[i..i + self.config.length]))
            .collect()
    }

    /// Get the preamble
    pub fn preamble(&self) -> &Preamble {
        &self.preamble
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.last_detection = None;
        self.symbols_processed = 0;
    }
}

/// Differential preamble for improved robustness
/// Uses differences between consecutive symbols rather than absolute values
#[derive(Clone, Debug)]
pub struct DifferentialPreamble {
    /// Base preamble
    base: Preamble,
    /// Differential pattern (transitions between symbols)
    differentials: Vec<Quaternion>,
}

impl DifferentialPreamble {
    /// Create differential preamble
    pub fn new(config: PreambleConfig) -> Self {
        let base = Preamble::new(config);

        // Compute differential pattern
        let differentials: Vec<Quaternion> = base.sequence
            .windows(2)
            .map(|w| (w[1] * w[0].conjugate()).normalize())
            .collect();

        Self { base, differentials }
    }

    /// Correlate using differential pattern
    pub fn correlate(&self, received: &[Quaternion]) -> f64 {
        if received.len() < self.base.sequence.len() {
            return 0.0;
        }

        let mut correlation = 0.0;
        for (i, expected_diff) in self.differentials.iter().enumerate() {
            let actual_diff = (received[i + 1] * received[i].conjugate()).normalize();
            let dot = expected_diff.w * actual_diff.w
                + expected_diff.x * actual_diff.x
                + expected_diff.y * actual_diff.y
                + expected_diff.z * actual_diff.z;
            correlation += dot.abs();
        }

        correlation / self.differentials.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preamble_creation() {
        let preamble = Preamble::default();
        assert_eq!(preamble.length(), 8);
        assert_eq!(preamble.sequence().len(), 8);

        // All symbols should be normalized
        for q in preamble.sequence() {
            assert!(q.is_normalized());
        }
    }

    #[test]
    fn test_preamble_self_correlation() {
        let preamble = Preamble::default();
        let correlation = preamble.correlate(preamble.sequence());

        // Self-correlation should be very high
        assert!(correlation > 0.99, "Self-correlation: {}", correlation);
    }

    #[test]
    fn test_preamble_cross_correlation() {
        let preamble = Preamble::default();
        let hex = Hexacosichoron::new();

        // Random sequence should have low correlation
        let random_seq: Vec<Quaternion> = (0..8)
            .map(|i| hex.vertices()[(i * 13 + 7) % 120].q)
            .collect();

        let correlation = preamble.correlate(&random_seq);
        assert!(correlation < 0.7, "Cross-correlation too high: {}", correlation);
    }

    #[test]
    fn test_preamble_with_channel() {
        let preamble = Preamble::default();
        let channel = Quaternion::from_axis_angle([0.0, 1.0, 0.0], 0.5);

        // Rotate preamble through channel
        let rotated = preamble.rotated(&channel);

        // Without compensation, correlation drops
        let raw_corr = preamble.correlate(&rotated);

        // With compensation, correlation is restored
        let comp_corr = preamble.correlate_with_channel(&rotated, &channel);

        assert!(comp_corr > raw_corr, "Compensation should help");
        assert!(comp_corr > 0.95, "Compensated correlation: {}", comp_corr);
    }

    #[test]
    fn test_detector_finds_preamble() {
        let config = PreambleConfig::default();
        let mut detector = PreambleDetector::new(config);
        let hex = Hexacosichoron::new();

        // Build stream: random + preamble + random
        let mut stream: Vec<Quaternion> = (0..10)
            .map(|i| hex.vertices()[(i * 7) % 120].q)
            .collect();

        // Insert preamble
        stream.extend(detector.preamble().sequence().iter().copied());

        // More random data
        stream.extend((0..10).map(|i| hex.vertices()[(i * 11 + 3) % 120].q));

        // Detect
        let pos = detector.detect(&stream);
        assert!(pos.is_some(), "Preamble not detected");
    }

    #[test]
    fn test_correlation_profile() {
        let config = PreambleConfig::default();
        let detector = PreambleDetector::new(config);
        let hex = Hexacosichoron::new();

        // Build stream with preamble in middle
        let mut stream: Vec<Quaternion> = (0..10)
            .map(|i| hex.vertices()[(i * 7) % 120].q)
            .collect();

        stream.extend(detector.preamble().sequence().iter().copied());

        stream.extend((0..10).map(|i| hex.vertices()[(i * 11 + 3) % 120].q));

        let profile = detector.correlation_profile(&stream);

        // Should have a peak around position 10
        let max_pos = profile
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i);

        assert_eq!(max_pos, Some(10), "Peak should be at preamble start");
    }

    #[test]
    fn test_differential_preamble() {
        let diff_preamble = DifferentialPreamble::new(PreambleConfig::default());

        // Self-correlation should be high
        let correlation = diff_preamble.correlate(diff_preamble.base.sequence());
        assert!(correlation > 0.99, "Differential self-correlation: {}", correlation);
    }

    #[test]
    fn test_differential_channel_invariance() {
        let diff_preamble = DifferentialPreamble::new(PreambleConfig::default());
        let channel = Quaternion::from_axis_angle([1.0, 0.0, 0.0], 0.3);

        // Rotate through channel
        let rotated = diff_preamble.base.rotated(&channel);

        // Differential preamble should still match well
        // because channel affects all symbols similarly
        let correlation = diff_preamble.correlate(&rotated);
        assert!(correlation > 0.9, "Differential correlation with channel: {}", correlation);
    }
}

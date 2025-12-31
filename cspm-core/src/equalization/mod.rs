//! Channel equalization for CSPM.
//!
//! This module provides algorithms to compensate for channel effects
//! (PMD, turbulence, phase distortion) that would otherwise corrupt
//! the received quaternion signal.
//!
//! ## Equalization Strategies
//!
//! 1. **Pilot-based**: Insert known symbols, estimate channel, compensate
//! 2. **Blind (CMA)**: Exploit constant modulus property of quaternion constellation
//! 3. **Decision-directed**: Refine estimates using decoded symbols
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cspm_core::equalization::{PilotPattern, ChannelEstimator, Equalizer};
//!
//! // Create pilot pattern (1 pilot every 16 symbols)
//! let pilot_pattern = PilotPattern::uniform(16);
//!
//! // Estimate channel from received pilots
//! let estimator = ChannelEstimator::least_squares();
//! let channel_estimate = estimator.estimate(&received_pilots, &known_pilots);
//!
//! // Apply equalization
//! let equalizer = Equalizer::new(channel_estimate);
//! let equalized = equalizer.apply(&received_symbols);
//! ```

pub mod pilot;
pub mod estimator;
pub mod cma;
pub mod adaptive;

pub use pilot::{PilotPattern, PilotInserter, PilotExtractor};
pub use estimator::{ChannelEstimate, ChannelEstimator, EstimatorConfig};
pub use cma::{CmaEqualizer, CmaConfig};
pub use adaptive::{AdaptiveEqualizer, EqualizerMode};

use crate::quaternion::Quaternion;

/// Represents a channel effect as a quaternion rotation
///
/// The channel is modeled as: received = H * transmitted * H†
/// where H is a unit quaternion representing the channel rotation.
#[derive(Clone, Debug)]
pub struct ChannelRotation {
    /// The rotation quaternion
    pub rotation: Quaternion,
    /// Confidence in estimate (0-1)
    pub confidence: f64,
    /// Estimation timestamp (symbol index)
    pub timestamp: u64,
}

impl ChannelRotation {
    /// Identity channel (no rotation)
    pub fn identity() -> Self {
        Self {
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            confidence: 1.0,
            timestamp: 0,
        }
    }

    /// Create from rotation quaternion
    pub fn new(rotation: Quaternion, confidence: f64) -> Self {
        Self {
            rotation: rotation.normalize(),
            confidence,
            timestamp: 0,
        }
    }

    /// Apply channel effect (what the channel does)
    pub fn apply(&self, q: &Quaternion) -> Quaternion {
        let result = self.rotation * *q * self.rotation.conjugate();
        result.normalize()
    }

    /// Compensate for channel effect (what the equalizer does)
    pub fn compensate(&self, q: &Quaternion) -> Quaternion {
        let inv = self.rotation.conjugate();
        let result = inv * *q * self.rotation;
        result.normalize()
    }

    /// Interpolate between two channel estimates
    pub fn interpolate(&self, other: &ChannelRotation, t: f64) -> ChannelRotation {
        ChannelRotation {
            rotation: self.rotation.slerp(&other.rotation, t),
            confidence: self.confidence * (1.0 - t) + other.confidence * t,
            timestamp: self.timestamp,
        }
    }

    /// Combine two channel rotations (composition)
    pub fn compose(&self, other: &ChannelRotation) -> ChannelRotation {
        ChannelRotation {
            rotation: (self.rotation * other.rotation).normalize(),
            confidence: self.confidence * other.confidence,
            timestamp: self.timestamp.max(other.timestamp),
        }
    }
}

/// Result of equalization process
#[derive(Clone, Debug)]
pub struct EqualizationResult {
    /// Equalized quaternion
    pub quaternion: Quaternion,
    /// Estimated SNR (dB)
    pub estimated_snr_db: f64,
    /// Was this a pilot symbol?
    pub is_pilot: bool,
    /// Channel estimate at this position
    pub channel: ChannelRotation,
}

/// Equalization statistics
#[derive(Clone, Debug, Default)]
pub struct EqualizationStats {
    /// Number of symbols processed
    pub symbols_processed: u64,
    /// Number of pilot symbols
    pub pilots_processed: u64,
    /// Average estimation error (radians)
    pub avg_estimation_error: f64,
    /// Channel variation rate (radians/symbol)
    pub channel_variation_rate: f64,
    /// Current SNR estimate (dB)
    pub current_snr_db: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_rotation_identity() {
        let channel = ChannelRotation::identity();
        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);

        let applied = channel.apply(&q);
        assert!(q.distance(&applied) < 1e-10);
    }

    #[test]
    fn test_channel_compensation() {
        // Create a channel rotation
        let channel = ChannelRotation::new(
            Quaternion::from_axis_angle([0.0, 0.0, 1.0], 0.5),
            1.0
        );

        let original = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let distorted = channel.apply(&original);
        let recovered = channel.compensate(&distorted);

        assert!(original.distance(&recovered) < 1e-10);
    }

    #[test]
    fn test_channel_interpolation() {
        let c1 = ChannelRotation::identity();
        let c2 = ChannelRotation::new(
            Quaternion::from_axis_angle([0.0, 0.0, 1.0], 1.0),
            1.0
        );

        let mid = c1.interpolate(&c2, 0.5);

        // Midpoint should be half rotation
        let expected = Quaternion::from_axis_angle([0.0, 0.0, 1.0], 0.5);
        assert!(mid.rotation.distance(&expected) < 0.01);
    }
}

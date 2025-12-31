//! Adaptive equalizer combining pilot-based and blind methods.
//!
//! This module provides a unified equalizer that:
//! 1. Uses pilot symbols for initial acquisition
//! 2. Switches to CMA for tracking
//! 3. Uses decision-directed updates after decoding
//!
//! ## Operating Modes
//!
//! - **Acquisition**: Waiting for pilots, high step size
//! - **Tracking**: CMA with pilot refinement
//! - **Steady**: Low step size, decision-directed

use crate::quaternion::Quaternion;
use super::{
    ChannelRotation, EqualizationResult, EqualizationStats,
    pilot::{PilotPattern, PilotExtractor, PilotMeasurement},
    estimator::{ChannelEstimator, ChannelEstimate, ChannelInterpolator},
    cma::CmaEqualizer,
};

/// Operating mode of the adaptive equalizer
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EqualizerMode {
    /// Waiting for initial pilots
    Acquisition,
    /// Pilot-based tracking
    PilotTracking,
    /// CMA blind tracking
    BlindTracking,
    /// Decision-directed refinement
    DecisionDirected,
}

/// Configuration for adaptive equalizer
#[derive(Clone, Debug)]
pub struct AdaptiveConfig {
    /// Pilot pattern
    pub pilot_pattern: PilotPattern,
    /// Minimum pilots before switching to tracking
    pub min_pilots_for_tracking: usize,
    /// CMA step size during acquisition
    pub acquisition_step: f64,
    /// CMA step size during tracking
    pub tracking_step: f64,
    /// Enable decision-directed updates
    pub enable_dd: bool,
    /// Error threshold for DD updates (only use if error below this)
    pub dd_threshold: f64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            pilot_pattern: PilotPattern::uniform(16),
            min_pilots_for_tracking: 4,
            acquisition_step: 0.05,
            tracking_step: 0.01,
            enable_dd: true,
            dd_threshold: 0.3,
        }
    }
}

/// Adaptive equalizer for CSPM
pub struct AdaptiveEqualizer {
    config: AdaptiveConfig,
    /// Current operating mode
    mode: EqualizerMode,
    /// Pilot extractor
    pilot_extractor: PilotExtractor,
    /// Channel estimator
    channel_estimator: ChannelEstimator,
    /// CMA equalizer
    cma: CmaEqualizer,
    /// Current channel estimate
    current_channel: ChannelRotation,
    /// Collected pilot measurements
    pilot_buffer: Vec<PilotMeasurement>,
    /// Statistics
    stats: EqualizationStats,
    /// Current position in stream
    position: usize,
}

impl AdaptiveEqualizer {
    /// Create new adaptive equalizer
    pub fn new() -> Self {
        Self::with_config(AdaptiveConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: AdaptiveConfig) -> Self {
        Self {
            pilot_extractor: PilotExtractor::new(config.pilot_pattern.clone()),
            channel_estimator: ChannelEstimator::new(),
            cma: CmaEqualizer::new(),
            current_channel: ChannelRotation::identity(),
            pilot_buffer: Vec::new(),
            stats: EqualizationStats::default(),
            mode: EqualizerMode::Acquisition,
            position: 0,
            config,
        }
    }

    /// Get current mode
    pub fn mode(&self) -> EqualizerMode {
        self.mode
    }

    /// Get current channel estimate
    pub fn current_channel(&self) -> &ChannelRotation {
        &self.current_channel
    }

    /// Get statistics
    pub fn stats(&self) -> &EqualizationStats {
        &self.stats
    }

    /// Process a single received symbol
    pub fn process(&mut self, received: &Quaternion) -> EqualizationResult {
        let is_pilot = self.config.pilot_pattern.is_pilot_position(self.position);

        let result = if is_pilot {
            self.process_pilot(received)
        } else {
            self.process_data(received)
        };

        self.position += 1;
        self.stats.symbols_processed += 1;

        result
    }

    /// Process a batch of symbols
    pub fn process_batch(&mut self, received: &[Quaternion]) -> Vec<EqualizationResult> {
        received.iter().map(|q| self.process(q)).collect()
    }

    /// Decision-directed update after decoding
    pub fn decision_feedback(&mut self, received: &Quaternion, decoded: &Quaternion) {
        if !self.config.enable_dd {
            return;
        }

        // Compute error
        let compensated = self.current_channel.compensate(received);
        let error = compensated.distance(decoded);

        if error < self.config.dd_threshold {
            // Use decoded symbol as reference to refine estimate
            let measurement = PilotMeasurement {
                position: self.position.saturating_sub(1),
                expected: *decoded,
                received: *received,
                vertex_index: 0, // Unknown
            };

            // Soft update of channel estimate
            let new_estimate = measurement.estimate_channel();
            self.current_channel.rotation = self.current_channel.rotation.slerp(
                &new_estimate.rotation,
                0.01 // Small weight
            );
        }
    }

    /// Reset equalizer state
    pub fn reset(&mut self) {
        self.mode = EqualizerMode::Acquisition;
        self.channel_estimator.reset();
        self.cma.reset();
        self.current_channel = ChannelRotation::identity();
        self.pilot_buffer.clear();
        self.stats = EqualizationStats::default();
        self.position = 0;
    }

    /// Process a pilot position
    fn process_pilot(&mut self, received: &Quaternion) -> EqualizationResult {
        // Get expected pilot
        let expected = self.pilot_extractor.expected_pilot(self.position)
            .unwrap_or(Quaternion::new(1.0, 0.0, 0.0, 0.0));

        let measurement = PilotMeasurement {
            position: self.position,
            expected,
            received: *received,
            vertex_index: self.config.pilot_pattern.pilot_index_at(self.position),
        };

        // Add to buffer
        self.pilot_buffer.push(measurement.clone());
        self.stats.pilots_processed += 1;

        // Update channel estimate
        let estimate = self.channel_estimator.update(&measurement);
        self.current_channel = estimate.rotation.clone();

        // Update CMA with pilot information
        self.cma.initialize(&self.current_channel);

        // Mode transition
        if self.mode == EqualizerMode::Acquisition
            && self.pilot_buffer.len() >= self.config.min_pilots_for_tracking
        {
            self.mode = EqualizerMode::PilotTracking;
        }

        // Compute equalized output
        let equalized = self.current_channel.compensate(received);

        EqualizationResult {
            quaternion: equalized,
            estimated_snr_db: self.estimate_snr(&measurement),
            is_pilot: true,
            channel: self.current_channel.clone(),
        }
    }

    /// Process a data position
    fn process_data(&mut self, received: &Quaternion) -> EqualizationResult {
        let equalized = match self.mode {
            EqualizerMode::Acquisition => {
                // During acquisition, just pass through with basic CMA
                self.cma.process(received)
            }
            EqualizerMode::PilotTracking | EqualizerMode::BlindTracking => {
                // Use current channel estimate
                let pilot_equalized = self.current_channel.compensate(received);

                // Also run CMA for tracking
                let cma_equalized = self.cma.process(received);

                // Blend based on confidence
                let blend = self.current_channel.confidence;
                pilot_equalized.slerp(&cma_equalized, 1.0 - blend)
            }
            EqualizerMode::DecisionDirected => {
                // Primarily use channel estimate, CMA for monitoring
                self.current_channel.compensate(received)
            }
        };

        // Update stats
        self.stats.avg_estimation_error =
            0.99 * self.stats.avg_estimation_error + 0.01 * self.cma.convergence_metric();

        EqualizationResult {
            quaternion: equalized,
            estimated_snr_db: self.stats.current_snr_db,
            is_pilot: false,
            channel: self.current_channel.clone(),
        }
    }

    /// Estimate SNR from pilot measurement
    fn estimate_snr(&mut self, pilot: &PilotMeasurement) -> f64 {
        let error = pilot.error_magnitude(&self.current_channel);

        // SNR ≈ 1 / error²
        let snr_linear = 1.0 / (error * error + 1e-10);
        let snr_db = 10.0 * snr_linear.log10();

        // Update running estimate
        self.stats.current_snr_db = 0.9 * self.stats.current_snr_db + 0.1 * snr_db;

        snr_db
    }
}

impl Default for AdaptiveEqualizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame-based equalizer (processes complete frames)
pub struct FrameEqualizer {
    config: AdaptiveConfig,
}

impl FrameEqualizer {
    /// Create new frame equalizer
    pub fn new(config: AdaptiveConfig) -> Self {
        Self { config }
    }

    /// Equalize a complete frame
    pub fn equalize_frame(&self, frame: &[Quaternion]) -> Vec<EqualizationResult> {
        let extractor = PilotExtractor::new(self.config.pilot_pattern.clone());
        let extraction = extractor.extract(frame);

        // Estimate channel from all pilots
        let mut estimator = ChannelEstimator::new();
        let estimate = estimator.estimate(&extraction.pilots);

        // Build interpolator for smooth channel tracking
        let pilot_estimates: Vec<(usize, ChannelRotation)> = extraction.pilots.iter()
            .map(|p| (p.position, p.estimate_channel()))
            .collect();
        let interpolator = ChannelInterpolator::new(pilot_estimates);

        // Equalize all symbols
        frame.iter().enumerate().map(|(pos, q)| {
            let is_pilot = self.config.pilot_pattern.is_pilot_position(pos);
            let channel = interpolator.at(pos);
            let equalized = channel.compensate(q);

            EqualizationResult {
                quaternion: equalized,
                estimated_snr_db: 10.0 * (1.0 / estimate.residual_error.powi(2)).log10(),
                is_pilot,
                channel,
            }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polytope::Hexacosichoron;

    fn create_test_frame(
        hex: &Hexacosichoron,
        pattern: &PilotPattern,
        channel: &Quaternion,
        length: usize,
    ) -> Vec<Quaternion> {
        (0..length).map(|pos| {
            let base = if pattern.is_pilot_position(pos) {
                hex.vertices()[pattern.pilot_index_at(pos)].q
            } else {
                hex.vertices()[(pos * 7) % 120].q
            };
            let distorted = *channel * base * channel.conjugate();
            distorted.normalize()
        }).collect()
    }

    #[test]
    fn test_adaptive_equalizer_acquisition() {
        let hex = Hexacosichoron::new();
        let config = AdaptiveConfig::default();
        let mut eq = AdaptiveEqualizer::with_config(config.clone());

        // Create frame with channel rotation
        let channel = Quaternion::from_axis_angle([0.0, 0.0, 1.0], 0.3);
        let frame = create_test_frame(&hex, &config.pilot_pattern, &channel, 100);

        // Process frame
        for q in &frame {
            let result = eq.process(q);
            assert!(result.quaternion.is_normalized());
        }

        // Should have transitioned from acquisition
        assert_ne!(eq.mode(), EqualizerMode::Acquisition);
    }

    #[test]
    fn test_frame_equalizer() {
        let hex = Hexacosichoron::new();
        let config = AdaptiveConfig {
            pilot_pattern: PilotPattern::uniform(8),
            ..Default::default()
        };

        let frame_eq = FrameEqualizer::new(config.clone());

        // Create frame
        let channel = Quaternion::from_axis_angle([1.0, 0.0, 0.0], 0.2);
        let frame = create_test_frame(&hex, &config.pilot_pattern, &channel, 64);

        // Equalize
        let results = frame_eq.equalize_frame(&frame);
        assert_eq!(results.len(), frame.len());

        // Check pilot positions are marked
        let pilot_count = results.iter().filter(|r| r.is_pilot).count();
        assert!(pilot_count > 0);
    }

    #[test]
    fn test_equalization_improves_signal() {
        let hex = Hexacosichoron::new();
        let config = AdaptiveConfig {
            pilot_pattern: PilotPattern::uniform(8),
            ..Default::default()
        };

        let frame_eq = FrameEqualizer::new(config.clone());

        // Create frame with channel
        let channel = Quaternion::from_axis_angle([0.0, 1.0, 0.0], 0.25);
        let frame = create_test_frame(&hex, &config.pilot_pattern, &channel, 64);

        // Get original symbols
        let originals: Vec<Quaternion> = (0..64).map(|pos| {
            if config.pilot_pattern.is_pilot_position(pos) {
                hex.vertices()[config.pilot_pattern.pilot_index_at(pos)].q
            } else {
                hex.vertices()[(pos * 7) % 120].q
            }
        }).collect();

        // Equalize
        let results = frame_eq.equalize_frame(&frame);

        // Compute average error before and after
        let error_before: f64 = frame.iter().zip(&originals)
            .map(|(r, o)| r.distance(o))
            .sum::<f64>() / frame.len() as f64;

        let error_after: f64 = results.iter().zip(&originals)
            .map(|(r, o)| r.quaternion.distance(o))
            .sum::<f64>() / results.len() as f64;

        // Equalization should reduce error
        assert!(error_after < error_before,
            "Error after ({}) should be less than before ({})",
            error_after, error_before);
    }
}

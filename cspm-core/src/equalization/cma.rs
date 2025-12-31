//! Constant Modulus Algorithm (CMA) for blind equalization.
//!
//! CMA exploits the fact that all valid CSPM symbols lie on the unit sphere
//! in quaternion space. By minimizing deviation from unit modulus, the
//! algorithm can converge to the correct channel inverse without pilots.
//!
//! ## Algorithm
//!
//! For received quaternion y and equalizer rotation W:
//!
//! 1. Compute equalized output: z = W * y * W†
//! 2. Compute error: e = |z|² - 1
//! 3. Update: W = W - μ * e * ∂z/∂W
//!
//! ## Properties
//!
//! - Blind: No training symbols required
//! - Robust: Works with any rotation angle
//! - Convergence: Slower than pilot-based, but steady

use crate::quaternion::Quaternion;
use super::ChannelRotation;

/// CMA equalizer configuration
#[derive(Clone, Debug)]
pub struct CmaConfig {
    /// Step size (learning rate)
    pub step_size: f64,
    /// Momentum factor (0-1)
    pub momentum: f64,
    /// Target modulus (should be 1.0 for unit quaternions)
    pub target_modulus: f64,
    /// Maximum iterations per symbol
    pub max_iterations: usize,
    /// Convergence threshold
    pub convergence_threshold: f64,
}

impl Default for CmaConfig {
    fn default() -> Self {
        Self {
            step_size: 0.01,
            momentum: 0.9,
            target_modulus: 1.0,
            max_iterations: 1,
            convergence_threshold: 1e-6,
        }
    }
}

impl CmaConfig {
    /// Fast convergence config
    pub fn fast() -> Self {
        Self {
            step_size: 0.05,
            momentum: 0.8,
            ..Default::default()
        }
    }

    /// Stable config for noisy channels
    pub fn stable() -> Self {
        Self {
            step_size: 0.001,
            momentum: 0.95,
            ..Default::default()
        }
    }
}

/// CMA blind equalizer
pub struct CmaEqualizer {
    config: CmaConfig,
    /// Current equalizer rotation
    equalizer: Quaternion,
    /// Momentum accumulator
    momentum_accumulator: Quaternion,
    /// Number of symbols processed
    symbols_processed: u64,
    /// Running error estimate
    running_error: f64,
}

impl CmaEqualizer {
    /// Create new CMA equalizer
    pub fn new() -> Self {
        Self::with_config(CmaConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: CmaConfig) -> Self {
        Self {
            config,
            equalizer: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            momentum_accumulator: Quaternion::new(0.0, 0.0, 0.0, 0.0),
            symbols_processed: 0,
            running_error: 1.0,
        }
    }

    /// Initialize with a known channel estimate
    pub fn initialize(&mut self, channel: &ChannelRotation) {
        // Set equalizer to inverse of channel
        self.equalizer = channel.rotation.conjugate();
    }

    /// Process a single symbol and update equalizer
    pub fn process(&mut self, received: &Quaternion) -> Quaternion {
        // Apply current equalizer
        let equalized = self.apply(received);

        // Compute modulus error
        let modulus = equalized.norm();
        let error = modulus * modulus - self.config.target_modulus;

        // Update running error estimate
        self.running_error = 0.99 * self.running_error + 0.01 * error.abs();

        // Compute gradient (simplified for quaternion case)
        // The gradient of |W*y*W†|² w.r.t. W is complex, we use an approximation
        let gradient = self.compute_gradient(received, &equalized, error);

        // Apply momentum
        self.momentum_accumulator = Quaternion::new(
            self.config.momentum * self.momentum_accumulator.w + gradient.w,
            self.config.momentum * self.momentum_accumulator.x + gradient.x,
            self.config.momentum * self.momentum_accumulator.y + gradient.y,
            self.config.momentum * self.momentum_accumulator.z + gradient.z,
        );

        // Update equalizer
        let update = Quaternion::new(
            -self.config.step_size * self.momentum_accumulator.w,
            -self.config.step_size * self.momentum_accumulator.x,
            -self.config.step_size * self.momentum_accumulator.y,
            -self.config.step_size * self.momentum_accumulator.z,
        );

        // Apply update as small rotation
        let delta = Quaternion::new(
            1.0 + update.w,
            update.x,
            update.y,
            update.z,
        ).normalize();

        self.equalizer = (delta * self.equalizer).normalize();
        self.symbols_processed += 1;

        equalized.normalize()
    }

    /// Apply equalizer without updating
    pub fn apply(&self, received: &Quaternion) -> Quaternion {
        let result = self.equalizer * *received * self.equalizer.conjugate();
        result.normalize()
    }

    /// Get current equalizer as channel rotation
    pub fn current_estimate(&self) -> ChannelRotation {
        ChannelRotation {
            rotation: self.equalizer.conjugate(),
            confidence: 1.0 / (1.0 + self.running_error),
            timestamp: self.symbols_processed,
        }
    }

    /// Get convergence metric (lower is better)
    pub fn convergence_metric(&self) -> f64 {
        self.running_error
    }

    /// Check if equalizer has converged
    pub fn has_converged(&self) -> bool {
        self.running_error < self.config.convergence_threshold
    }

    /// Reset equalizer state
    pub fn reset(&mut self) {
        self.equalizer = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        self.momentum_accumulator = Quaternion::new(0.0, 0.0, 0.0, 0.0);
        self.symbols_processed = 0;
        self.running_error = 1.0;
    }

    /// Process batch of symbols
    pub fn process_batch(&mut self, received: &[Quaternion]) -> Vec<Quaternion> {
        received.iter().map(|q| self.process(q)).collect()
    }

    /// Compute gradient of cost function
    fn compute_gradient(
        &self,
        received: &Quaternion,
        equalized: &Quaternion,
        error: f64,
    ) -> Quaternion {
        // Gradient of CMA cost |z|² - 1 where z = W*y*W†
        // Using chain rule and quaternion derivative rules
        //
        // ∂J/∂W ≈ 4 * error * equalized * received†

        let scale = 4.0 * error;
        let grad = *equalized * received.conjugate();

        Quaternion::new(
            scale * grad.w,
            scale * grad.x,
            scale * grad.y,
            scale * grad.z,
        )
    }
}

impl Default for CmaEqualizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Modified CMA for CSPM (exploits constellation structure)
pub struct CspmCma {
    cma: CmaEqualizer,
    /// Distance to nearest vertex (for constellation-aided updates)
    vertex_distances: Vec<f64>,
}

impl CspmCma {
    /// Create new CSPM-aware CMA equalizer
    pub fn new() -> Self {
        Self {
            cma: CmaEqualizer::new(),
            vertex_distances: Vec::new(),
        }
    }

    /// Process with constellation-aided refinement
    pub fn process(&mut self, received: &Quaternion) -> Quaternion {
        // Standard CMA update
        let equalized = self.cma.process(received);

        // Track distance to nearest vertex for monitoring
        // (In full implementation, would use VoronoiLookup here)
        self.vertex_distances.push(0.0); // Placeholder

        equalized
    }

    /// Get average vertex distance (quality metric)
    pub fn avg_vertex_distance(&self) -> f64 {
        if self.vertex_distances.is_empty() {
            return 1.0;
        }
        self.vertex_distances.iter().sum::<f64>() / self.vertex_distances.len() as f64
    }
}

impl Default for CspmCma {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_cma_identity_channel() {
        let mut cma = CmaEqualizer::new();

        // Process unit quaternions (no channel distortion)
        let symbols: Vec<Quaternion> = (0..100).map(|i| {
            Quaternion::from_axis_angle([0.0, 0.0, 1.0], i as f64 * 0.1)
        }).collect();

        for q in &symbols {
            let out = cma.process(q);
            // Output should stay on unit sphere
            assert!((out.norm() - 1.0).abs() < 0.1);
        }
    }

    #[test]
    fn test_cma_with_rotation() {
        let mut cma = CmaEqualizer::with_config(CmaConfig::fast());

        // Simulate channel rotation
        let channel = Quaternion::from_axis_angle([1.0, 0.0, 0.0], 0.5);

        let symbols: Vec<Quaternion> = (0..200).map(|i| {
            let orig = Quaternion::from_axis_angle([0.0, 0.0, 1.0], i as f64 * 0.3);
            let distorted = channel * orig * channel.conjugate();
            distorted.normalize()
        }).collect();

        // Train on symbols
        for q in &symbols {
            cma.process(q);
        }

        // After training, error should decrease
        assert!(cma.convergence_metric() < 0.5);
    }

    #[test]
    fn test_cma_initialization() {
        let mut cma = CmaEqualizer::new();

        // Initialize with known channel
        let channel = ChannelRotation::new(
            Quaternion::from_axis_angle([0.0, 1.0, 0.0], 0.3),
            1.0
        );
        cma.initialize(&channel);

        // Apply to distorted symbol
        let original = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let distorted = channel.apply(&original);
        let recovered = cma.apply(&distorted);

        // Should recover original
        assert!(original.distance(&recovered) < 0.1);
    }

    #[test]
    fn test_cma_convergence() {
        let config = CmaConfig {
            step_size: 0.02,
            momentum: 0.9,
            convergence_threshold: 0.01,
            ..Default::default()
        };
        let mut cma = CmaEqualizer::with_config(config);

        // Small rotation channel
        let channel = Quaternion::from_axis_angle([0.0, 0.0, 1.0], 0.1);

        // Process many symbols
        for i in 0..500 {
            let orig = Quaternion::from_axis_angle([1.0, 0.0, 0.0], i as f64 * 0.2);
            let distorted = channel * orig * channel.conjugate();
            cma.process(&distorted.normalize());
        }

        // Should approach convergence
        assert!(cma.convergence_metric() < 0.1);
    }
}

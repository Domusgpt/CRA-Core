//! Channel estimation algorithms.
//!
//! Estimates the channel rotation from pilot measurements using various methods:
//! - Least squares averaging
//! - Weighted least squares (by SNR)
//! - Kalman filtering for time-varying channels

use crate::quaternion::Quaternion;
use super::{ChannelRotation, pilot::PilotMeasurement};

/// Configuration for channel estimator
#[derive(Clone, Debug)]
pub struct EstimatorConfig {
    /// Number of pilots to average
    pub averaging_window: usize,
    /// Exponential forgetting factor (0-1, higher = more weight on recent)
    pub forgetting_factor: f64,
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Enable interpolation between pilots
    pub enable_interpolation: bool,
}

impl Default for EstimatorConfig {
    fn default() -> Self {
        Self {
            averaging_window: 4,
            forgetting_factor: 0.9,
            min_confidence: 0.5,
            enable_interpolation: true,
        }
    }
}

/// Channel estimate with metadata
#[derive(Clone, Debug)]
pub struct ChannelEstimate {
    /// Primary rotation estimate
    pub rotation: ChannelRotation,
    /// Rate of change (radians per symbol)
    pub rotation_rate: Option<Quaternion>,
    /// Noise variance estimate
    pub noise_variance: f64,
    /// Number of pilots used
    pub pilots_used: usize,
    /// Residual error after fitting
    pub residual_error: f64,
}

impl ChannelEstimate {
    /// Predict channel at future position
    pub fn predict(&self, symbols_ahead: usize) -> ChannelRotation {
        if let Some(rate) = &self.rotation_rate {
            // Extrapolate using rotation rate
            let delta = Quaternion::new(
                1.0 + rate.w * symbols_ahead as f64,
                rate.x * symbols_ahead as f64,
                rate.y * symbols_ahead as f64,
                rate.z * symbols_ahead as f64,
            ).normalize();

            ChannelRotation {
                rotation: (delta * self.rotation.rotation).normalize(),
                confidence: self.rotation.confidence * 0.99_f64.powi(symbols_ahead as i32),
                timestamp: self.rotation.timestamp + symbols_ahead as u64,
            }
        } else {
            // Static channel assumption
            ChannelRotation {
                rotation: self.rotation.rotation,
                confidence: self.rotation.confidence * 0.995_f64.powi(symbols_ahead as i32),
                timestamp: self.rotation.timestamp + symbols_ahead as u64,
            }
        }
    }

    /// Get channel at specific position using interpolation
    pub fn at_position(&self, position: usize) -> ChannelRotation {
        let delta = position as i64 - self.rotation.timestamp as i64;
        if delta >= 0 {
            self.predict(delta as usize)
        } else {
            // Backwards prediction (less accurate)
            self.predict(0)
        }
    }
}

/// Channel estimator using pilot symbols
pub struct ChannelEstimator {
    config: EstimatorConfig,
    /// History of channel estimates for tracking
    history: Vec<ChannelRotation>,
    /// Current best estimate
    current_estimate: Option<ChannelEstimate>,
}

impl ChannelEstimator {
    /// Create new estimator with default config
    pub fn new() -> Self {
        Self::with_config(EstimatorConfig::default())
    }

    /// Create estimator with custom config
    pub fn with_config(config: EstimatorConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
            current_estimate: None,
        }
    }

    /// Least-squares estimator
    pub fn least_squares() -> Self {
        Self::with_config(EstimatorConfig {
            averaging_window: 8,
            forgetting_factor: 1.0, // No forgetting
            ..Default::default()
        })
    }

    /// Fast-tracking estimator for dynamic channels
    pub fn fast_tracking() -> Self {
        Self::with_config(EstimatorConfig {
            averaging_window: 2,
            forgetting_factor: 0.8,
            ..Default::default()
        })
    }

    /// Estimate channel from pilot measurements
    pub fn estimate(&mut self, pilots: &[PilotMeasurement]) -> ChannelEstimate {
        if pilots.is_empty() {
            return self.current_estimate.clone().unwrap_or_else(|| {
                ChannelEstimate {
                    rotation: ChannelRotation::identity(),
                    rotation_rate: None,
                    noise_variance: 1.0,
                    pilots_used: 0,
                    residual_error: 1.0,
                }
            });
        }

        // Get individual channel estimates from each pilot
        let estimates: Vec<ChannelRotation> = pilots.iter()
            .map(|p| p.estimate_channel())
            .collect();

        // Weighted average using exponential forgetting
        let rotation = self.weighted_average(&estimates);

        // Estimate rotation rate from consecutive pilots
        let rotation_rate = if pilots.len() >= 2 {
            self.estimate_rotation_rate(&estimates)
        } else {
            None
        };

        // Estimate noise variance from residuals
        let (noise_variance, residual_error) = self.compute_residuals(pilots, &rotation);

        // Update history
        self.history.push(rotation.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        let estimate = ChannelEstimate {
            rotation,
            rotation_rate,
            noise_variance,
            pilots_used: pilots.len(),
            residual_error,
        };

        self.current_estimate = Some(estimate.clone());
        estimate
    }

    /// Update estimate with new pilot (incremental update)
    pub fn update(&mut self, pilot: &PilotMeasurement) -> ChannelEstimate {
        let new_estimate = pilot.estimate_channel();

        if let Some(ref mut current) = self.current_estimate {
            // Exponential moving average update
            let alpha = self.config.forgetting_factor;
            current.rotation.rotation = current.rotation.rotation.slerp(
                &new_estimate.rotation,
                1.0 - alpha
            );
            current.rotation.confidence = current.rotation.confidence * alpha
                + new_estimate.confidence * (1.0 - alpha);
            current.rotation.timestamp = pilot.position as u64;
            current.pilots_used += 1;

            current.clone()
        } else {
            self.estimate(&[pilot.clone()])
        }
    }

    /// Get current estimate
    pub fn current(&self) -> Option<&ChannelEstimate> {
        self.current_estimate.as_ref()
    }

    /// Reset estimator state
    pub fn reset(&mut self) {
        self.history.clear();
        self.current_estimate = None;
    }

    /// Weighted average of quaternions using SLERP
    fn weighted_average(&self, estimates: &[ChannelRotation]) -> ChannelRotation {
        if estimates.is_empty() {
            return ChannelRotation::identity();
        }

        if estimates.len() == 1 {
            return estimates[0].clone();
        }

        // Compute weights based on confidence and recency
        let mut weights: Vec<f64> = estimates.iter().enumerate().map(|(i, e)| {
            let recency = self.config.forgetting_factor.powi((estimates.len() - 1 - i) as i32);
            e.confidence * recency
        }).collect();

        // Normalize weights
        let sum: f64 = weights.iter().sum();
        if sum > 0.0 {
            for w in &mut weights {
                *w /= sum;
            }
        }

        // Iterative weighted mean using SLERP
        let mut result = estimates[0].rotation;
        let mut cumulative_weight = weights[0];

        for i in 1..estimates.len() {
            cumulative_weight += weights[i];
            if cumulative_weight > 0.0 {
                let t = weights[i] / cumulative_weight;
                result = result.slerp(&estimates[i].rotation, t);
            }
        }

        let avg_confidence: f64 = estimates.iter().zip(&weights)
            .map(|(e, w)| e.confidence * w)
            .sum();

        let latest_timestamp = estimates.iter()
            .map(|e| e.timestamp)
            .max()
            .unwrap_or(0);

        ChannelRotation {
            rotation: result.normalize(),
            confidence: avg_confidence,
            timestamp: latest_timestamp,
        }
    }

    /// Estimate rotation rate from consecutive estimates
    fn estimate_rotation_rate(&self, estimates: &[ChannelRotation]) -> Option<Quaternion> {
        if estimates.len() < 2 {
            return None;
        }

        // Compute rotation differences
        let mut diffs = Vec::new();
        for i in 1..estimates.len() {
            if estimates[i].timestamp > estimates[i-1].timestamp {
                let dt = (estimates[i].timestamp - estimates[i-1].timestamp) as f64;
                let diff = estimates[i].rotation * estimates[i-1].rotation.conjugate();

                // Normalize to rate per symbol
                let rate = Quaternion::new(
                    (diff.w - 1.0) / dt,
                    diff.x / dt,
                    diff.y / dt,
                    diff.z / dt,
                );
                diffs.push(rate);
            }
        }

        if diffs.is_empty() {
            return None;
        }

        // Average the rates
        let n = diffs.len() as f64;
        let avg = Quaternion::new(
            diffs.iter().map(|d| d.w).sum::<f64>() / n,
            diffs.iter().map(|d| d.x).sum::<f64>() / n,
            diffs.iter().map(|d| d.y).sum::<f64>() / n,
            diffs.iter().map(|d| d.z).sum::<f64>() / n,
        );

        Some(avg)
    }

    /// Compute residual error after channel compensation
    fn compute_residuals(
        &self,
        pilots: &[PilotMeasurement],
        channel: &ChannelRotation,
    ) -> (f64, f64) {
        if pilots.is_empty() {
            return (1.0, 1.0);
        }

        let mut total_error = 0.0;
        for pilot in pilots {
            let compensated = channel.compensate(&pilot.received);
            let error = pilot.expected.distance(&compensated);
            total_error += error * error;
        }

        let mse = total_error / pilots.len() as f64;
        let rmse = mse.sqrt();

        // Noise variance estimate (assuming errors are Gaussian)
        let noise_variance = mse / 4.0; // 4 components in quaternion

        (noise_variance, rmse)
    }
}

impl Default for ChannelEstimator {
    fn default() -> Self {
        Self::new()
    }
}

/// Interpolates channel estimates between pilot positions
pub struct ChannelInterpolator {
    estimates: Vec<(usize, ChannelRotation)>,
}

impl ChannelInterpolator {
    /// Create new interpolator from estimates at pilot positions
    pub fn new(estimates: Vec<(usize, ChannelRotation)>) -> Self {
        let mut sorted = estimates;
        sorted.sort_by_key(|(pos, _)| *pos);
        Self { estimates: sorted }
    }

    /// Get interpolated channel at any position
    pub fn at(&self, position: usize) -> ChannelRotation {
        if self.estimates.is_empty() {
            return ChannelRotation::identity();
        }

        if self.estimates.len() == 1 {
            return self.estimates[0].1.clone();
        }

        // Find bracketing estimates
        let mut lower = &self.estimates[0];
        let mut upper = &self.estimates[self.estimates.len() - 1];

        for (i, (pos, _)) in self.estimates.iter().enumerate() {
            if *pos <= position {
                lower = &self.estimates[i];
            }
            if *pos >= position && i > 0 {
                upper = &self.estimates[i];
                break;
            }
        }

        if lower.0 == upper.0 {
            return lower.1.clone();
        }

        // Linear interpolation parameter
        let t = (position - lower.0) as f64 / (upper.0 - lower.0) as f64;
        lower.1.interpolate(&upper.1, t.clamp(0.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polytope::Hexacosichoron;

    #[test]
    fn test_estimator_identity_channel() {
        let hex = Hexacosichoron::new();
        let mut estimator = ChannelEstimator::new();

        // Create pilots with no channel distortion
        let pilots: Vec<PilotMeasurement> = (0..4).map(|i| {
            let vertex = &hex.vertices()[i * 10];
            PilotMeasurement {
                position: i * 16,
                expected: vertex.q,
                received: vertex.q,
                vertex_index: i * 10,
            }
        }).collect();

        let estimate = estimator.estimate(&pilots);

        // Should estimate identity channel
        let identity = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        assert!(estimate.rotation.rotation.distance(&identity) < 0.1);
    }

    #[test]
    fn test_estimator_with_rotation() {
        let hex = Hexacosichoron::new();
        let mut estimator = ChannelEstimator::new();

        // Simulate a channel rotation
        let channel = Quaternion::from_axis_angle([0.0, 1.0, 0.0], 0.5);

        let pilots: Vec<PilotMeasurement> = (0..4).map(|i| {
            let vertex = &hex.vertices()[i * 10];
            let distorted = channel * vertex.q * channel.conjugate();
            PilotMeasurement {
                position: i * 16,
                expected: vertex.q,
                received: distorted.normalize(),
                vertex_index: i * 10,
            }
        }).collect();

        let estimate = estimator.estimate(&pilots);

        // Test compensation
        let test_q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let distorted = channel * test_q * channel.conjugate();
        let recovered = estimate.rotation.compensate(&distorted.normalize());

        assert!(test_q.distance(&recovered) < 0.2);
    }

    #[test]
    fn test_interpolator() {
        let estimates = vec![
            (0, ChannelRotation::new(Quaternion::new(1.0, 0.0, 0.0, 0.0), 1.0)),
            (100, ChannelRotation::new(
                Quaternion::from_axis_angle([0.0, 0.0, 1.0], 1.0),
                1.0
            )),
        ];

        let interp = ChannelInterpolator::new(estimates);

        // At position 50, should be halfway between
        let mid = interp.at(50);
        let expected = Quaternion::from_axis_angle([0.0, 0.0, 1.0], 0.5);
        assert!(mid.rotation.distance(&expected) < 0.1);
    }

    #[test]
    fn test_prediction() {
        let estimate = ChannelEstimate {
            rotation: ChannelRotation::new(
                Quaternion::from_axis_angle([0.0, 0.0, 1.0], 0.1),
                1.0
            ),
            rotation_rate: Some(Quaternion::new(0.0, 0.0, 0.0, 0.01)),
            noise_variance: 0.01,
            pilots_used: 4,
            residual_error: 0.01,
        };

        let predicted = estimate.predict(10);

        // Confidence should decrease with prediction distance
        assert!(predicted.confidence < estimate.rotation.confidence);
    }
}

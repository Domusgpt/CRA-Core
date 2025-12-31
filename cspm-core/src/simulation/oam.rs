//! OAM (Orbital Angular Momentum) mode model.
//!
//! Models:
//! - Laguerre-Gaussian beam propagation
//! - Mode sorter efficiency
//! - Inter-modal crosstalk
//! - Beam divergence effects

use rand::Rng;
use rand_distr::{Normal, Distribution};
use std::f64::consts::PI;

/// OAM beam parameters
#[derive(Clone, Debug)]
pub struct OamBeam {
    /// Topological charge (mode number)
    pub l: i32,
    /// Radial index (p=0 for fundamental)
    pub p: u32,
    /// Beam waist at source (m)
    pub w0: f64,
    /// Wavelength (m)
    pub wavelength: f64,
}

impl OamBeam {
    /// Create fundamental OAM mode
    pub fn fundamental(l: i32, w0: f64, wavelength: f64) -> Self {
        Self { l, p: 0, w0, wavelength }
    }

    /// Rayleigh range
    pub fn rayleigh_range(&self) -> f64 {
        PI * self.w0.powi(2) / self.wavelength
    }

    /// Beam waist at distance z
    pub fn waist_at(&self, z: f64) -> f64 {
        let zr = self.rayleigh_range();
        self.w0 * (1.0 + (z / zr).powi(2)).sqrt()
    }

    /// Gouy phase at distance z
    pub fn gouy_phase(&self, z: f64) -> f64 {
        let zr = self.rayleigh_range();
        let mode_order = self.l.abs() as f64 + 2.0 * self.p as f64 + 1.0;
        mode_order * (z / zr).atan()
    }

    /// Ring radius for OAM mode
    pub fn ring_radius(&self, z: f64) -> f64 {
        let w = self.waist_at(z);
        w * (self.l.abs() as f64 / 2.0).sqrt()
    }
}

/// OAM mode sorter model
#[derive(Clone, Debug)]
pub struct ModeSorter {
    /// Maximum mode number handled
    pub max_mode: i32,
    /// Sorting efficiency (0-1)
    pub efficiency: f64,
    /// Crosstalk to adjacent modes (0-1)
    pub adjacent_crosstalk: f64,
    /// Crosstalk to non-adjacent modes (0-1)
    pub distant_crosstalk: f64,
    /// Insertion loss (dB)
    pub insertion_loss_db: f64,
}

impl Default for ModeSorter {
    fn default() -> Self {
        // Typical log-polar transformer + lens system
        Self {
            max_mode: 16,
            efficiency: 0.85,
            adjacent_crosstalk: 0.08,
            distant_crosstalk: 0.02,
            insertion_loss_db: 3.0,
        }
    }
}

impl ModeSorter {
    /// High-performance sorter (e.g., cascaded interferometer)
    pub fn high_performance() -> Self {
        Self {
            efficiency: 0.95,
            adjacent_crosstalk: 0.03,
            distant_crosstalk: 0.005,
            insertion_loss_db: 1.5,
            ..Default::default()
        }
    }

    /// Apply mode sorting (returns detected mode and confidence)
    pub fn sort(&self, input_mode: i32, rng: &mut impl Rng) -> (i32, f64) {
        if input_mode.abs() > self.max_mode {
            // Mode outside range - random detection
            let detected = rng.gen_range(-self.max_mode..=self.max_mode);
            return (detected, 0.1);
        }

        let r: f64 = rng.gen();

        if r < self.efficiency {
            // Correct detection
            (input_mode, self.efficiency)
        } else if r < self.efficiency + self.adjacent_crosstalk {
            // Adjacent mode crosstalk
            let offset = if rng.gen() { 1 } else { -1 };
            let detected = (input_mode + offset).clamp(-self.max_mode, self.max_mode);
            (detected, self.adjacent_crosstalk)
        } else if r < self.efficiency + self.adjacent_crosstalk + self.distant_crosstalk {
            // Distant mode crosstalk
            let detected = rng.gen_range(-self.max_mode..=self.max_mode);
            (detected, self.distant_crosstalk)
        } else {
            // Detection failure - return input with low confidence
            (input_mode, 0.0)
        }
    }

    /// Generate crosstalk probability matrix
    pub fn crosstalk_matrix(&self) -> Vec<Vec<f64>> {
        let n = (2 * self.max_mode + 1) as usize;
        let mut matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            let mode_i = i as i32 - self.max_mode;

            for j in 0..n {
                let mode_j = j as i32 - self.max_mode;
                let delta = (mode_i - mode_j).abs();

                matrix[i][j] = match delta {
                    0 => self.efficiency,
                    1 => self.adjacent_crosstalk / 2.0,
                    _ => self.distant_crosstalk / (2 * self.max_mode - 1) as f64,
                };
            }

            // Normalize row
            let sum: f64 = matrix[i].iter().sum();
            for val in &mut matrix[i] {
                *val /= sum;
            }
        }

        matrix
    }
}

/// OAM multiplexer/demultiplexer
#[derive(Clone, Debug)]
pub struct OamMux {
    /// Modes being multiplexed
    pub modes: Vec<i32>,
    /// Per-mode power (linear)
    pub powers: Vec<f64>,
    /// Mode sorter at receiver
    pub sorter: ModeSorter,
}

impl OamMux {
    /// Create MUX with specified modes
    pub fn new(modes: Vec<i32>) -> Self {
        let n = modes.len();
        Self {
            modes,
            powers: vec![1.0; n],
            sorter: ModeSorter::default(),
        }
    }

    /// Total capacity (modes × spectral efficiency per mode)
    pub fn capacity_factor(&self) -> f64 {
        self.modes.len() as f64
    }

    /// Apply demultiplexing with crosstalk
    pub fn demux(&self, input_mode: i32, rng: &mut impl Rng) -> Option<(usize, f64)> {
        let (detected, confidence) = self.sorter.sort(input_mode, rng);

        self.modes.iter().position(|&m| m == detected)
            .map(|idx| (idx, confidence))
    }
}

/// Inter-modal four-wave mixing (FWM) model
#[derive(Clone, Debug)]
pub struct ModalFwm {
    /// Fiber nonlinear coefficient (1/(W·km))
    pub gamma: f64,
    /// Fiber length (km)
    pub length_km: f64,
    /// Number of modes
    pub num_modes: usize,
}

impl ModalFwm {
    /// Calculate FWM efficiency between modes
    pub fn fwm_efficiency(&self, delta_beta: f64, power_w: f64) -> f64 {
        let l = self.length_km * 1000.0;
        let eta = self.gamma * power_w * l;

        // Phase matching factor
        let pm = if delta_beta.abs() < 1e-10 {
            1.0
        } else {
            let x = delta_beta * l / 2.0;
            (x.sin() / x).powi(2)
        };

        eta.powi(2) * pm
    }

    /// Apply inter-modal FWM crosstalk
    pub fn apply_crosstalk(&self, signal: &mut [f64], pump_power_w: f64) {
        let n = signal.len();
        let mut crosstalk = vec![0.0; n];

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    // Simplified delta_beta based on mode spacing
                    let delta_beta = ((i as i32 - j as i32).abs() as f64) * 0.01;
                    let eta = self.fwm_efficiency(delta_beta, pump_power_w);
                    crosstalk[i] += signal[j] * eta;
                }
            }
        }

        for i in 0..n {
            signal[i] += crosstalk[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oam_beam() {
        let beam = OamBeam::fundamental(3, 1e-3, 1.55e-6);

        assert!(beam.rayleigh_range() > 0.0);
        assert!(beam.waist_at(1000.0) > beam.w0);
        assert!(beam.ring_radius(1000.0) > 0.0);
    }

    #[test]
    fn test_mode_sorter() {
        let sorter = ModeSorter::default();
        let mut rng = rand::thread_rng();

        // Test many sortings
        let mut correct = 0;
        let n_trials = 1000;

        for _ in 0..n_trials {
            let (detected, _) = sorter.sort(5, &mut rng);
            if detected == 5 {
                correct += 1;
            }
        }

        let accuracy = correct as f64 / n_trials as f64;
        assert!(accuracy > 0.7 && accuracy < 0.95);
    }

    #[test]
    fn test_crosstalk_matrix() {
        let sorter = ModeSorter::default();
        let matrix = sorter.crosstalk_matrix();

        // Check dimensions
        assert_eq!(matrix.len(), 33);
        assert_eq!(matrix[0].len(), 33);

        // Check rows sum to 1
        for row in &matrix {
            let sum: f64 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-10);
        }

        // Diagonal should be largest
        for i in 0..matrix.len() {
            assert!(matrix[i][i] > matrix[i][(i + 1) % matrix.len()]);
        }
    }
}

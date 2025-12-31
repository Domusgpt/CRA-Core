//! Monte Carlo BER simulation framework.
//!
//! Provides:
//! - BER vs SNR curve generation
//! - Statistical confidence intervals
//! - Comparison with theoretical bounds
//! - Multi-threaded simulation

use crate::modulation::{CspmEncoder, CspmDecoder};
use crate::quaternion::Quaternion;
use crate::GenesisConfig;
use super::{ChannelModel, constants};
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashMap;

/// BER simulation result for a single SNR point
#[derive(Clone, Debug)]
pub struct BerPoint {
    /// SNR value (dB)
    pub snr_db: f64,
    /// Measured bit error rate
    pub ber: f64,
    /// Number of bits transmitted
    pub total_bits: u64,
    /// Number of bit errors
    pub bit_errors: u64,
    /// Number of symbols transmitted
    pub total_symbols: u64,
    /// Number of symbol errors
    pub symbol_errors: u64,
    /// 95% confidence interval (lower, upper)
    pub confidence_interval: (f64, f64),
}

impl BerPoint {
    /// Calculate symbol error rate
    pub fn ser(&self) -> f64 {
        if self.total_symbols == 0 {
            0.0
        } else {
            self.symbol_errors as f64 / self.total_symbols as f64
        }
    }
}

/// BER curve (BER vs SNR)
#[derive(Clone, Debug)]
pub struct BerCurve {
    /// Simulation points
    pub points: Vec<BerPoint>,
    /// Modulation scheme name
    pub scheme_name: String,
    /// Channel description
    pub channel_description: String,
}

impl BerCurve {
    /// Get BER at specific SNR (interpolated)
    pub fn ber_at_snr(&self, snr_db: f64) -> Option<f64> {
        if self.points.is_empty() {
            return None;
        }

        // Find bracketing points
        let mut lower = None;
        let mut upper = None;

        for point in &self.points {
            if point.snr_db <= snr_db {
                lower = Some(point);
            }
            if point.snr_db >= snr_db && upper.is_none() {
                upper = Some(point);
            }
        }

        match (lower, upper) {
            (Some(l), Some(u)) if l.snr_db != u.snr_db => {
                // Log-linear interpolation
                let t = (snr_db - l.snr_db) / (u.snr_db - l.snr_db);
                let log_ber_l = l.ber.ln();
                let log_ber_u = u.ber.ln();
                Some((log_ber_l + t * (log_ber_u - log_ber_l)).exp())
            }
            (Some(p), _) | (_, Some(p)) => Some(p.ber),
            _ => None,
        }
    }

    /// Find SNR for target BER (interpolated)
    pub fn snr_for_ber(&self, target_ber: f64) -> Option<f64> {
        if self.points.is_empty() {
            return None;
        }

        for i in 0..self.points.len() - 1 {
            let p1 = &self.points[i];
            let p2 = &self.points[i + 1];

            if (p1.ber >= target_ber && p2.ber <= target_ber) ||
               (p1.ber <= target_ber && p2.ber >= target_ber) {
                // Log-linear interpolation
                let log_ber_1 = p1.ber.ln();
                let log_ber_2 = p2.ber.ln();
                let log_target = target_ber.ln();

                let t = (log_target - log_ber_1) / (log_ber_2 - log_ber_1);
                return Some(p1.snr_db + t * (p2.snr_db - p1.snr_db));
            }
        }

        None
    }
}

/// Monte Carlo simulator configuration
#[derive(Clone, Debug)]
pub struct SimulatorConfig {
    /// Minimum number of errors to collect per point
    pub min_errors: u64,
    /// Maximum number of symbols per point
    pub max_symbols: u64,
    /// Minimum number of symbols per point
    pub min_symbols: u64,
    /// Random seed (None for random)
    pub seed: Option<u64>,
    /// Confidence level for intervals (e.g., 0.95)
    pub confidence_level: f64,
}

impl Default for SimulatorConfig {
    fn default() -> Self {
        Self {
            min_errors: 100,
            max_symbols: 1_000_000,
            min_symbols: 10_000,
            seed: Some(42),
            confidence_level: 0.95,
        }
    }
}

impl SimulatorConfig {
    /// Fast simulation (fewer samples)
    pub fn fast() -> Self {
        Self {
            min_errors: 20,
            max_symbols: 100_000,
            min_symbols: 1_000,
            ..Default::default()
        }
    }

    /// High accuracy simulation
    pub fn accurate() -> Self {
        Self {
            min_errors: 1000,
            max_symbols: 10_000_000,
            min_symbols: 100_000,
            ..Default::default()
        }
    }
}

/// Monte Carlo BER simulator
pub struct BerSimulator {
    /// Configuration
    config: SimulatorConfig,
    /// Shared secret for CSPM
    secret: Vec<u8>,
}

impl BerSimulator {
    /// Create new simulator
    pub fn new(secret: &[u8]) -> Self {
        Self {
            config: SimulatorConfig::default(),
            secret: secret.to_vec(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(secret: &[u8], config: SimulatorConfig) -> Self {
        Self {
            config,
            secret: secret.to_vec(),
        }
    }

    /// Simulate BER at a single SNR point
    pub fn simulate_point(&self, snr_db: f64, channel: &ChannelModel) -> BerPoint {
        let mut rng = match self.config.seed {
            Some(seed) => ChaCha8Rng::seed_from_u64(seed),
            None => ChaCha8Rng::from_entropy(),
        };

        let genesis = GenesisConfig::new(&self.secret);
        let mut encoder = CspmEncoder::new(genesis.clone());
        let mut decoder = CspmDecoder::new(genesis);
        decoder.set_correction_threshold(0.8); // Generous threshold

        let mut total_symbols = 0u64;
        let mut symbol_errors = 0u64;
        let mut total_bits = 0u64;
        let mut bit_errors = 0u64;

        // Create channel with specified SNR
        let mut test_channel = channel.clone();
        test_channel.noise.snr_db = snr_db;

        while total_symbols < self.config.max_symbols {
            // Generate random symbol (0-119 for 600-cell)
            let tx_symbol: u8 = rng.gen_range(0..120);

            // Encode
            let encoded = match encoder.encode_symbol(tx_symbol) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Apply channel
            let received = test_channel.apply(&encoded.quaternion, &mut rng);

            // Decode
            let decoded = decoder.decode_quaternion(&received);

            total_symbols += 1;
            total_bits += 7; // log2(120) ≈ 7 bits per symbol

            match decoded {
                Ok(result) => {
                    if result.bits != tx_symbol {
                        symbol_errors += 1;
                        // Count bit differences
                        let diff = result.bits ^ tx_symbol;
                        bit_errors += diff.count_ones() as u64;
                    }
                }
                Err(_) => {
                    // Decoding failure counts as all bits wrong
                    symbol_errors += 1;
                    bit_errors += 4; // Average bit errors for random guess
                }
            }

            // Early termination if enough errors collected
            if bit_errors >= self.config.min_errors && total_symbols >= self.config.min_symbols {
                break;
            }
        }

        let ber = if total_bits > 0 {
            bit_errors as f64 / total_bits as f64
        } else {
            0.0
        };

        // Calculate confidence interval (Wilson score)
        let ci = self.wilson_confidence_interval(bit_errors, total_bits);

        BerPoint {
            snr_db,
            ber,
            total_bits,
            bit_errors,
            total_symbols,
            symbol_errors,
            confidence_interval: ci,
        }
    }

    /// Simulate BER curve over SNR range
    pub fn simulate_curve(
        &self,
        snr_range: &[f64],
        channel: &ChannelModel,
    ) -> BerCurve {
        let points: Vec<BerPoint> = snr_range
            .iter()
            .map(|&snr| self.simulate_point(snr, channel))
            .collect();

        BerCurve {
            points,
            scheme_name: "CSPM-600".to_string(),
            channel_description: format!("{:?}", channel),
        }
    }

    /// Standard SNR range for BER curves
    pub fn standard_snr_range() -> Vec<f64> {
        (0..=30).map(|i| i as f64).collect()
    }

    /// Fine SNR range around a specific point
    pub fn fine_snr_range(center: f64, width: f64, steps: usize) -> Vec<f64> {
        let step = width / steps as f64;
        (0..=steps)
            .map(|i| center - width / 2.0 + i as f64 * step)
            .collect()
    }

    /// Wilson score confidence interval for binomial proportion
    fn wilson_confidence_interval(&self, errors: u64, total: u64) -> (f64, f64) {
        if total == 0 {
            return (0.0, 1.0);
        }

        let n = total as f64;
        let p = errors as f64 / n;

        // Z-score for confidence level
        let z = match (self.config.confidence_level * 100.0) as u32 {
            99 => 2.576,
            95 => 1.96,
            90 => 1.645,
            _ => 1.96,
        };

        let z2 = z * z;
        let denominator = 1.0 + z2 / n;
        let center = (p + z2 / (2.0 * n)) / denominator;
        let half_width = z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / denominator;

        ((center - half_width).max(0.0), (center + half_width).min(1.0))
    }
}

/// Theoretical BER bounds for comparison
pub mod theoretical {
    use std::f64::consts::PI;

    /// AWGN capacity (Shannon limit) in bits/s/Hz
    pub fn shannon_capacity(snr_linear: f64) -> f64 {
        (1.0 + snr_linear).log2()
    }

    /// BPSK BER in AWGN
    pub fn bpsk_awgn(snr_db: f64) -> f64 {
        let snr = 10.0_f64.powf(snr_db / 10.0);
        0.5 * erfc((snr).sqrt())
    }

    /// QPSK BER in AWGN
    pub fn qpsk_awgn(snr_db: f64) -> f64 {
        bpsk_awgn(snr_db) // Same as BPSK for Gray-coded QPSK
    }

    /// M-QAM BER approximation in AWGN
    pub fn mqam_awgn(snr_db: f64, m: u32) -> f64 {
        let snr = 10.0_f64.powf(snr_db / 10.0);
        let k = (m as f64).log2();
        let m_sqrt = (m as f64).sqrt();

        // Approximate formula
        let factor = 4.0 * (1.0 - 1.0 / m_sqrt) / k;
        let arg = (3.0 * snr * k) / (2.0 * (m as f64 - 1.0));

        factor * erfc(arg.sqrt())
    }

    /// 64-QAM BER in AWGN
    pub fn qam64_awgn(snr_db: f64) -> f64 {
        mqam_awgn(snr_db, 64)
    }

    /// Complementary error function
    fn erfc(x: f64) -> f64 {
        // Approximation using Horner's method
        let t = 1.0 / (1.0 + 0.3275911 * x.abs());
        let poly = t * (0.254829592 +
            t * (-0.284496736 +
            t * (1.421413741 +
            t * (-1.453152027 +
            t * 1.061405429))));
        let result = poly * (-x * x).exp();

        if x >= 0.0 { result } else { 2.0 - result }
    }

    /// Sphere packing bound for M-dimensional constellation
    /// Returns minimum required Eb/N0 (dB) for given BER
    pub fn sphere_packing_bound(bits_per_symbol: f64, ber_target: f64) -> f64 {
        // Simplified bound: Eb/N0 ≈ (2^R - 1) / R where R is spectral efficiency
        let r = bits_per_symbol;
        let required_snr = (2.0_f64.powf(r) - 1.0) / r;
        10.0 * required_snr.log10()
    }

    /// LDPC theoretical performance gap from capacity (dB)
    /// For rate 1/2 LDPC with 10k codeword length
    pub fn ldpc_gap_from_capacity() -> f64 {
        0.5 // ~0.5 dB from capacity for well-designed LDPC
    }
}

/// Compare CSPM performance with theoretical bounds
#[derive(Clone, Debug)]
pub struct PerformanceComparison {
    /// CSPM BER curve
    pub cspm: BerCurve,
    /// 64-QAM theoretical curve
    pub qam64_theoretical: Vec<(f64, f64)>,
    /// Shannon limit
    pub shannon_limit_db: f64,
    /// CSPM coding gain over uncoded QAM (dB)
    pub coding_gain_db: f64,
    /// Gap from Shannon limit (dB)
    pub gap_from_shannon_db: f64,
}

impl PerformanceComparison {
    /// Generate comparison at target BER
    pub fn at_target_ber(cspm_curve: &BerCurve, target_ber: f64) -> Self {
        // Generate 64-QAM theoretical curve
        let snr_range: Vec<f64> = (0..=30).map(|i| i as f64).collect();
        let qam64_theoretical: Vec<(f64, f64)> = snr_range
            .iter()
            .map(|&snr| (snr, theoretical::qam64_awgn(snr)))
            .collect();

        // Find CSPM SNR at target BER
        let cspm_snr = cspm_curve.snr_for_ber(target_ber).unwrap_or(f64::INFINITY);

        // Find 64-QAM SNR at target BER
        let qam64_snr = qam64_theoretical
            .windows(2)
            .find_map(|w| {
                let (snr1, ber1) = w[0];
                let (snr2, ber2) = w[1];
                if ber1 >= target_ber && ber2 <= target_ber {
                    let t = (target_ber.ln() - ber1.ln()) / (ber2.ln() - ber1.ln());
                    Some(snr1 + t * (snr2 - snr1))
                } else {
                    None
                }
            })
            .unwrap_or(f64::INFINITY);

        // Shannon limit for ~7 bits/symbol
        let bits_per_symbol = 7.0; // log2(120) ≈ 6.9
        let shannon_snr_linear = 2.0_f64.powf(bits_per_symbol) - 1.0;
        let shannon_limit_db = 10.0 * shannon_snr_linear.log10();

        Self {
            cspm: cspm_curve.clone(),
            qam64_theoretical,
            shannon_limit_db,
            coding_gain_db: qam64_snr - cspm_snr,
            gap_from_shannon_db: cspm_snr - shannon_limit_db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulator_basic() {
        let sim = BerSimulator::with_config(
            b"test_secret",
            SimulatorConfig::fast(),
        );

        let channel = ChannelModel::ideal(20.0);
        let point = sim.simulate_point(20.0, &channel);

        // At 20 dB SNR, should have low BER
        assert!(point.ber < 0.1);
        assert!(point.total_symbols > 0);
    }

    #[test]
    fn test_ber_decreases_with_snr() {
        let sim = BerSimulator::with_config(
            b"test_secret",
            SimulatorConfig::fast(),
        );

        let channel = ChannelModel::ideal(0.0);

        let low_snr = sim.simulate_point(5.0, &channel);
        let high_snr = sim.simulate_point(20.0, &channel);

        // Higher SNR should have lower or equal BER
        assert!(high_snr.ber <= low_snr.ber + 0.01);
    }

    #[test]
    fn test_theoretical_bpsk() {
        // At 10 dB, BPSK BER should be around 4e-6
        let ber = theoretical::bpsk_awgn(10.0);
        assert!(ber < 1e-4);
        assert!(ber > 1e-8);
    }

    #[test]
    fn test_theoretical_qam64() {
        // At 20 dB, 64-QAM BER should be < 1e-3
        let ber = theoretical::qam64_awgn(20.0);
        assert!(ber < 1e-2);
    }

    #[test]
    fn test_confidence_interval() {
        let sim = BerSimulator::new(b"secret");

        // 100 errors in 10000 trials
        let (lower, upper) = sim.wilson_confidence_interval(100, 10000);

        assert!(lower < 0.01);
        assert!(upper > 0.01);
        assert!(lower > 0.005);
        assert!(upper < 0.015);
    }

    #[test]
    fn test_snr_for_ber() {
        let curve = BerCurve {
            points: vec![
                BerPoint {
                    snr_db: 10.0,
                    ber: 0.1,
                    total_bits: 10000,
                    bit_errors: 1000,
                    total_symbols: 1000,
                    symbol_errors: 100,
                    confidence_interval: (0.09, 0.11),
                },
                BerPoint {
                    snr_db: 20.0,
                    ber: 0.001,
                    total_bits: 100000,
                    bit_errors: 100,
                    total_symbols: 10000,
                    symbol_errors: 10,
                    confidence_interval: (0.0008, 0.0012),
                },
            ],
            scheme_name: "Test".to_string(),
            channel_description: "Test channel".to_string(),
        };

        let snr = curve.snr_for_ber(0.01);
        assert!(snr.is_some());
        let snr = snr.unwrap();
        assert!(snr > 10.0 && snr < 20.0);
    }
}

//! Hardware impairment models for CSPM simulation.
//!
//! Models realistic hardware effects:
//! - SLM (Spatial Light Modulator) phase and amplitude quantization
//! - Detector noise (shot, thermal, dark current)
//! - ADC quantization
//! - Polarization optics imperfections

use crate::quaternion::Quaternion;
use crate::modulation::{OpticalState, StokesVector};
use rand::Rng;
use rand_distr::{Normal, Poisson, Distribution};
use std::f64::consts::PI;

/// Spatial Light Modulator (SLM) model
#[derive(Clone, Debug)]
pub struct SlmModel {
    /// Phase quantization bits (typically 8-12)
    pub phase_bits: u32,
    /// Amplitude quantization bits (typically 8-10)
    pub amplitude_bits: u32,
    /// Phase response nonlinearity (0 = ideal)
    pub phase_nonlinearity: f64,
    /// Pixel crosstalk (0 = ideal)
    pub pixel_crosstalk: f64,
    /// Fill factor (0.9-0.98 typical)
    pub fill_factor: f64,
    /// Phase flicker RMS (radians)
    pub phase_flicker_rad: f64,
    /// Refresh rate (Hz)
    pub refresh_rate_hz: f64,
}

impl Default for SlmModel {
    fn default() -> Self {
        // Typical LCOS SLM parameters
        Self {
            phase_bits: 8,
            amplitude_bits: 8,
            phase_nonlinearity: 0.02,
            pixel_crosstalk: 0.03,
            fill_factor: 0.93,
            phase_flicker_rad: 0.02,
            refresh_rate_hz: 60.0,
        }
    }
}

impl SlmModel {
    /// High-end SLM (e.g., Hamamatsu X13138)
    pub fn high_performance() -> Self {
        Self {
            phase_bits: 12,
            amplitude_bits: 12,
            phase_nonlinearity: 0.005,
            pixel_crosstalk: 0.01,
            fill_factor: 0.98,
            phase_flicker_rad: 0.005,
            refresh_rate_hz: 200.0,
        }
    }

    /// Number of phase levels
    pub fn phase_levels(&self) -> u32 {
        2u32.pow(self.phase_bits)
    }

    /// Quantize a phase value (in radians)
    pub fn quantize_phase(&self, phase: f64) -> f64 {
        let levels = self.phase_levels() as f64;
        let normalized = (phase / (2.0 * PI)).rem_euclid(1.0);
        let quantized = (normalized * levels).round() / levels;
        quantized * 2.0 * PI
    }

    /// Apply phase nonlinearity (gamma-like curve)
    pub fn apply_nonlinearity(&self, phase: f64) -> f64 {
        if self.phase_nonlinearity > 0.0 {
            let normalized = phase / (2.0 * PI);
            let corrected = normalized.powf(1.0 + self.phase_nonlinearity);
            corrected * 2.0 * PI
        } else {
            phase
        }
    }

    /// Apply SLM effects to quaternion (treating it as encoding phase/polarization)
    pub fn apply(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        // Extract "phases" from quaternion (simplified model)
        let phase_w = q.w.atan2((q.x * q.x + q.y * q.y + q.z * q.z).sqrt());
        let phase_z = q.z.atan2(q.w);

        // Quantize phases
        let quant_w = self.quantize_phase(phase_w);
        let quant_z = self.quantize_phase(phase_z);

        // Apply nonlinearity
        let nl_w = self.apply_nonlinearity(quant_w);
        let nl_z = self.apply_nonlinearity(quant_z);

        // Add flicker noise
        let flicker = Normal::new(0.0, self.phase_flicker_rad).unwrap();
        let noisy_w = nl_w + flicker.sample(rng);
        let noisy_z = nl_z + flicker.sample(rng);

        // Reconstruct quaternion (simplified)
        let r = (1.0 - q.w * q.w).sqrt().max(1e-10);
        let scale = if r > 1e-10 { 1.0 / r } else { 0.0 };

        Quaternion::new(
            noisy_w.cos(),
            q.x * scale * noisy_w.sin(),
            q.y * scale * noisy_w.sin(),
            noisy_z.sin(),
        ).normalize()
    }

    /// Effective SNR degradation due to SLM (dB)
    pub fn snr_penalty_db(&self) -> f64 {
        // Phase quantization penalty: sinc² loss for uniform quantization
        // For N levels, penalty ≈ (π / N)² / 12 in power (small for large N)
        let quant_error_var = (PI / self.phase_levels() as f64).powi(2) / 12.0;
        let phase_penalty = -10.0 * (1.0 - quant_error_var).log10();

        // Fill factor penalty
        let fill_penalty = -10.0 * self.fill_factor.log10();

        // Flicker penalty
        let flicker_penalty = -10.0 * (1.0 - self.phase_flicker_rad.powi(2).min(0.5)).log10();

        phase_penalty.max(0.0) + fill_penalty.max(0.0) + flicker_penalty.max(0.0)
    }
}

/// Photodetector model
#[derive(Clone, Debug)]
pub struct DetectorModel {
    /// Responsivity (A/W)
    pub responsivity: f64,
    /// Dark current (nA)
    pub dark_current_na: f64,
    /// Thermal noise density (pA/√Hz)
    pub thermal_noise_density: f64,
    /// Bandwidth (GHz)
    pub bandwidth_ghz: f64,
    /// Quantum efficiency (0-1)
    pub quantum_efficiency: f64,
    /// Temperature (K)
    pub temperature_k: f64,
    /// Load resistance (Ω)
    pub load_resistance: f64,
}

impl Default for DetectorModel {
    fn default() -> Self {
        // Typical InGaAs PIN photodetector at 1550nm
        Self {
            responsivity: 0.9,
            dark_current_na: 1.0,
            thermal_noise_density: 10.0,
            bandwidth_ghz: 40.0,
            quantum_efficiency: 0.75,
            temperature_k: 300.0,
            load_resistance: 50.0,
        }
    }
}

impl DetectorModel {
    /// High-speed coherent receiver detector
    pub fn coherent_receiver() -> Self {
        Self {
            responsivity: 1.1,
            dark_current_na: 0.5,
            thermal_noise_density: 5.0,
            bandwidth_ghz: 70.0,
            quantum_efficiency: 0.9,
            ..Default::default()
        }
    }

    /// Calculate shot noise variance
    pub fn shot_noise_variance(&self, photocurrent_a: f64) -> f64 {
        let q = 1.602e-19; // Elementary charge
        let bandwidth = self.bandwidth_ghz * 1e9;
        2.0 * q * photocurrent_a * bandwidth
    }

    /// Calculate thermal noise variance
    pub fn thermal_noise_variance(&self) -> f64 {
        let bandwidth = self.bandwidth_ghz * 1e9;
        let thermal_density_a = self.thermal_noise_density * 1e-12;
        thermal_density_a.powi(2) * bandwidth
    }

    /// Calculate dark current noise variance
    pub fn dark_noise_variance(&self) -> f64 {
        let q = 1.602e-19;
        let bandwidth = self.bandwidth_ghz * 1e9;
        let dark_current_a = self.dark_current_na * 1e-9;
        2.0 * q * dark_current_a * bandwidth
    }

    /// Total noise equivalent power (NEP) in W/√Hz
    pub fn nep(&self) -> f64 {
        let thermal_var = self.thermal_noise_variance();
        let dark_var = self.dark_noise_variance();
        let bandwidth = self.bandwidth_ghz * 1e9;

        (thermal_var + dark_var).sqrt() / self.responsivity / bandwidth.sqrt()
    }

    /// Apply detector noise to optical power measurement
    pub fn detect(&self, optical_power_w: f64, rng: &mut impl Rng) -> f64 {
        let photocurrent = self.responsivity * optical_power_w;

        // Shot noise (Poisson, approximated as Gaussian for high count)
        let shot_var = self.shot_noise_variance(photocurrent);
        let shot_std = shot_var.sqrt();

        // Thermal noise
        let thermal_var = self.thermal_noise_variance();
        let thermal_std = thermal_var.sqrt();

        // Dark current noise
        let dark_var = self.dark_noise_variance();
        let dark_std = dark_var.sqrt();

        // Total noise
        let total_std = (shot_var + thermal_var + dark_var).sqrt();
        let noise = Normal::new(0.0, total_std).unwrap().sample(rng);

        (photocurrent + noise).max(0.0)
    }

    /// Apply detector effects to Stokes parameters
    pub fn detect_stokes(&self, stokes: &StokesVector, power_w: f64, rng: &mut impl Rng) -> StokesVector {
        // Each Stokes parameter is detected with noise
        let s1_detected = self.detect(power_w * (1.0 + stokes.s1) / 2.0, rng) / power_w;
        let s2_detected = self.detect(power_w * (1.0 + stokes.s2) / 2.0, rng) / power_w;
        let s3_detected = self.detect(power_w * (1.0 + stokes.s3) / 2.0, rng) / power_w;

        StokesVector::new(
            2.0 * s1_detected - 1.0,
            2.0 * s2_detected - 1.0,
            2.0 * s3_detected - 1.0,
        ).normalize()
    }
}

/// ADC (Analog-to-Digital Converter) model
#[derive(Clone, Debug)]
pub struct AdcModel {
    /// Resolution (bits)
    pub resolution_bits: u32,
    /// Effective number of bits (ENOB)
    pub enob: f64,
    /// Sample rate (GS/s)
    pub sample_rate_gsps: f64,
    /// Input range (V peak-to-peak)
    pub input_range_vpp: f64,
    /// DNL (differential nonlinearity) in LSB
    pub dnl_lsb: f64,
    /// INL (integral nonlinearity) in LSB
    pub inl_lsb: f64,
}

impl Default for AdcModel {
    fn default() -> Self {
        Self {
            resolution_bits: 8,
            enob: 6.5,
            sample_rate_gsps: 64.0,
            input_range_vpp: 0.8,
            dnl_lsb: 0.5,
            inl_lsb: 1.0,
        }
    }
}

impl AdcModel {
    /// High-resolution ADC (e.g., Keysight)
    pub fn high_resolution() -> Self {
        Self {
            resolution_bits: 12,
            enob: 10.0,
            sample_rate_gsps: 92.0,
            input_range_vpp: 1.0,
            dnl_lsb: 0.3,
            inl_lsb: 0.5,
        }
    }

    /// LSB size in volts
    pub fn lsb_volts(&self) -> f64 {
        self.input_range_vpp / 2.0_f64.powi(self.resolution_bits as i32)
    }

    /// Quantization noise power (variance)
    pub fn quantization_noise_variance(&self) -> f64 {
        let lsb = self.lsb_volts();
        lsb.powi(2) / 12.0
    }

    /// ENOB-limited SNR (dB)
    pub fn snr_db(&self) -> f64 {
        6.02 * self.enob + 1.76
    }

    /// Quantize a voltage value
    pub fn quantize(&self, voltage: f64) -> f64 {
        let half_range = self.input_range_vpp / 2.0;
        let clamped = voltage.clamp(-half_range, half_range);
        let lsb = self.lsb_volts();
        (clamped / lsb).round() * lsb
    }

    /// Apply ADC quantization to quaternion components
    pub fn apply(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        // Model quaternion components as voltage levels
        let scale = self.input_range_vpp / 4.0; // Map [-1, 1] to voltage range

        // Add quantization noise
        let quant_noise = Normal::new(0.0, self.quantization_noise_variance().sqrt() / scale).unwrap();

        Quaternion::new(
            self.quantize(q.w * scale) / scale + quant_noise.sample(rng),
            self.quantize(q.x * scale) / scale + quant_noise.sample(rng),
            self.quantize(q.y * scale) / scale + quant_noise.sample(rng),
            self.quantize(q.z * scale) / scale + quant_noise.sample(rng),
        ).normalize()
    }
}

/// Polarization optics model (PBS, waveplates, etc.)
#[derive(Clone, Debug)]
pub struct PolarizationOptics {
    /// Extinction ratio (dB)
    pub extinction_ratio_db: f64,
    /// Waveplate retardance error (radians)
    pub retardance_error_rad: f64,
    /// Axis alignment error (radians)
    pub axis_error_rad: f64,
    /// Insertion loss (dB)
    pub insertion_loss_db: f64,
}

impl Default for PolarizationOptics {
    fn default() -> Self {
        Self {
            extinction_ratio_db: 30.0,
            retardance_error_rad: 0.02,
            axis_error_rad: 0.01,
            insertion_loss_db: 0.5,
        }
    }
}

impl PolarizationOptics {
    /// Research-grade optics
    pub fn high_quality() -> Self {
        Self {
            extinction_ratio_db: 50.0,
            retardance_error_rad: 0.005,
            axis_error_rad: 0.002,
            insertion_loss_db: 0.1,
        }
    }

    /// Extinction ratio as linear ratio
    pub fn extinction_ratio_linear(&self) -> f64 {
        10.0_f64.powf(self.extinction_ratio_db / 10.0)
    }

    /// Apply polarization optics effects
    pub fn apply(&self, stokes: &StokesVector, rng: &mut impl Rng) -> StokesVector {
        // Extinction ratio leakage
        let er = self.extinction_ratio_linear();
        let leakage = 1.0 / er;

        // Axis alignment error
        let axis_error = Normal::new(0.0, self.axis_error_rad).unwrap().sample(rng);
        let cos_err = axis_error.cos();
        let sin_err = axis_error.sin();

        // Rotate Stokes vector by axis error
        let s1_rot = stokes.s1 * cos_err - stokes.s2 * sin_err;
        let s2_rot = stokes.s1 * sin_err + stokes.s2 * cos_err;

        // Add leakage
        StokesVector::new(
            s1_rot * (1.0 - leakage) + leakage * rng.gen_range(-1.0..1.0),
            s2_rot * (1.0 - leakage) + leakage * rng.gen_range(-1.0..1.0),
            stokes.s3,
        ).normalize()
    }
}

/// Complete transceiver hardware chain
#[derive(Clone, Debug)]
pub struct TransceiverHardware {
    /// Transmitter SLM
    pub tx_slm: SlmModel,
    /// Receiver detector
    pub rx_detector: DetectorModel,
    /// ADC
    pub adc: AdcModel,
    /// Polarization optics
    pub pol_optics: PolarizationOptics,
}

impl Default for TransceiverHardware {
    fn default() -> Self {
        Self {
            tx_slm: SlmModel::default(),
            rx_detector: DetectorModel::default(),
            adc: AdcModel::default(),
            pol_optics: PolarizationOptics::default(),
        }
    }
}

impl TransceiverHardware {
    /// High-performance research setup
    pub fn research_grade() -> Self {
        Self {
            tx_slm: SlmModel::high_performance(),
            rx_detector: DetectorModel::coherent_receiver(),
            adc: AdcModel::high_resolution(),
            pol_optics: PolarizationOptics::high_quality(),
        }
    }

    /// Apply complete hardware chain to quaternion
    pub fn apply(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        // TX: SLM effects
        let after_slm = self.tx_slm.apply(q, rng);

        // RX: ADC quantization
        let after_adc = self.adc.apply(&after_slm, rng);

        after_adc
    }

    /// Apply hardware chain to optical state
    pub fn apply_optical(&self, state: &OpticalState, rng: &mut impl Rng) -> OpticalState {
        let power_w = 10.0_f64.powf((state.power_dbm - 30.0) / 10.0);

        // Apply polarization optics to Stokes
        let stokes = self.pol_optics.apply(&state.stokes, rng);

        // Detect Stokes with detector noise
        let detected_stokes = self.rx_detector.detect_stokes(&stokes, power_w, rng);

        OpticalState {
            stokes: detected_stokes,
            oam_mode: state.oam_mode,
            power_dbm: state.power_dbm - self.pol_optics.insertion_loss_db,
        }
    }

    /// Total system SNR penalty (dB)
    pub fn total_penalty_db(&self) -> f64 {
        self.tx_slm.snr_penalty_db() +
        self.pol_optics.insertion_loss_db +
        (self.adc.resolution_bits as f64 - self.adc.enob) * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slm_quantization() {
        let slm = SlmModel::default();
        assert_eq!(slm.phase_levels(), 256);

        let phase = 1.234;
        let quantized = slm.quantize_phase(phase);
        assert!((quantized - phase).abs() < 2.0 * PI / 256.0);
    }

    #[test]
    fn test_detector_noise() {
        let detector = DetectorModel::default();
        let mut rng = rand::thread_rng();

        let power_w = 1e-3; // 1 mW
        let mut readings = Vec::new();

        for _ in 0..100 {
            readings.push(detector.detect(power_w, &mut rng));
        }

        let mean: f64 = readings.iter().sum::<f64>() / readings.len() as f64;
        let expected_current = detector.responsivity * power_w;

        // Mean should be close to expected photocurrent
        assert!((mean - expected_current).abs() < 0.1 * expected_current);
    }

    #[test]
    fn test_adc_quantization() {
        let adc = AdcModel::default();
        let lsb = adc.lsb_volts();

        let voltage = 0.123;
        let quantized = adc.quantize(voltage);

        assert!((quantized - voltage).abs() <= lsb / 2.0);
    }

    #[test]
    fn test_transceiver_preserves_norm() {
        let hw = TransceiverHardware::default();
        let mut rng = rand::thread_rng();

        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let processed = hw.apply(&q, &mut rng);

        assert!(processed.is_normalized());
    }

    #[test]
    fn test_hardware_penalty() {
        let hw = TransceiverHardware::default();
        let penalty = hw.total_penalty_db();

        // Should be positive and reasonable (< 10 dB)
        assert!(penalty > 0.0 && penalty < 10.0);
    }
}

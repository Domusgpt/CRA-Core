//! Realistic optical channel simulation for CSPM validation.
//!
//! This module provides physics-based channel models for:
//! - Fiber optic transmission (dispersion, PMD, attenuation)
//! - Free-space optical (atmospheric turbulence)
//! - OAM mode crosstalk
//! - Hardware impairments (SLM quantization, detector noise)

pub mod fiber;
pub mod freespace;
pub mod oam;
pub mod hardware;
pub mod monte_carlo;
pub mod baseline;

// Re-exports
pub use monte_carlo::{BerSimulator, BerCurve, BerPoint, SimulatorConfig};
pub use baseline::{QamConstellation, QamLdpcTransceiver, BaselineComparison};

use crate::quaternion::Quaternion;
use crate::modulation::{OpticalState, StokesVector};
use rand::Rng;
use rand_distr::{Normal, Distribution};

/// Physical constants for optical simulation
pub mod constants {
    /// Speed of light in vacuum (m/s)
    pub const C: f64 = 299_792_458.0;

    /// Planck's constant (J·s)
    pub const H: f64 = 6.626e-34;

    /// Boltzmann constant (J/K)
    pub const K_B: f64 = 1.381e-23;

    /// C-band center wavelength (m)
    pub const LAMBDA_C: f64 = 1.55e-6;

    /// C-band frequency (Hz)
    pub const FREQ_C: f64 = C / LAMBDA_C;

    /// Typical fiber attenuation at 1550nm (dB/km)
    pub const FIBER_ATTENUATION_DB_KM: f64 = 0.2;

    /// Typical chromatic dispersion at 1550nm (ps/(nm·km))
    pub const CHROMATIC_DISPERSION: f64 = 17.0;

    /// Typical PMD coefficient (ps/√km)
    pub const PMD_COEFFICIENT: f64 = 0.1;

    /// Kolmogorov turbulence structure constant (typical clear air)
    pub const CN2_CLEAR: f64 = 1e-15;

    /// Kolmogorov turbulence structure constant (moderate turbulence)
    pub const CN2_MODERATE: f64 = 1e-14;

    /// Kolmogorov turbulence structure constant (strong turbulence)
    pub const CN2_STRONG: f64 = 1e-13;
}

/// Combined channel model applying all impairments
#[derive(Clone, Debug)]
pub struct ChannelModel {
    /// Fiber parameters (None for free-space)
    pub fiber: Option<FiberParameters>,
    /// Free-space parameters (None for fiber)
    pub freespace: Option<FreespaceParameters>,
    /// OAM crosstalk matrix
    pub oam_crosstalk: OamCrosstalkModel,
    /// Hardware impairments
    pub hardware: HardwareModel,
    /// Additive noise (ASE, thermal, shot)
    pub noise: NoiseModel,
}

/// Fiber channel parameters
#[derive(Clone, Debug)]
pub struct FiberParameters {
    /// Fiber length (km)
    pub length_km: f64,
    /// Attenuation (dB/km)
    pub attenuation_db_km: f64,
    /// Chromatic dispersion (ps/(nm·km))
    pub chromatic_dispersion: f64,
    /// PMD coefficient (ps/√km)
    pub pmd_coefficient: f64,
    /// Number of amplifier spans
    pub num_spans: usize,
    /// Span length (km)
    pub span_length_km: f64,
    /// EDFA noise figure (dB)
    pub edfa_nf_db: f64,
}

impl Default for FiberParameters {
    fn default() -> Self {
        Self {
            length_km: 100.0,
            attenuation_db_km: constants::FIBER_ATTENUATION_DB_KM,
            chromatic_dispersion: constants::CHROMATIC_DISPERSION,
            pmd_coefficient: constants::PMD_COEFFICIENT,
            num_spans: 2,
            span_length_km: 50.0,
            edfa_nf_db: 5.0,
        }
    }
}

/// Free-space optical parameters
#[derive(Clone, Debug)]
pub struct FreespaceParameters {
    /// Link distance (m)
    pub distance_m: f64,
    /// Turbulence structure constant Cn²
    pub cn2: f64,
    /// Aperture diameter (m)
    pub aperture_m: f64,
    /// Wavelength (m)
    pub wavelength_m: f64,
    /// Pointing error standard deviation (μrad)
    pub pointing_error_urad: f64,
}

impl Default for FreespaceParameters {
    fn default() -> Self {
        Self {
            distance_m: 1000.0,
            cn2: constants::CN2_MODERATE,
            aperture_m: 0.1,
            wavelength_m: constants::LAMBDA_C,
            pointing_error_urad: 1.0,
        }
    }
}

/// OAM mode crosstalk model
#[derive(Clone, Debug)]
pub struct OamCrosstalkModel {
    /// Maximum OAM mode number
    pub max_mode: i32,
    /// Crosstalk probability to adjacent modes
    pub adjacent_crosstalk: f64,
    /// Crosstalk probability to non-adjacent modes
    pub distant_crosstalk: f64,
}

impl Default for OamCrosstalkModel {
    fn default() -> Self {
        Self {
            max_mode: 16,
            adjacent_crosstalk: 0.05,  // 5% to adjacent modes
            distant_crosstalk: 0.01,   // 1% to distant modes
        }
    }
}

/// Hardware impairment model
#[derive(Clone, Debug)]
pub struct HardwareModel {
    /// SLM phase quantization bits
    pub slm_phase_bits: u32,
    /// SLM amplitude quantization bits
    pub slm_amplitude_bits: u32,
    /// Polarization extinction ratio (dB)
    pub polarization_extinction_db: f64,
    /// Detector bandwidth (GHz)
    pub detector_bandwidth_ghz: f64,
    /// ADC resolution (bits)
    pub adc_bits: u32,
}

impl Default for HardwareModel {
    fn default() -> Self {
        Self {
            slm_phase_bits: 8,        // 256 phase levels
            slm_amplitude_bits: 8,
            polarization_extinction_db: 30.0,
            detector_bandwidth_ghz: 40.0,
            adc_bits: 12,
        }
    }
}

/// Noise model
#[derive(Clone, Debug)]
pub struct NoiseModel {
    /// Signal-to-noise ratio (dB)
    pub snr_db: f64,
    /// Include ASE noise
    pub include_ase: bool,
    /// Include shot noise
    pub include_shot: bool,
    /// Include thermal noise
    pub include_thermal: bool,
    /// Receiver temperature (K)
    pub temperature_k: f64,
}

impl Default for NoiseModel {
    fn default() -> Self {
        Self {
            snr_db: 20.0,
            include_ase: true,
            include_shot: true,
            include_thermal: true,
            temperature_k: 300.0,
        }
    }
}

impl ChannelModel {
    /// Create a fiber channel model
    pub fn fiber(length_km: f64, snr_db: f64) -> Self {
        Self {
            fiber: Some(FiberParameters {
                length_km,
                num_spans: (length_km / 50.0).ceil() as usize,
                span_length_km: 50.0,
                ..Default::default()
            }),
            freespace: None,
            oam_crosstalk: OamCrosstalkModel::default(),
            hardware: HardwareModel::default(),
            noise: NoiseModel { snr_db, ..Default::default() },
        }
    }

    /// Create a free-space optical channel model
    pub fn freespace(distance_m: f64, cn2: f64, snr_db: f64) -> Self {
        Self {
            fiber: None,
            freespace: Some(FreespaceParameters {
                distance_m,
                cn2,
                ..Default::default()
            }),
            oam_crosstalk: OamCrosstalkModel::default(),
            hardware: HardwareModel::default(),
            noise: NoiseModel { snr_db, ..Default::default() },
        }
    }

    /// Create an ideal channel (no impairments except noise)
    pub fn ideal(snr_db: f64) -> Self {
        Self {
            fiber: None,
            freespace: None,
            oam_crosstalk: OamCrosstalkModel {
                adjacent_crosstalk: 0.0,
                distant_crosstalk: 0.0,
                ..Default::default()
            },
            hardware: HardwareModel {
                slm_phase_bits: 16,  // Effectively continuous
                slm_amplitude_bits: 16,
                polarization_extinction_db: 100.0,
                ..Default::default()
            },
            noise: NoiseModel {
                snr_db,
                include_ase: false,
                include_shot: false,
                include_thermal: false,
                ..Default::default()
            },
        }
    }

    /// Apply channel effects to a quaternion signal
    pub fn apply(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        let mut result = *q;

        // 1. Apply fiber effects (if fiber channel)
        if let Some(ref fiber) = self.fiber {
            result = self.apply_fiber_effects(&result, fiber, rng);
        }

        // 2. Apply free-space effects (if FSO channel)
        if let Some(ref fso) = self.freespace {
            result = self.apply_freespace_effects(&result, fso, rng);
        }

        // 3. Apply OAM crosstalk
        result = self.apply_oam_crosstalk(&result, rng);

        // 4. Apply hardware impairments
        result = self.apply_hardware_effects(&result, rng);

        // 5. Apply additive noise
        result = self.apply_noise(&result, rng);

        result.normalize()
    }

    /// Apply channel effects to an optical state
    pub fn apply_optical(&self, state: &OpticalState, rng: &mut impl Rng) -> OpticalState {
        let mut result = state.clone();

        // Apply Stokes parameter noise
        let noise_std = self.stokes_noise_std();
        let normal = Normal::new(0.0, noise_std).unwrap();

        result.stokes = StokesVector::new(
            result.stokes.s1 + normal.sample(rng),
            result.stokes.s2 + normal.sample(rng),
            result.stokes.s3 + normal.sample(rng),
        ).normalize();

        // Apply OAM mode crosstalk
        result.oam_mode = self.apply_oam_mode_crosstalk(result.oam_mode, rng);

        // Apply power loss
        if let Some(ref fiber) = self.fiber {
            let loss_db = fiber.attenuation_db_km * fiber.length_km;
            result.power_dbm -= loss_db;
        }

        result
    }

    fn stokes_noise_std(&self) -> f64 {
        // Convert SNR to noise standard deviation on Stokes parameters
        let snr_linear = 10.0_f64.powf(self.noise.snr_db / 10.0);
        1.0 / snr_linear.sqrt()
    }

    fn apply_fiber_effects(&self, q: &Quaternion, fiber: &FiberParameters, rng: &mut impl Rng) -> Quaternion {
        let mut result = *q;

        // PMD: Random polarization rotation accumulating over distance
        let pmd_variance = (fiber.pmd_coefficient * fiber.length_km.sqrt()).powi(2);
        let pmd_std = pmd_variance.sqrt() * 0.01; // Scale to quaternion space

        let normal = Normal::new(0.0, pmd_std).unwrap();
        let pmd_rotation = Quaternion::new(
            1.0 + normal.sample(rng),
            normal.sample(rng),
            normal.sample(rng),
            normal.sample(rng),
        ).normalize();

        result = pmd_rotation * result * pmd_rotation.conjugate();

        // Chromatic dispersion: Phase distortion (simplified as phase noise)
        // Real CD would require frequency-domain modeling
        let cd_phase_std = fiber.chromatic_dispersion * fiber.length_km * 0.001;
        let phase_noise = Normal::new(0.0, cd_phase_std).unwrap().sample(rng);

        let phase_rotation = Quaternion::from_axis_angle([0.0, 0.0, 1.0], phase_noise);
        result = phase_rotation * result;

        result.normalize()
    }

    fn apply_freespace_effects(&self, q: &Quaternion, fso: &FreespaceParameters, rng: &mut impl Rng) -> Quaternion {
        // Rytov variance for scintillation
        let k = 2.0 * std::f64::consts::PI / fso.wavelength_m;
        let rytov = 1.23 * fso.cn2 * k.powf(7.0/6.0) * fso.distance_m.powf(11.0/6.0);

        // Log-amplitude variance
        let sigma_chi = (rytov / 4.0).sqrt().min(0.5); // Clamp for weak-to-moderate

        // Apply amplitude scintillation
        let log_normal = Normal::new(0.0, sigma_chi).unwrap();
        let amplitude_factor = (log_normal.sample(rng)).exp();

        // Apply phase scintillation (wavefront distortion)
        let phase_std = (rytov).sqrt().min(1.0);
        let phase_distortion = Normal::new(0.0, phase_std).unwrap().sample(rng);

        let phase_rotation = Quaternion::from_axis_angle([0.0, 0.0, 1.0], phase_distortion);
        let mut result = phase_rotation * *q;

        // Scale by amplitude (affects SNR, simplified here)
        result = result * amplitude_factor;

        // Pointing error
        let pointing_rad = fso.pointing_error_urad * 1e-6;
        let pointing_normal = Normal::new(0.0, pointing_rad).unwrap();
        let tilt_x = pointing_normal.sample(rng);
        let tilt_y = pointing_normal.sample(rng);

        let tilt_rotation = Quaternion::from_axis_angle([tilt_x, tilt_y, 0.0], (tilt_x*tilt_x + tilt_y*tilt_y).sqrt());
        result = tilt_rotation * result;

        result.normalize()
    }

    fn apply_oam_crosstalk(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        // OAM crosstalk affects the "twist" component encoded in quaternion
        // Simplified: add noise to the w-z plane (OAM phase)
        let crosstalk_std = self.oam_crosstalk.adjacent_crosstalk;

        if crosstalk_std > 0.0 {
            let normal = Normal::new(0.0, crosstalk_std).unwrap();
            Quaternion::new(
                q.w + normal.sample(rng) * 0.1,
                q.x,
                q.y,
                q.z + normal.sample(rng) * 0.1,
            ).normalize()
        } else {
            *q
        }
    }

    fn apply_oam_mode_crosstalk(&self, mode: i32, rng: &mut impl Rng) -> i32 {
        let r: f64 = rng.gen();

        if r < self.oam_crosstalk.adjacent_crosstalk {
            // Jump to adjacent mode
            if rng.gen::<bool>() {
                (mode + 1).min(self.oam_crosstalk.max_mode)
            } else {
                (mode - 1).max(-self.oam_crosstalk.max_mode)
            }
        } else if r < self.oam_crosstalk.adjacent_crosstalk + self.oam_crosstalk.distant_crosstalk {
            // Jump to random mode
            rng.gen_range(-self.oam_crosstalk.max_mode..=self.oam_crosstalk.max_mode)
        } else {
            mode
        }
    }

    fn apply_hardware_effects(&self, q: &Quaternion, _rng: &mut impl Rng) -> Quaternion {
        // SLM phase quantization
        let phase_levels = 2.0_f64.powi(self.hardware.slm_phase_bits as i32);

        // Quantize each component (simplified model)
        let quantize = |x: f64| -> f64 {
            let scaled = (x + 1.0) / 2.0 * phase_levels;
            (scaled.round() / phase_levels) * 2.0 - 1.0
        };

        Quaternion::new(
            quantize(q.w),
            quantize(q.x),
            quantize(q.y),
            quantize(q.z),
        ).normalize()
    }

    fn apply_noise(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        let snr_linear = 10.0_f64.powf(self.noise.snr_db / 10.0);
        let noise_std = 1.0 / (2.0 * snr_linear).sqrt();

        let normal = Normal::new(0.0, noise_std).unwrap();

        Quaternion::new(
            q.w + normal.sample(rng),
            q.x + normal.sample(rng),
            q.y + normal.sample(rng),
            q.z + normal.sample(rng),
        ).normalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ideal_channel() {
        let channel = ChannelModel::ideal(30.0);
        let mut rng = rand::thread_rng();

        let q = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        let received = channel.apply(&q, &mut rng);

        // High SNR should preserve signal well
        assert!(received.distance(&q) < 0.1);
    }

    #[test]
    fn test_fiber_channel() {
        let channel = ChannelModel::fiber(100.0, 20.0);
        let mut rng = rand::thread_rng();

        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let received = channel.apply(&q, &mut rng);

        // Should still be normalized
        assert!(received.is_normalized());
    }

    #[test]
    fn test_freespace_channel() {
        let channel = ChannelModel::freespace(1000.0, constants::CN2_MODERATE, 15.0);
        let mut rng = rand::thread_rng();

        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let received = channel.apply(&q, &mut rng);

        assert!(received.is_normalized());
    }
}

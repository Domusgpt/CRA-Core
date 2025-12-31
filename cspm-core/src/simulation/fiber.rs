//! Fiber optic channel model with realistic physical effects.
//!
//! Models:
//! - Chromatic dispersion (GVD)
//! - Polarization mode dispersion (PMD)
//! - Fiber attenuation
//! - EDFA amplified spontaneous emission (ASE)
//! - Kerr nonlinearity (simplified)

use crate::quaternion::Quaternion;
use rand::Rng;
use rand_distr::{Normal, Distribution};
use std::f64::consts::PI;

/// Detailed fiber segment model
#[derive(Clone, Debug)]
pub struct FiberSegment {
    /// Segment length (km)
    pub length_km: f64,
    /// Attenuation coefficient (dB/km)
    pub alpha_db_km: f64,
    /// Dispersion parameter D (ps/(nm·km))
    pub dispersion_d: f64,
    /// Dispersion slope S (ps/(nm²·km))
    pub dispersion_slope: f64,
    /// PMD coefficient (ps/√km)
    pub pmd_coeff: f64,
    /// Effective area (μm²)
    pub effective_area_um2: f64,
    /// Nonlinear coefficient n2 (m²/W)
    pub n2: f64,
}

impl Default for FiberSegment {
    fn default() -> Self {
        // Standard SMF-28 parameters
        Self {
            length_km: 80.0,
            alpha_db_km: 0.2,
            dispersion_d: 17.0,
            dispersion_slope: 0.058,
            pmd_coeff: 0.1,
            effective_area_um2: 80.0,
            n2: 2.6e-20,
        }
    }
}

/// EDFA amplifier model
#[derive(Clone, Debug)]
pub struct Edfa {
    /// Gain (dB)
    pub gain_db: f64,
    /// Noise figure (dB)
    pub noise_figure_db: f64,
    /// Saturation output power (dBm)
    pub p_sat_dbm: f64,
}

impl Default for Edfa {
    fn default() -> Self {
        Self {
            gain_db: 20.0,
            noise_figure_db: 5.0,
            p_sat_dbm: 17.0,
        }
    }
}

/// Complete fiber link model
#[derive(Clone, Debug)]
pub struct FiberLink {
    /// Fiber segments
    pub segments: Vec<FiberSegment>,
    /// Amplifiers (one per span)
    pub amplifiers: Vec<Edfa>,
    /// Symbol rate (GBaud)
    pub symbol_rate_gbaud: f64,
    /// Channel wavelength (nm)
    pub wavelength_nm: f64,
    /// Launch power (dBm)
    pub launch_power_dbm: f64,
}

impl FiberLink {
    /// Create a simple link with uniform spans
    pub fn uniform_spans(num_spans: usize, span_length_km: f64, symbol_rate_gbaud: f64) -> Self {
        let segments: Vec<_> = (0..num_spans)
            .map(|_| FiberSegment {
                length_km: span_length_km,
                ..Default::default()
            })
            .collect();

        let amplifiers: Vec<_> = (0..num_spans)
            .map(|_| Edfa {
                gain_db: span_length_km * 0.2, // Compensate attenuation
                ..Default::default()
            })
            .collect();

        Self {
            segments,
            amplifiers,
            symbol_rate_gbaud,
            wavelength_nm: 1550.0,
            launch_power_dbm: 0.0,
        }
    }

    /// Total link length
    pub fn total_length_km(&self) -> f64 {
        self.segments.iter().map(|s| s.length_km).sum()
    }

    /// Total accumulated dispersion
    pub fn total_dispersion(&self) -> f64 {
        self.segments.iter().map(|s| s.dispersion_d * s.length_km).sum()
    }

    /// Calculate OSNR at receiver (dB)
    pub fn osnr_db(&self) -> f64 {
        let h = 6.626e-34; // Planck's constant
        let c = 3e8;
        let lambda = self.wavelength_nm * 1e-9;
        let freq = c / lambda;

        let launch_power_w = 10.0_f64.powf((self.launch_power_dbm - 30.0) / 10.0);

        // ASE noise power per span
        let mut total_ase_power = 0.0;
        for (seg, amp) in self.segments.iter().zip(&self.amplifiers) {
            let nf_linear = 10.0_f64.powf(amp.noise_figure_db / 10.0);
            let gain_linear = 10.0_f64.powf(amp.gain_db / 10.0);
            let bandwidth = self.symbol_rate_gbaud * 1e9; // Hz

            // ASE power: P_ase = (NF - 1) * G * h * ν * B
            let p_ase = (nf_linear - 1.0) * gain_linear * h * freq * bandwidth;
            total_ase_power += p_ase;
        }

        if total_ase_power > 0.0 {
            10.0 * (launch_power_w / total_ase_power).log10()
        } else {
            100.0 // Infinite OSNR (no noise)
        }
    }

    /// Apply fiber effects to quaternion
    pub fn apply(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        let mut result = *q;

        for (segment, _amp) in self.segments.iter().zip(&self.amplifiers) {
            result = self.apply_segment(&result, segment, rng);
        }

        // Apply ASE noise based on OSNR
        result = self.apply_ase(&result, rng);

        result.normalize()
    }

    fn apply_segment(&self, q: &Quaternion, segment: &FiberSegment, rng: &mut impl Rng) -> Quaternion {
        // 1. PMD: Random birefringence rotation
        let pmd = self.apply_pmd(q, segment, rng);

        // 2. Chromatic dispersion: Phase distortion
        let cd = self.apply_chromatic_dispersion(&pmd, segment, rng);

        // 3. Nonlinear effects (SPM/XPM simplified)
        let nl = self.apply_nonlinearity(&cd, segment);

        nl
    }

    fn apply_pmd(&self, q: &Quaternion, segment: &FiberSegment, rng: &mut impl Rng) -> Quaternion {
        // PMD causes random rotation accumulating as √L
        let dgd_ps = segment.pmd_coeff * segment.length_km.sqrt();
        let dgd_normalized = dgd_ps / (1e12 / self.symbol_rate_gbaud / 1e9);

        // Random rotation axis on Poincaré sphere
        let normal = Normal::new(0.0, 1.0).unwrap();
        let axis: [f64; 3] = [
            normal.sample(rng),
            normal.sample(rng),
            normal.sample(rng),
        ];
        let axis_norm = (axis[0]*axis[0] + axis[1]*axis[1] + axis[2]*axis[2]).sqrt();

        if axis_norm > 1e-10 {
            let rotation_angle = dgd_normalized * PI;
            let rotation = Quaternion::from_axis_angle(
                [axis[0]/axis_norm, axis[1]/axis_norm, axis[2]/axis_norm],
                rotation_angle
            );
            (rotation * *q * rotation.conjugate()).normalize()
        } else {
            *q
        }
    }

    fn apply_chromatic_dispersion(&self, q: &Quaternion, segment: &FiberSegment, rng: &mut impl Rng) -> Quaternion {
        // CD causes phase rotation proportional to frequency offset
        // Simplified: add phase noise proportional to accumulated dispersion
        let accumulated_d = segment.dispersion_d * segment.length_km;
        let bandwidth_nm = 0.1; // Approximate signal bandwidth in nm

        let phase_spread = accumulated_d * bandwidth_nm * 1e-12 * self.symbol_rate_gbaud * 1e9;
        let phase_spread_rad = phase_spread * 2.0 * PI;

        let normal = Normal::new(0.0, phase_spread_rad.min(0.5)).unwrap();
        let phase_noise = normal.sample(rng);

        let phase_rotation = Quaternion::from_axis_angle([0.0, 0.0, 1.0], phase_noise);
        (phase_rotation * *q).normalize()
    }

    fn apply_nonlinearity(&self, q: &Quaternion, segment: &FiberSegment) -> Quaternion {
        // Simplified SPM: Phase rotation proportional to power and length
        let launch_power_w = 10.0_f64.powf((self.launch_power_dbm - 30.0) / 10.0);
        let effective_area_m2 = segment.effective_area_um2 * 1e-12;
        let gamma = 2.0 * PI * segment.n2 / (self.wavelength_nm * 1e-9 * effective_area_m2);

        // Effective length considering attenuation
        let alpha_neper_km = segment.alpha_db_km / 4.343;
        let l_eff = (1.0 - (-alpha_neper_km * segment.length_km).exp()) / alpha_neper_km;

        let spm_phase = gamma * launch_power_w * l_eff * 1000.0; // Convert km to m

        let phase_rotation = Quaternion::from_axis_angle([0.0, 0.0, 1.0], spm_phase.min(0.3));
        (phase_rotation * *q).normalize()
    }

    fn apply_ase(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        let osnr_db = self.osnr_db();
        let osnr_linear = 10.0_f64.powf(osnr_db / 10.0);
        let noise_std = 1.0 / (2.0 * osnr_linear).sqrt();

        let normal = Normal::new(0.0, noise_std).unwrap();

        Quaternion::new(
            q.w + normal.sample(rng),
            q.x + normal.sample(rng),
            q.y + normal.sample(rng),
            q.z + normal.sample(rng),
        ).normalize()
    }
}

/// Jones matrix for polarization transformation
#[derive(Clone, Debug)]
pub struct JonesMatrix {
    pub j00: (f64, f64), // Complex: (real, imag)
    pub j01: (f64, f64),
    pub j10: (f64, f64),
    pub j11: (f64, f64),
}

impl JonesMatrix {
    /// Identity matrix
    pub fn identity() -> Self {
        Self {
            j00: (1.0, 0.0),
            j01: (0.0, 0.0),
            j10: (0.0, 0.0),
            j11: (1.0, 0.0),
        }
    }

    /// Random unitary (for fiber PMD)
    pub fn random_unitary(rng: &mut impl Rng) -> Self {
        let normal = Normal::new(0.0, 1.0).unwrap();

        // Generate random SU(2) matrix via quaternion
        let q = Quaternion::new(
            normal.sample(rng),
            normal.sample(rng),
            normal.sample(rng),
            normal.sample(rng),
        ).normalize();

        // Convert quaternion to Jones matrix (SU(2) representation)
        Self {
            j00: (q.w, q.z),
            j01: (q.y, q.x),
            j10: (-q.y, q.x),
            j11: (q.w, -q.z),
        }
    }

    /// Apply Jones matrix to polarization state
    pub fn apply_to_stokes(&self, s: &crate::modulation::StokesVector) -> crate::modulation::StokesVector {
        // Convert Stokes to Jones, apply matrix, convert back
        // Simplified: use Mueller matrix formalism
        let m = self.to_mueller();

        crate::modulation::StokesVector::new(
            m[0][0] * 1.0 + m[0][1] * s.s1 + m[0][2] * s.s2 + m[0][3] * s.s3,
            m[1][0] * 1.0 + m[1][1] * s.s1 + m[1][2] * s.s2 + m[1][3] * s.s3,
            m[2][0] * 1.0 + m[2][1] * s.s1 + m[2][2] * s.s2 + m[2][3] * s.s3,
        ).normalize()
    }

    fn to_mueller(&self) -> [[f64; 4]; 4] {
        // Convert Jones to Mueller matrix (for pure polarization effects)
        // Using standard formulas
        let (a_r, a_i) = self.j00;
        let (b_r, b_i) = self.j01;
        let (c_r, c_i) = self.j10;
        let (d_r, d_i) = self.j11;

        let a2 = a_r*a_r + a_i*a_i;
        let b2 = b_r*b_r + b_i*b_i;
        let c2 = c_r*c_r + c_i*c_i;
        let d2 = d_r*d_r + d_i*d_i;

        [
            [0.5 * (a2 + b2 + c2 + d2), 0.5 * (a2 - b2 + c2 - d2), 0.0, 0.0],
            [0.5 * (a2 + b2 - c2 - d2), 0.5 * (a2 - b2 - c2 + d2), 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_link() {
        let link = FiberLink::uniform_spans(5, 80.0, 32.0);
        assert_eq!(link.total_length_km(), 400.0);
    }

    #[test]
    fn test_osnr_calculation() {
        let link = FiberLink::uniform_spans(5, 80.0, 32.0);
        let osnr = link.osnr_db();
        // Should be positive and reasonable (10-30 dB typical)
        assert!(osnr > 0.0 && osnr < 50.0);
    }

    #[test]
    fn test_fiber_preserves_norm() {
        let link = FiberLink::uniform_spans(2, 50.0, 32.0);
        let mut rng = rand::thread_rng();

        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let received = link.apply(&q, &mut rng);

        assert!(received.is_normalized());
    }
}

//! Free-space optical (FSO) channel model.
//!
//! Models atmospheric turbulence effects on OAM beams using:
//! - Kolmogorov turbulence theory
//! - Rytov approximation for weak-to-moderate turbulence
//! - Extended Rytov for strong turbulence
//! - Beam wander and scintillation

use crate::quaternion::Quaternion;
use rand::Rng;
use rand_distr::{Normal, LogNormal, Distribution};
use std::f64::consts::PI;

/// Atmospheric turbulence parameters
#[derive(Clone, Debug)]
pub struct TurbulenceProfile {
    /// Refractive index structure constant Cn² (m^(-2/3))
    pub cn2: f64,
    /// Inner scale of turbulence (m)
    pub l0_inner: f64,
    /// Outer scale of turbulence (m)
    pub l0_outer: f64,
    /// Wind speed (m/s) - for temporal coherence
    pub wind_speed: f64,
}

impl TurbulenceProfile {
    /// Clear air conditions
    pub fn clear() -> Self {
        Self {
            cn2: 1e-17,
            l0_inner: 0.01,
            l0_outer: 10.0,
            wind_speed: 5.0,
        }
    }

    /// Moderate turbulence
    pub fn moderate() -> Self {
        Self {
            cn2: 1e-14,
            l0_inner: 0.005,
            l0_outer: 5.0,
            wind_speed: 10.0,
        }
    }

    /// Strong turbulence
    pub fn strong() -> Self {
        Self {
            cn2: 1e-13,
            l0_inner: 0.001,
            l0_outer: 2.0,
            wind_speed: 15.0,
        }
    }

    /// Hufnagel-Valley model for slant path
    pub fn hufnagel_valley(altitude_m: f64, zenith_angle_rad: f64) -> Self {
        // HV 5/7 model parameters
        let h: f64 = altitude_m / zenith_angle_rad.cos();
        let v_rms: f64 = 21.0; // RMS wind speed (m/s)

        let cn2 = 5.94e-53 * (v_rms / 27.0).powi(2) * h.powi(10) * (-h / 1000.0).exp()
            + 2.7e-16 * (-h / 1500.0).exp()
            + 1.7e-14 * (-h / 100.0).exp();

        Self {
            cn2,
            l0_inner: 0.005,
            l0_outer: 5.0,
            wind_speed: v_rms,
        }
    }
}

/// Free-space optical link model
#[derive(Clone, Debug)]
pub struct FsoLink {
    /// Propagation distance (m)
    pub distance_m: f64,
    /// Wavelength (m)
    pub wavelength_m: f64,
    /// Transmitter aperture diameter (m)
    pub tx_aperture_m: f64,
    /// Receiver aperture diameter (m)
    pub rx_aperture_m: f64,
    /// Turbulence profile
    pub turbulence: TurbulenceProfile,
    /// Pointing error standard deviation (rad)
    pub pointing_error_rad: f64,
    /// Maximum OAM mode
    pub max_oam_mode: i32,
}

impl Default for FsoLink {
    fn default() -> Self {
        Self {
            distance_m: 1000.0,
            wavelength_m: 1.55e-6,
            tx_aperture_m: 0.1,
            rx_aperture_m: 0.2,
            turbulence: TurbulenceProfile::moderate(),
            pointing_error_rad: 1e-6,
            max_oam_mode: 16,
        }
    }
}

impl FsoLink {
    /// Create a ground-to-ground horizontal link
    pub fn horizontal(distance_m: f64, cn2: f64) -> Self {
        Self {
            distance_m,
            turbulence: TurbulenceProfile { cn2, ..TurbulenceProfile::moderate() },
            ..Default::default()
        }
    }

    /// Create a ground-to-satellite uplink
    pub fn uplink(zenith_angle_deg: f64) -> Self {
        let zenith_rad = zenith_angle_deg * PI / 180.0;
        let slant_range = 400_000.0 / zenith_rad.cos(); // LEO altitude

        Self {
            distance_m: slant_range,
            turbulence: TurbulenceProfile::hufnagel_valley(0.0, zenith_rad),
            tx_aperture_m: 0.3,
            rx_aperture_m: 0.5,
            ..Default::default()
        }
    }

    /// Wave number k = 2π/λ
    pub fn wavenumber(&self) -> f64 {
        2.0 * PI / self.wavelength_m
    }

    /// Rytov variance (scintillation strength)
    pub fn rytov_variance(&self) -> f64 {
        let k = self.wavenumber();
        1.23 * self.turbulence.cn2 * k.powf(7.0/6.0) * self.distance_m.powf(11.0/6.0)
    }

    /// Fried parameter r0 (coherence length)
    pub fn fried_parameter(&self) -> f64 {
        let k = self.wavenumber();
        (0.423 * k.powi(2) * self.turbulence.cn2 * self.distance_m).powf(-3.0/5.0)
    }

    /// Isoplanatic angle (rad)
    pub fn isoplanatic_angle(&self) -> f64 {
        let r0 = self.fried_parameter();
        0.314 * r0 / self.distance_m
    }

    /// Scintillation index for plane wave
    pub fn scintillation_index(&self) -> f64 {
        let sigma_r2 = self.rytov_variance();

        if sigma_r2 < 0.3 {
            // Weak turbulence: σ_I² ≈ σ_R²
            sigma_r2
        } else {
            // Strong turbulence: saturation
            1.0 + 0.86 / sigma_r2.powf(2.0/5.0)
        }
    }

    /// Apply atmospheric effects to quaternion
    pub fn apply(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        let mut result = *q;

        // 1. Scintillation (amplitude fluctuation)
        result = self.apply_scintillation(&result, rng);

        // 2. Phase distortion (wavefront aberration)
        result = self.apply_phase_distortion(&result, rng);

        // 3. Beam wander (pointing error)
        result = self.apply_beam_wander(&result, rng);

        // 4. OAM mode coupling
        result = self.apply_oam_coupling(&result, rng);

        result.normalize()
    }

    fn apply_scintillation(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        let sigma_i = self.scintillation_index().sqrt();

        // Log-normal amplitude fluctuation
        let sigma_chi = (sigma_i.powi(2) / 4.0).sqrt().min(0.5);
        let log_normal = LogNormal::new(-sigma_chi.powi(2) / 2.0, sigma_chi).unwrap();
        let amplitude_factor = log_normal.sample(rng);

        // Scale quaternion components (simplified)
        *q * amplitude_factor.sqrt()
    }

    fn apply_phase_distortion(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        let sigma_r2 = self.rytov_variance();
        let sigma_phi = sigma_r2.sqrt().min(2.0);

        let normal = Normal::new(0.0, sigma_phi).unwrap();
        let phase_error = normal.sample(rng);

        let rotation = Quaternion::from_axis_angle([0.0, 0.0, 1.0], phase_error);
        (rotation * *q).normalize()
    }

    fn apply_beam_wander(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        // Beam wander variance
        let r0 = self.fried_parameter();
        let sigma_bw = 0.54 * (self.distance_m / (2.0 * self.wavenumber() * r0.powi(2))).powf(0.5);

        let normal = Normal::new(0.0, sigma_bw + self.pointing_error_rad).unwrap();
        let theta_x = normal.sample(rng);
        let theta_y = normal.sample(rng);

        let tilt_angle = (theta_x.powi(2) + theta_y.powi(2)).sqrt();
        if tilt_angle > 1e-10 {
            let rotation = Quaternion::from_axis_angle(
                [theta_x / tilt_angle, theta_y / tilt_angle, 0.0],
                tilt_angle
            );
            (rotation * *q).normalize()
        } else {
            *q
        }
    }

    fn apply_oam_coupling(&self, q: &Quaternion, rng: &mut impl Rng) -> Quaternion {
        // OAM mode coupling due to turbulence
        // Characterized by spiral phase distortion
        let r0 = self.fried_parameter();
        let d_over_r0 = self.tx_aperture_m / r0;

        // Mode coupling probability increases with D/r0
        let coupling_prob = (1.0 - (-d_over_r0.powi(2)).exp()).min(0.3);

        if rng.gen::<f64>() < coupling_prob {
            // Apply random phase twist (OAM mixing)
            let normal = Normal::new(0.0, 0.1).unwrap();
            Quaternion::new(
                q.w + normal.sample(rng),
                q.x,
                q.y,
                q.z + normal.sample(rng),
            ).normalize()
        } else {
            *q
        }
    }

    /// Calculate OAM crosstalk matrix
    pub fn oam_crosstalk_matrix(&self) -> Vec<Vec<f64>> {
        let r0 = self.fried_parameter();
        let d_over_r0 = self.tx_aperture_m / r0;
        let n = (2 * self.max_oam_mode + 1) as usize;

        let mut matrix = vec![vec![0.0; n]; n];

        for i in 0..n {
            for j in 0..n {
                let l_i = i as i32 - self.max_oam_mode;
                let l_j = j as i32 - self.max_oam_mode;
                let delta_l = (l_i - l_j).abs() as f64;

                if i == j {
                    // Self-coupling (signal retention)
                    matrix[i][j] = (-0.1 * d_over_r0.powi(2)).exp();
                } else {
                    // Cross-coupling
                    matrix[i][j] = 0.1 * d_over_r0.powi(2) * (-delta_l / 2.0).exp();
                }
            }
        }

        // Normalize rows
        for row in &mut matrix {
            let sum: f64 = row.iter().sum();
            if sum > 0.0 {
                for val in row.iter_mut() {
                    *val /= sum;
                }
            }
        }

        matrix
    }
}

/// Aperture averaging factor
pub fn aperture_averaging(diameter_m: f64, wavelength_m: f64, distance_m: f64, cn2: f64) -> f64 {
    let k = 2.0 * PI / wavelength_m;
    let rho_f = (distance_m / k).sqrt(); // Fresnel zone size

    let d_over_rho = diameter_m / rho_f;

    // Churnside aperture averaging formula
    (1.0 + 1.062 * d_over_rho.powi(2)).powf(-7.0/6.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rytov_variance() {
        let link = FsoLink::horizontal(1000.0, 1e-14);
        let sigma_r2 = link.rytov_variance();

        // Should be in weak-to-moderate range for these parameters
        assert!(sigma_r2 > 0.0 && sigma_r2 < 10.0);
    }

    #[test]
    fn test_fried_parameter() {
        let link = FsoLink::horizontal(1000.0, 1e-14);
        let r0 = link.fried_parameter();

        // Typically cm to m scale
        assert!(r0 > 0.001 && r0 < 1.0);
    }

    #[test]
    fn test_fso_preserves_norm() {
        let link = FsoLink::horizontal(500.0, 1e-15);
        let mut rng = rand::thread_rng();

        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5);
        let received = link.apply(&q, &mut rng);

        assert!(received.is_normalized());
    }

    #[test]
    fn test_oam_crosstalk_matrix() {
        let link = FsoLink::default();
        let matrix = link.oam_crosstalk_matrix();

        // Matrix should be square and stochastic (rows sum to 1)
        assert_eq!(matrix.len(), 33); // 2*16+1 modes

        for row in &matrix {
            let sum: f64 = row.iter().sum();
            assert!((sum - 1.0).abs() < 0.01);
        }
    }
}

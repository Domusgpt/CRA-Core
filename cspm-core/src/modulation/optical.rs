//! Optical state representation (Stokes parameters + OAM).

use serde::{Deserialize, Serialize};

/// Stokes vector representing polarization state on Poincaré sphere
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StokesVector {
    /// S₁: Horizontal vs Vertical (+1 = H, -1 = V)
    pub s1: f64,
    /// S₂: Diagonal vs Anti-diagonal (+1 = D, -1 = A)
    pub s2: f64,
    /// S₃: Right vs Left circular (+1 = R, -1 = L)
    pub s3: f64,
}

impl StokesVector {
    /// Create a new Stokes vector
    pub fn new(s1: f64, s2: f64, s3: f64) -> Self {
        Self { s1, s2, s3 }
    }

    /// Horizontal linear polarization
    pub fn horizontal() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    /// Vertical linear polarization
    pub fn vertical() -> Self {
        Self::new(-1.0, 0.0, 0.0)
    }

    /// Diagonal (+45°) linear polarization
    pub fn diagonal() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    /// Anti-diagonal (-45°) linear polarization
    pub fn anti_diagonal() -> Self {
        Self::new(0.0, -1.0, 0.0)
    }

    /// Right circular polarization
    pub fn right_circular() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }

    /// Left circular polarization
    pub fn left_circular() -> Self {
        Self::new(0.0, 0.0, -1.0)
    }

    /// Degree of polarization (should be ≤ 1 for physical states)
    pub fn degree_of_polarization(&self) -> f64 {
        (self.s1 * self.s1 + self.s2 * self.s2 + self.s3 * self.s3).sqrt()
    }

    /// Check if this is a valid (fully polarized) state
    pub fn is_valid(&self) -> bool {
        let dop = self.degree_of_polarization();
        (dop - 1.0).abs() < 0.01
    }

    /// Normalize to unit sphere
    pub fn normalize(&self) -> Self {
        let dop = self.degree_of_polarization();
        if dop < 1e-10 {
            Self::horizontal() // Default
        } else {
            Self::new(self.s1 / dop, self.s2 / dop, self.s3 / dop)
        }
    }

    /// Euclidean distance to another Stokes vector
    pub fn distance(&self, other: &Self) -> f64 {
        let ds1 = self.s1 - other.s1;
        let ds2 = self.s2 - other.s2;
        let ds3 = self.s3 - other.s3;
        (ds1 * ds1 + ds2 * ds2 + ds3 * ds3).sqrt()
    }

    /// Convert to array [S₁, S₂, S₃]
    pub fn to_array(&self) -> [f64; 3] {
        [self.s1, self.s2, self.s3]
    }

    /// Create from array [S₁, S₂, S₃]
    pub fn from_array(arr: [f64; 3]) -> Self {
        Self::new(arr[0], arr[1], arr[2])
    }

    /// Convert to spherical coordinates (azimuth, elevation)
    pub fn to_spherical(&self) -> (f64, f64) {
        let azimuth = self.s2.atan2(self.s1);
        let elevation = self.s3.asin();
        (azimuth, elevation)
    }

    /// Create from spherical coordinates
    pub fn from_spherical(azimuth: f64, elevation: f64) -> Self {
        Self::new(
            elevation.cos() * azimuth.cos(),
            elevation.cos() * azimuth.sin(),
            elevation.sin(),
        )
    }
}

impl Default for StokesVector {
    fn default() -> Self {
        Self::horizontal()
    }
}

/// Complete optical state (polarization + OAM)
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OpticalState {
    /// Polarization state (Stokes vector)
    pub stokes: StokesVector,
    /// OAM mode (topological charge ℓ)
    pub oam_mode: i32,
    /// Optical power in dBm
    pub power_dbm: f64,
}

impl OpticalState {
    /// Create a new optical state
    pub fn new(stokes: StokesVector, oam_mode: i32, power_dbm: f64) -> Self {
        Self {
            stokes,
            oam_mode,
            power_dbm,
        }
    }

    /// Create with default power
    pub fn from_stokes_oam(stokes: StokesVector, oam_mode: i32) -> Self {
        Self::new(stokes, oam_mode, 0.0)
    }

    /// Add Gaussian noise to simulate channel effects
    pub fn add_noise(&self, stokes_noise_std: f64, oam_noise_prob: f64) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Add Gaussian noise to Stokes parameters
        let noisy_stokes = StokesVector::new(
            self.stokes.s1 + rng.gen::<f64>() * stokes_noise_std * 2.0 - stokes_noise_std,
            self.stokes.s2 + rng.gen::<f64>() * stokes_noise_std * 2.0 - stokes_noise_std,
            self.stokes.s3 + rng.gen::<f64>() * stokes_noise_std * 2.0 - stokes_noise_std,
        ).normalize();

        // Occasionally flip OAM mode (modal crosstalk)
        let noisy_oam = if rng.gen::<f64>() < oam_noise_prob {
            self.oam_mode + if rng.gen::<bool>() { 1 } else { -1 }
        } else {
            self.oam_mode
        };

        Self::new(noisy_stokes, noisy_oam, self.power_dbm)
    }

    /// Distance to another optical state (combined metric)
    pub fn distance(&self, other: &Self) -> f64 {
        let stokes_dist = self.stokes.distance(&other.stokes);
        let oam_dist = (self.oam_mode - other.oam_mode).abs() as f64;
        // Weight: Stokes contributes more due to continuous nature
        stokes_dist + oam_dist * 0.5
    }
}

impl Default for OpticalState {
    fn default() -> Self {
        Self::new(StokesVector::horizontal(), 0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stokes_polarizations() {
        assert!(StokesVector::horizontal().is_valid());
        assert!(StokesVector::vertical().is_valid());
        assert!(StokesVector::right_circular().is_valid());
    }

    #[test]
    fn test_stokes_orthogonal() {
        let h = StokesVector::horizontal();
        let v = StokesVector::vertical();
        assert!((h.distance(&v) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_stokes_spherical_roundtrip() {
        let original = StokesVector::new(0.5, 0.5, 0.707);
        let normalized = original.normalize();
        let (az, el) = normalized.to_spherical();
        let recovered = StokesVector::from_spherical(az, el);
        assert!(normalized.distance(&recovered) < 1e-10);
    }

    #[test]
    fn test_optical_state_noise() {
        let state = OpticalState::default();
        let noisy = state.add_noise(0.1, 0.0);

        // Stokes should be slightly different
        assert!(state.stokes.distance(&noisy.stokes) > 0.0);
        // OAM should be same (0% noise probability)
        assert_eq!(state.oam_mode, noisy.oam_mode);
    }
}

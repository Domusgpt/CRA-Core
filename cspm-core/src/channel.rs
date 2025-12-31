//! Physical Channel Encoding - Quaternion to Light
//!
//! Maps 4D quaternions onto physical light properties:
//! - **Polarization** (x, y, z) → Poincaré Sphere (Stokes parameters)
//! - **OAM Mode** (w) → Orbital Angular Momentum (twist number)
//!
//! This creates a true 4D signal space in optical hardware.

use crate::quaternion::Quaternion;
use serde::{Deserialize, Serialize};

/// Stokes vector representing polarization state
///
/// Normalized Stokes parameters on the Poincaré sphere:
/// - S₁: Horizontal vs Vertical
/// - S₂: +45° vs -45° diagonal
/// - S₃: Right vs Left circular
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StokesVector {
    /// S₁/S₀: Horizontal (1) to Vertical (-1)
    pub s1: f64,
    /// S₂/S₀: +45° (1) to -45° (-1)
    pub s2: f64,
    /// S₃/S₀: Right circular (1) to Left circular (-1)
    pub s3: f64,
}

impl StokesVector {
    /// Create a new Stokes vector
    pub fn new(s1: f64, s2: f64, s3: f64) -> Self {
        Self { s1, s2, s3 }
    }

    /// Horizontal polarization
    pub fn horizontal() -> Self {
        Self::new(1.0, 0.0, 0.0)
    }

    /// Vertical polarization
    pub fn vertical() -> Self {
        Self::new(-1.0, 0.0, 0.0)
    }

    /// +45° diagonal polarization
    pub fn diagonal_plus() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }

    /// Right circular polarization
    pub fn right_circular() -> Self {
        Self::new(0.0, 0.0, 1.0)
    }

    /// Left circular polarization
    pub fn left_circular() -> Self {
        Self::new(0.0, 0.0, -1.0)
    }

    /// Normalize to unit sphere
    pub fn normalize(&self) -> Self {
        let mag = (self.s1 * self.s1 + self.s2 * self.s2 + self.s3 * self.s3).sqrt();
        if mag < 1e-10 {
            return Self::horizontal();
        }
        Self {
            s1: self.s1 / mag,
            s2: self.s2 / mag,
            s3: self.s3 / mag,
        }
    }

    /// Distance between two Stokes vectors (on unit sphere)
    pub fn angular_distance(&self, other: &Self) -> f64 {
        let dot = (self.s1 * other.s1 + self.s2 * other.s2 + self.s3 * other.s3)
            .min(1.0)
            .max(-1.0);
        dot.acos()
    }

    /// Add noise to Stokes vector
    pub fn add_noise(&self, noise_std: f64, rng: &mut impl rand::Rng) -> Self {
        use rand::Rng;
        Self {
            s1: self.s1 + rng.gen_range(-noise_std..noise_std),
            s2: self.s2 + rng.gen_range(-noise_std..noise_std),
            s3: self.s3 + rng.gen_range(-noise_std..noise_std),
        }
        .normalize()
    }
}

/// Poincaré sphere mapper - converts quaternion imaginary part to polarization
pub struct PoincareMapper;

impl PoincareMapper {
    /// Convert quaternion imaginary components (x, y, z) to Stokes vector
    ///
    /// The imaginary part of a unit quaternion lives on a 2-sphere embedded in R³,
    /// which maps naturally to the Poincaré sphere.
    pub fn to_stokes(q: &Quaternion) -> StokesVector {
        // The imaginary part magnitude
        let imag_mag = (q.x * q.x + q.y * q.y + q.z * q.z).sqrt();

        if imag_mag < 1e-10 {
            // Pure real quaternion - default to horizontal
            return StokesVector::horizontal();
        }

        // Normalize imaginary part to Poincaré sphere
        StokesVector {
            s1: q.x / imag_mag,
            s2: q.y / imag_mag,
            s3: q.z / imag_mag,
        }
    }

    /// Convert Stokes vector back to quaternion imaginary part
    ///
    /// Needs the magnitude (from OAM) to fully reconstruct
    pub fn from_stokes(stokes: &StokesVector, imag_magnitude: f64) -> (f64, f64, f64) {
        let s = stokes.normalize();
        (
            s.s1 * imag_magnitude,
            s.s2 * imag_magnitude,
            s.s3 * imag_magnitude,
        )
    }

    /// Get the imaginary magnitude from quaternion (encodes in OAM)
    pub fn imaginary_magnitude(q: &Quaternion) -> f64 {
        (q.x * q.x + q.y * q.y + q.z * q.z).sqrt()
    }
}

/// OAM mode for orbital angular momentum encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OAMMode {
    /// Topological charge (twist number)
    pub l: i32,
}

impl OAMMode {
    pub fn new(l: i32) -> Self {
        Self { l }
    }
}

/// OAM mapper - converts quaternion scalar (w) to OAM mode
pub struct OAMMapper {
    /// Maximum OAM mode number (typically ±5 to ±10)
    max_mode: i32,
}

impl OAMMapper {
    /// Create a new OAM mapper with given maximum mode
    pub fn new(max_mode: i32) -> Self {
        Self { max_mode }
    }

    /// Default mapper with ±8 modes (17 total states)
    pub fn default_mapper() -> Self {
        Self::new(8)
    }

    /// Convert quaternion w component to OAM mode
    ///
    /// Maps w ∈ [-1, 1] to mode ℓ ∈ [-max_mode, max_mode]
    pub fn to_oam_mode(&self, w: f64) -> OAMMode {
        let scaled = w * (self.max_mode as f64);
        OAMMode::new(scaled.round() as i32)
    }

    /// Convert OAM mode back to w component
    pub fn from_oam_mode(&self, mode: &OAMMode) -> f64 {
        let clamped = mode.l.clamp(-self.max_mode, self.max_mode);
        (clamped as f64) / (self.max_mode as f64)
    }

    /// Number of distinct OAM states
    pub fn num_states(&self) -> usize {
        (2 * self.max_mode + 1) as usize
    }

    /// Add noise to OAM mode
    pub fn add_noise(&self, mode: &OAMMode, noise_std: f64, rng: &mut impl rand::Rng) -> OAMMode {
        use rand::Rng;
        let noisy = mode.l as f64 + rng.gen_range(-noise_std..noise_std);
        OAMMode::new(noisy.round() as i32)
    }
}

/// Complete physical channel state (what gets transmitted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalState {
    /// Polarization on Poincaré sphere
    pub stokes: StokesVector,
    /// OAM mode (twist number)
    pub oam: OAMMode,
    /// Intensity (normalized, for amplitude encoding if needed)
    pub intensity: f64,
}

impl PhysicalState {
    /// Create a new physical state
    pub fn new(stokes: StokesVector, oam: OAMMode, intensity: f64) -> Self {
        Self {
            stokes,
            oam,
            intensity,
        }
    }
}

/// Physical channel encoder - quaternion to light
pub struct PhysicalEncoder {
    oam_mapper: OAMMapper,
}

impl PhysicalEncoder {
    pub fn new(max_oam_mode: i32) -> Self {
        Self {
            oam_mapper: OAMMapper::new(max_oam_mode),
        }
    }

    pub fn default_encoder() -> Self {
        Self::new(8)
    }

    /// Encode a quaternion to physical light state
    pub fn encode(&self, q: &Quaternion) -> PhysicalState {
        // Polarization from imaginary part
        let stokes = PoincareMapper::to_stokes(q);

        // OAM from real part (w)
        let oam = self.oam_mapper.to_oam_mode(q.w);

        // Intensity encodes imaginary magnitude (for full reconstruction)
        let intensity = PoincareMapper::imaginary_magnitude(q);

        PhysicalState::new(stokes, oam, intensity)
    }
}

/// Physical channel decoder - light to quaternion
pub struct PhysicalDecoder {
    oam_mapper: OAMMapper,
}

impl PhysicalDecoder {
    pub fn new(max_oam_mode: i32) -> Self {
        Self {
            oam_mapper: OAMMapper::new(max_oam_mode),
        }
    }

    pub fn default_decoder() -> Self {
        Self::new(8)
    }

    /// Decode physical light state to quaternion
    pub fn decode(&self, state: &PhysicalState) -> Quaternion {
        // Recover w from OAM
        let w = self.oam_mapper.from_oam_mode(&state.oam);

        // Recover imaginary part from Stokes + intensity
        let (x, y, z) = PoincareMapper::from_stokes(&state.stokes, state.intensity);

        Quaternion::new(w, x, y, z).normalize()
    }
}

/// Channel noise model for simulation
pub struct ChannelNoiseModel {
    /// Standard deviation for Stokes parameter noise
    pub stokes_noise_std: f64,
    /// Standard deviation for OAM mode noise
    pub oam_noise_std: f64,
    /// Intensity noise (relative)
    pub intensity_noise_std: f64,
}

impl ChannelNoiseModel {
    /// Low noise channel (fiber, short distance)
    pub fn low_noise() -> Self {
        Self {
            stokes_noise_std: 0.01,
            oam_noise_std: 0.1,
            intensity_noise_std: 0.01,
        }
    }

    /// Medium noise channel (fiber, long distance with repeaters)
    pub fn medium_noise() -> Self {
        Self {
            stokes_noise_std: 0.05,
            oam_noise_std: 0.3,
            intensity_noise_std: 0.05,
        }
    }

    /// High noise channel (free-space, atmospheric)
    pub fn high_noise() -> Self {
        Self {
            stokes_noise_std: 0.15,
            oam_noise_std: 0.5,
            intensity_noise_std: 0.1,
        }
    }

    /// Apply noise to a physical state
    pub fn apply(&self, state: &PhysicalState, rng: &mut impl rand::Rng) -> PhysicalState {
        use rand::Rng;

        let noisy_stokes = state.stokes.add_noise(self.stokes_noise_std, rng);
        let noisy_oam = OAMMapper::default_mapper().add_noise(&state.oam, self.oam_noise_std, rng);
        let noisy_intensity =
            (state.intensity + rng.gen_range(-self.intensity_noise_std..self.intensity_noise_std))
                .max(0.0);

        PhysicalState::new(noisy_stokes, noisy_oam, noisy_intensity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stokes_roundtrip() {
        let q = Quaternion::new(0.5, 0.5, 0.5, 0.5).normalize();

        let stokes = PoincareMapper::to_stokes(&q);
        let imag_mag = PoincareMapper::imaginary_magnitude(&q);
        let (x, y, z) = PoincareMapper::from_stokes(&stokes, imag_mag);

        assert!((q.x - x).abs() < 0.01);
        assert!((q.y - y).abs() < 0.01);
        assert!((q.z - z).abs() < 0.01);
    }

    #[test]
    fn test_oam_roundtrip() {
        let mapper = OAMMapper::new(10);

        for w in [-1.0, -0.5, 0.0, 0.5, 1.0] {
            let mode = mapper.to_oam_mode(w);
            let recovered = mapper.from_oam_mode(&mode);
            assert!(
                (w - recovered).abs() < 0.2,
                "OAM roundtrip failed for w={}",
                w
            );
        }
    }

    #[test]
    fn test_physical_encode_decode() {
        let encoder = PhysicalEncoder::default_encoder();
        let decoder = PhysicalDecoder::default_decoder();

        let q = Quaternion::new(0.6, 0.4, 0.5, 0.3).normalize();

        let state = encoder.encode(&q);
        let recovered = decoder.decode(&state);

        // Should approximately recover the quaternion
        let dist = q.geodesic_distance(&recovered);
        assert!(
            dist < 0.3,
            "Physical roundtrip distance too large: {}",
            dist
        );
    }

    #[test]
    fn test_noise_tolerance() {
        let encoder = PhysicalEncoder::default_encoder();
        let decoder = PhysicalDecoder::default_decoder();
        let noise = ChannelNoiseModel::low_noise();
        let mut rng = rand::thread_rng();

        let q = Quaternion::new(0.7, 0.3, 0.4, 0.2).normalize();

        let clean_state = encoder.encode(&q);
        let noisy_state = noise.apply(&clean_state, &mut rng);

        let recovered = decoder.decode(&noisy_state);

        // Low noise should still give reasonable recovery
        let dist = q.geodesic_distance(&recovered);
        assert!(
            dist < 0.5,
            "Low noise recovery distance too large: {}",
            dist
        );
    }
}

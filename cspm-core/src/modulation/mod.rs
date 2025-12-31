//! CSPM modulation and demodulation.
//!
//! Implements the full encoding/decoding pipeline:
//! - Data → Vertex → Quaternion → Optical State (encode)
//! - Optical State → Quaternion → Vertex → Data (decode)

mod optical;
mod encoder;
mod decoder;

pub use optical::{OpticalState, StokesVector};
pub use encoder::CspmEncoder;
pub use decoder::CspmDecoder;

use crate::MAX_OAM_MODES;
use std::f64::consts::PI;

/// Convert quaternion to optical state (Stokes + OAM)
pub fn quaternion_to_optical(q: &crate::quaternion::Quaternion) -> OpticalState {
    // Compute Stokes parameters from quaternion
    // Using the Hopf fibration mapping
    let s1 = 2.0 * (q.x * q.y + q.w * q.z);
    let s2 = 2.0 * (q.y * q.z - q.w * q.x);
    let s3 = q.w * q.w + q.z * q.z - q.x * q.x - q.y * q.y;

    // Compute OAM mode from quaternion phase
    let phase = q.z.atan2(q.w);
    let oam_mode = (phase * MAX_OAM_MODES as f64 / PI).round() as i32;
    let oam_mode = oam_mode.clamp(-MAX_OAM_MODES, MAX_OAM_MODES);

    OpticalState {
        stokes: StokesVector { s1, s2, s3 },
        oam_mode,
        power_dbm: 0.0, // Nominal power
    }
}

/// Convert optical state to quaternion
pub fn optical_to_quaternion(state: &OpticalState) -> crate::quaternion::Quaternion {
    let StokesVector { s1, s2, s3 } = state.stokes;

    // Compute polarization angles
    let r = (s1 * s1 + s2 * s2 + s3 * s3).sqrt();
    let r = if r < 1e-10 { 1.0 } else { r };

    let phi = s2.atan2(s1);
    let psi = (s3 / r).clamp(-1.0, 1.0).acos();

    // OAM phase
    let theta = PI * state.oam_mode as f64 / MAX_OAM_MODES as f64;

    // Reconstruct quaternion
    let half_theta = theta / 2.0;
    let half_psi = psi / 2.0;

    let q = crate::quaternion::Quaternion::new(
        half_theta.cos() * half_psi.cos(),
        half_theta.sin() * half_psi.sin() * phi.cos(),
        half_theta.sin() * half_psi.sin() * phi.sin(),
        half_theta.cos() * half_psi.sin(),
    );

    q.normalize()
}

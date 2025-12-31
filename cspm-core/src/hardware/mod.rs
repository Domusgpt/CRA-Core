//! Hardware abstraction layer for optical CSPM systems.
//!
//! Provides trait-based interfaces for:
//! - Spatial Light Modulators (SLM)
//! - Coherent optical receivers
//! - DAC/ADC systems
//! - FPGA accelerators
//!
//! This allows the CSPM protocol to be hardware-agnostic while
//! enabling lab integration when equipment becomes available.

pub mod slm;
pub mod receiver;
pub mod traits;

pub use traits::{
    OpticalModulator, OpticalReceiver, HardwareConfig,
    ModulatorCapabilities, ReceiverCapabilities,
};
pub use slm::{SlmController, SlmConfig, HologramGenerator};
pub use receiver::{CoherentReceiver, ReceiverConfig, StokesExtractor};

use crate::modulation::{OpticalState, StokesVector};

/// Hardware error types
#[derive(Debug, Clone)]
pub enum HardwareError {
    /// Device not connected
    NotConnected,
    /// Device busy
    Busy,
    /// Invalid configuration
    InvalidConfig(String),
    /// Communication error
    CommError(String),
    /// Calibration required
    CalibrationRequired,
    /// Feature not supported
    NotSupported(String),
}

impl std::fmt::Display for HardwareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => write!(f, "Device not connected"),
            Self::Busy => write!(f, "Device busy"),
            Self::InvalidConfig(s) => write!(f, "Invalid configuration: {}", s),
            Self::CommError(s) => write!(f, "Communication error: {}", s),
            Self::CalibrationRequired => write!(f, "Calibration required"),
            Self::NotSupported(s) => write!(f, "Feature not supported: {}", s),
        }
    }
}

impl std::error::Error for HardwareError {}

pub type HardwareResult<T> = Result<T, HardwareError>;

/// Simulated hardware backend for testing without physical devices
pub struct SimulatedBackend {
    /// Simulated SLM state
    current_hologram: Option<Vec<f64>>,
    /// Simulated receiver state
    last_measurement: Option<OpticalState>,
    /// Add noise to simulation
    noise_enabled: bool,
    /// Noise standard deviation
    noise_std: f64,
}

impl SimulatedBackend {
    /// Create new simulated backend
    pub fn new() -> Self {
        Self {
            current_hologram: None,
            last_measurement: None,
            noise_enabled: true,
            noise_std: 0.01,
        }
    }

    /// Enable/disable noise simulation
    pub fn set_noise(&mut self, enabled: bool, std: f64) {
        self.noise_enabled = enabled;
        self.noise_std = std;
    }

    /// Simulate SLM output and receiver input (loopback test)
    pub fn loopback(&mut self, state: &OpticalState) -> OpticalState {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        if self.noise_enabled {
            OpticalState {
                stokes: StokesVector::new(
                    state.stokes.s1 + rng.gen::<f64>() * self.noise_std,
                    state.stokes.s2 + rng.gen::<f64>() * self.noise_std,
                    state.stokes.s3 + rng.gen::<f64>() * self.noise_std,
                ),
                oam_mode: state.oam_mode,
                power_dbm: state.power_dbm + rng.gen::<f64>() * self.noise_std,
            }
        } else {
            state.clone()
        }
    }
}

impl Default for SimulatedBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulated_backend() {
        let mut backend = SimulatedBackend::new();
        backend.set_noise(false, 0.0);

        let state = OpticalState {
            stokes: StokesVector::new(0.5, 0.3, 0.1),
            oam_mode: 2,
            power_dbm: 0.0,
        };

        let received = backend.loopback(&state);
        assert_eq!(received.oam_mode, state.oam_mode);
    }
}

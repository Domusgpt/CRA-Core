//! Coherent optical receiver interface.
//!
//! Provides abstraction for optical receivers that extract:
//! - Stokes polarization parameters (S0, S1, S2, S3)
//! - OAM mode detection
//! - Phase and amplitude
//!
//! Supports various detector configurations:
//! - 4-detector polarimeter
//! - Coherent receivers with local oscillator
//! - Camera-based OAM sorters

use crate::quaternion::Quaternion;
use crate::modulation::{OpticalState, StokesVector};
use super::traits::{OpticalReceiver, ReceiverCapabilities, DeviceStatus};
use super::HardwareResult;

/// Receiver configuration
#[derive(Clone, Debug)]
pub struct ReceiverConfig {
    /// Device type
    pub device_type: ReceiverDeviceType,
    /// Number of detectors
    pub detector_count: u8,
    /// Sampling rate (Hz)
    pub sample_rate: f64,
    /// Integration time (seconds)
    pub integration_time: f64,
    /// Local oscillator wavelength (nm)
    pub lo_wavelength_nm: Option<f64>,
}

/// Supported receiver device types
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiverDeviceType {
    /// Simulated for testing
    Simulated,
    /// 4-detector polarimeter
    Polarimeter4,
    /// Coherent receiver
    CoherentRx,
    /// Camera-based OAM sorter
    OamCamera,
    /// Generic ADC-based
    Generic(String),
}

impl Default for ReceiverConfig {
    fn default() -> Self {
        Self {
            device_type: ReceiverDeviceType::Simulated,
            detector_count: 4,
            sample_rate: 1e9,
            integration_time: 1e-9,
            lo_wavelength_nm: Some(1550.0),
        }
    }
}

/// Coherent optical receiver
pub struct CoherentReceiver {
    config: ReceiverConfig,
    capabilities: ReceiverCapabilities,
    connected: bool,
    status: DeviceStatus,
    /// Simulated input state (for testing)
    simulated_input: Option<OpticalState>,
    /// Noise standard deviation
    noise_std: f64,
}

impl CoherentReceiver {
    /// Create new coherent receiver
    pub fn new(config: ReceiverConfig) -> Self {
        let capabilities = ReceiverCapabilities {
            detector_count: config.detector_count,
            bandwidth: config.sample_rate / 2.0,
            coherent: config.lo_wavelength_nm.is_some(),
            stokes_extraction: true,
            oam_detection: true,
            max_oam_mode: 16,
        };

        Self {
            config,
            capabilities,
            connected: false,
            status: DeviceStatus::Disconnected,
            simulated_input: None,
            noise_std: 0.01,
        }
    }

    /// Create simulated receiver for testing
    pub fn simulated() -> Self {
        Self::new(ReceiverConfig::default())
    }

    /// Set simulated input (for loopback testing)
    pub fn set_simulated_input(&mut self, state: OpticalState) {
        self.simulated_input = Some(state);
    }

    /// Set noise level for simulation
    pub fn set_noise(&mut self, std: f64) {
        self.noise_std = std;
    }

    /// Get configuration
    pub fn config(&self) -> &ReceiverConfig {
        &self.config
    }
}

impl OpticalReceiver for CoherentReceiver {
    fn capabilities(&self) -> &ReceiverCapabilities {
        &self.capabilities
    }

    fn connect(&mut self) -> HardwareResult<()> {
        match &self.config.device_type {
            ReceiverDeviceType::Simulated => {
                self.connected = true;
                self.status = DeviceStatus::Ready;
                Ok(())
            }
            _ => {
                // Real hardware connection would go here
                self.connected = true;
                self.status = DeviceStatus::Ready;
                Ok(())
            }
        }
    }

    fn disconnect(&mut self) -> HardwareResult<()> {
        self.connected = false;
        self.status = DeviceStatus::Disconnected;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn measure(&mut self) -> HardwareResult<OpticalState> {
        if !self.connected {
            return Err(super::HardwareError::NotConnected);
        }

        match &self.simulated_input {
            Some(input) => {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                // Add noise to simulate real measurement
                Ok(OpticalState {
                    stokes: StokesVector::new(
                        input.stokes.s1 + rng.gen::<f64>() * self.noise_std,
                        input.stokes.s2 + rng.gen::<f64>() * self.noise_std,
                        input.stokes.s3 + rng.gen::<f64>() * self.noise_std,
                    ),
                    oam_mode: input.oam_mode,
                    power_dbm: input.power_dbm + rng.gen::<f64>() * self.noise_std,
                })
            }
            None => {
                // No input - return zero state
                Ok(OpticalState::default())
            }
        }
    }

    fn measure_stokes(&mut self) -> HardwareResult<StokesVector> {
        let state = self.measure()?;
        Ok(state.stokes)
    }

    fn measure_oam(&mut self) -> HardwareResult<i32> {
        let state = self.measure()?;
        Ok(state.oam_mode)
    }

    fn start_streaming(&mut self, _callback: Box<dyn Fn(OpticalState) + Send>)
        -> HardwareResult<()>
    {
        if !self.connected {
            return Err(super::HardwareError::NotConnected);
        }
        // Streaming implementation would go here
        Ok(())
    }

    fn stop_streaming(&mut self) -> HardwareResult<()> {
        Ok(())
    }

    fn calibrate(&mut self) -> HardwareResult<()> {
        if !self.connected {
            return Err(super::HardwareError::NotConnected);
        }
        self.status = DeviceStatus::Calibrating;
        // Calibration logic would go here
        self.status = DeviceStatus::Ready;
        Ok(())
    }

    fn status(&self) -> DeviceStatus {
        self.status.clone()
    }
}

/// Stokes parameter extraction utilities
pub struct StokesExtractor;

impl StokesExtractor {
    /// Extract Stokes parameters from 4-detector measurements
    /// I_H, I_V, I_D, I_R (horizontal, vertical, diagonal, right-circular)
    /// Returns normalized Stokes vector (s1, s2, s3)
    pub fn from_4_detector(i_h: f64, i_v: f64, i_d: f64, i_r: f64) -> StokesVector {
        let s0 = i_h + i_v;
        if s0 < 1e-10 {
            return StokesVector::horizontal();
        }
        let s1 = (i_h - i_v) / s0;
        let s2 = (2.0 * i_d - s0) / s0;
        let s3 = (2.0 * i_r - s0) / s0;

        StokesVector::new(s1, s2, s3)
    }

    /// Extract from 6-detector complete polarimeter
    /// Returns normalized Stokes vector (s1, s2, s3)
    pub fn from_6_detector(
        i_h: f64, i_v: f64,  // H/V
        i_d: f64, i_a: f64,  // D/A (diagonal/anti-diagonal)
        i_r: f64, i_l: f64,  // R/L (right/left circular)
    ) -> StokesVector {
        let s0 = i_h + i_v;
        if s0 < 1e-10 {
            return StokesVector::horizontal();
        }
        let s1 = (i_h - i_v) / s0;
        let s2 = (i_d - i_a) / s0;
        let s3 = (i_r - i_l) / s0;

        StokesVector::new(s1, s2, s3)
    }

    /// Normalize Stokes vector to unit sphere
    pub fn normalize(stokes: &StokesVector) -> StokesVector {
        stokes.normalize()
    }

    /// Convert Stokes to Jones vector (partially polarized light approximation)
    pub fn stokes_to_jones(stokes: &StokesVector) -> (f64, f64, f64, f64) {
        let normalized = Self::normalize(stokes);

        // Azimuth and ellipticity
        let psi = 0.5 * normalized.s2.atan2(normalized.s1);
        let chi = 0.5 * normalized.s3.asin();

        // Jones vector components (Ex, Ey as complex: (re, im))
        let ex_re = psi.cos() * chi.cos();
        let ex_im = -psi.sin() * chi.sin();
        let ey_re = psi.sin() * chi.cos();
        let ey_im = psi.cos() * chi.sin();

        (ex_re, ex_im, ey_re, ey_im)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_creation() {
        let rx = CoherentReceiver::simulated();
        assert!(!rx.is_connected());
        assert_eq!(rx.status(), DeviceStatus::Disconnected);
    }

    #[test]
    fn test_receiver_connect() {
        let mut rx = CoherentReceiver::simulated();
        rx.connect().unwrap();
        assert!(rx.is_connected());
        assert_eq!(rx.status(), DeviceStatus::Ready);
    }

    #[test]
    fn test_stokes_extraction_4_detector() {
        // Pure horizontal polarization
        let stokes = StokesExtractor::from_4_detector(1.0, 0.0, 0.5, 0.5);
        assert!((stokes.s1 - 1.0).abs() < 0.01);
        assert!(stokes.s2.abs() < 0.01);
        assert!(stokes.s3.abs() < 0.01);
    }

    #[test]
    fn test_simulated_measurement() {
        let mut rx = CoherentReceiver::simulated();
        rx.connect().unwrap();
        rx.set_noise(0.0);

        let input = OpticalState {
            stokes: StokesVector::new(0.5, 0.3, 0.1),
            oam_mode: 2,
            power_dbm: 0.0,
        };

        rx.set_simulated_input(input.clone());
        let measured = rx.measure().unwrap();

        assert_eq!(measured.oam_mode, input.oam_mode);
        assert!((measured.stokes.s1 - input.stokes.s1).abs() < 0.01);
    }

    #[test]
    fn test_stokes_to_jones() {
        // Horizontal polarization
        let stokes = StokesVector::horizontal();
        let (ex_re, _ex_im, ey_re, _ey_im) = StokesExtractor::stokes_to_jones(&stokes);

        // Should be mostly Ex
        assert!(ex_re.abs() > 0.9);
        assert!(ey_re.abs() < 0.1);
    }
}

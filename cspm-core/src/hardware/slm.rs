//! Spatial Light Modulator (SLM) controller interface.
//!
//! Provides abstraction for various SLM hardware:
//! - Hamamatsu LCOS-SLM
//! - Meadowlark Optics
//! - Holoeye
//! - Thorlabs
//!
//! The SLM generates holograms that encode both polarization
//! (via Jones matrix) and OAM mode (via spiral phase plate).

use std::f64::consts::PI;
use crate::modulation::{OpticalState, StokesVector};
use super::traits::{OpticalModulator, ModulatorCapabilities, DeviceStatus};
use super::HardwareResult;

/// SLM configuration
#[derive(Clone, Debug)]
pub struct SlmConfig {
    /// Device type
    pub device_type: SlmDeviceType,
    /// Resolution
    pub resolution: (u32, u32),
    /// Wavelength (nm)
    pub wavelength_nm: f64,
    /// Pixel pitch (μm)
    pub pixel_pitch_um: f64,
    /// Maximum phase modulation (radians)
    pub max_phase: f64,
    /// Gamma correction LUT
    pub gamma_lut: Option<Vec<u8>>,
}

/// Supported SLM device types
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlmDeviceType {
    /// Simulated for testing
    Simulated,
    /// Hamamatsu LCOS-SLM
    Hamamatsu,
    /// Meadowlark Optics
    Meadowlark,
    /// Holoeye
    Holoeye,
    /// Thorlabs
    Thorlabs,
    /// Generic USB/network device
    Generic(String),
}

impl Default for SlmConfig {
    fn default() -> Self {
        Self {
            device_type: SlmDeviceType::Simulated,
            resolution: (1920, 1080),
            wavelength_nm: 1550.0,
            pixel_pitch_um: 8.0,
            max_phase: 2.0 * PI,
            gamma_lut: None,
        }
    }
}

/// SLM controller
pub struct SlmController {
    config: SlmConfig,
    capabilities: ModulatorCapabilities,
    connected: bool,
    current_state: Option<OpticalState>,
    current_hologram: Vec<f64>,
    status: DeviceStatus,
}

impl SlmController {
    /// Create new SLM controller
    pub fn new(config: SlmConfig) -> Self {
        let (w, h) = config.resolution;
        let capabilities = ModulatorCapabilities {
            resolution: config.resolution,
            max_oam_mode: 16,
            refresh_rate: 60.0,
            phase_bits: 8,
            polarization_control: true,
            amplitude_modulation: false,
        };

        Self {
            config,
            capabilities,
            connected: false,
            current_state: None,
            current_hologram: vec![0.0; (w * h) as usize],
            status: DeviceStatus::Disconnected,
        }
    }

    /// Create simulated controller for testing
    pub fn simulated() -> Self {
        Self::new(SlmConfig::default())
    }

    /// Get configuration
    pub fn config(&self) -> &SlmConfig {
        &self.config
    }
}

impl OpticalModulator for SlmController {
    fn capabilities(&self) -> &ModulatorCapabilities {
        &self.capabilities
    }

    fn connect(&mut self) -> HardwareResult<()> {
        match &self.config.device_type {
            SlmDeviceType::Simulated => {
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

    fn set_state(&mut self, state: &OpticalState) -> HardwareResult<()> {
        if !self.connected {
            return Err(super::HardwareError::NotConnected);
        }

        // Generate hologram from optical state
        let hologram = HologramGenerator::generate(
            state,
            self.config.resolution,
            self.config.wavelength_nm,
            self.config.pixel_pitch_um,
        );

        self.current_hologram = hologram;
        self.current_state = Some(state.clone());
        Ok(())
    }

    fn set_hologram(&mut self, pattern: &[f64]) -> HardwareResult<()> {
        if !self.connected {
            return Err(super::HardwareError::NotConnected);
        }

        let expected_size = (self.config.resolution.0 * self.config.resolution.1) as usize;
        if pattern.len() != expected_size {
            return Err(super::HardwareError::InvalidConfig(
                format!("Hologram size mismatch: expected {}, got {}", expected_size, pattern.len())
            ));
        }

        self.current_hologram = pattern.to_vec();
        Ok(())
    }

    fn get_state(&self) -> Option<&OpticalState> {
        self.current_state.as_ref()
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

/// Hologram generation for SLM
pub struct HologramGenerator;

impl HologramGenerator {
    /// Generate hologram for optical state
    pub fn generate(
        state: &OpticalState,
        resolution: (u32, u32),
        wavelength_nm: f64,
        pixel_pitch_um: f64,
    ) -> Vec<f64> {
        let (width, height) = resolution;
        let size = (width * height) as usize;
        let mut hologram = vec![0.0; size];

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;

                // Position relative to center
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;

                // OAM spiral phase
                let oam_phase = if state.oam_mode != 0 {
                    (state.oam_mode as f64) * dy.atan2(dx)
                } else {
                    0.0
                };

                // Polarization encoding (simplified)
                let pol_phase = Self::stokes_to_phase(&state.stokes, dx, dy);

                // Combined phase (power_dbm influences amplitude, mapped to phase offset)
                let power_phase = state.power_dbm * 0.01; // Small phase contribution from power
                hologram[idx] = (oam_phase + pol_phase + power_phase) % (2.0 * PI);
            }
        }

        hologram
    }

    /// Convert Stokes parameters to phase pattern
    fn stokes_to_phase(stokes: &StokesVector, _x: f64, _y: f64) -> f64 {
        // Simplified mapping - real implementation would use Jones calculus
        // For normalized Stokes vectors, s0 = sqrt(s1² + s2² + s3²) ≈ 1
        let dop = stokes.degree_of_polarization();
        let s0 = if dop > 1e-10 { dop } else { 1.0 };
        let azimuth = 0.5 * stokes.s2.atan2(stokes.s1);
        let ellipticity = 0.5 * (stokes.s3 / s0).clamp(-1.0, 1.0).asin();

        azimuth + ellipticity * 0.1
    }

    /// Generate OAM mode hologram
    pub fn oam_hologram(mode: i32, resolution: (u32, u32)) -> Vec<f64> {
        let (width, height) = resolution;
        let size = (width * height) as usize;
        let mut hologram = vec![0.0; size];

        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                hologram[idx] = (mode as f64) * dy.atan2(dx);
            }
        }

        hologram
    }

    /// Generate blazed grating
    pub fn blazed_grating(
        resolution: (u32, u32),
        period_pixels: f64,
        angle_rad: f64,
    ) -> Vec<f64> {
        let (width, height) = resolution;
        let size = (width * height) as usize;
        let mut hologram = vec![0.0; size];

        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let rotated = x as f64 * cos_a + y as f64 * sin_a;
                hologram[idx] = (2.0 * PI * rotated / period_pixels) % (2.0 * PI);
            }
        }

        hologram
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slm_controller_creation() {
        let slm = SlmController::simulated();
        assert!(!slm.is_connected());
        assert_eq!(slm.status(), DeviceStatus::Disconnected);
    }

    #[test]
    fn test_slm_connect_disconnect() {
        let mut slm = SlmController::simulated();
        slm.connect().unwrap();
        assert!(slm.is_connected());
        assert_eq!(slm.status(), DeviceStatus::Ready);

        slm.disconnect().unwrap();
        assert!(!slm.is_connected());
    }

    #[test]
    fn test_hologram_generation() {
        let state = OpticalState {
            stokes: StokesVector::new(0.5, 0.3, 0.1),
            oam_mode: 2,
            power_dbm: 0.0,
        };

        let hologram = HologramGenerator::generate(
            &state,
            (64, 64),
            1550.0,
            8.0,
        );

        assert_eq!(hologram.len(), 64 * 64);
    }

    #[test]
    fn test_oam_hologram() {
        let hologram = HologramGenerator::oam_hologram(3, (128, 128));
        assert_eq!(hologram.len(), 128 * 128);

        // Center should have well-defined phase
        let center = hologram[64 * 128 + 64];
        assert!(center.is_finite());
    }
}

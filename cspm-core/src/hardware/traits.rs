//! Hardware abstraction traits for optical devices.
//!
//! These traits define the interface that any hardware implementation
//! must provide, enabling hardware-agnostic CSPM operation.

use crate::quaternion::Quaternion;
use crate::modulation::{OpticalState, StokesVector};
use super::HardwareResult;

/// Capabilities of an optical modulator (SLM)
#[derive(Clone, Debug)]
pub struct ModulatorCapabilities {
    /// Resolution (pixels)
    pub resolution: (u32, u32),
    /// Maximum OAM mode supported
    pub max_oam_mode: i32,
    /// Refresh rate (Hz)
    pub refresh_rate: f64,
    /// Bit depth for phase
    pub phase_bits: u8,
    /// Supports polarization control
    pub polarization_control: bool,
    /// Supports amplitude modulation
    pub amplitude_modulation: bool,
}

/// Capabilities of an optical receiver
#[derive(Clone, Debug)]
pub struct ReceiverCapabilities {
    /// Number of photodetectors
    pub detector_count: u8,
    /// Bandwidth (Hz)
    pub bandwidth: f64,
    /// Supports coherent detection
    pub coherent: bool,
    /// Supports Stokes parameter extraction
    pub stokes_extraction: bool,
    /// Supports OAM mode detection
    pub oam_detection: bool,
    /// Maximum OAM mode detectable
    pub max_oam_mode: i32,
}

/// Configuration for hardware devices
#[derive(Clone, Debug)]
pub struct HardwareConfig {
    /// Device identifier
    pub device_id: String,
    /// Connection string (USB, network, etc.)
    pub connection: String,
    /// Custom parameters
    pub params: std::collections::HashMap<String, String>,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        Self {
            device_id: "simulated".to_string(),
            connection: "loopback".to_string(),
            params: std::collections::HashMap::new(),
        }
    }
}

/// Trait for optical modulators (SLMs, DMDs, etc.)
pub trait OpticalModulator: Send + Sync {
    /// Get device capabilities
    fn capabilities(&self) -> &ModulatorCapabilities;

    /// Connect to device
    fn connect(&mut self) -> HardwareResult<()>;

    /// Disconnect from device
    fn disconnect(&mut self) -> HardwareResult<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Set optical state (quaternion → hologram → SLM)
    fn set_state(&mut self, state: &OpticalState) -> HardwareResult<()>;

    /// Set raw hologram pattern
    fn set_hologram(&mut self, pattern: &[f64]) -> HardwareResult<()>;

    /// Get current state
    fn get_state(&self) -> Option<&OpticalState>;

    /// Calibrate device
    fn calibrate(&mut self) -> HardwareResult<()>;

    /// Get device status
    fn status(&self) -> DeviceStatus;
}

/// Trait for optical receivers (photodetectors, cameras, etc.)
pub trait OpticalReceiver: Send + Sync {
    /// Get device capabilities
    fn capabilities(&self) -> &ReceiverCapabilities;

    /// Connect to device
    fn connect(&mut self) -> HardwareResult<()>;

    /// Disconnect from device
    fn disconnect(&mut self) -> HardwareResult<()>;

    /// Check if connected
    fn is_connected(&self) -> bool;

    /// Measure optical state (single shot)
    fn measure(&mut self) -> HardwareResult<OpticalState>;

    /// Measure Stokes parameters
    fn measure_stokes(&mut self) -> HardwareResult<StokesVector>;

    /// Measure OAM mode
    fn measure_oam(&mut self) -> HardwareResult<i32>;

    /// Continuous measurement stream
    fn start_streaming(&mut self, callback: Box<dyn Fn(OpticalState) + Send>)
        -> HardwareResult<()>;

    /// Stop streaming
    fn stop_streaming(&mut self) -> HardwareResult<()>;

    /// Calibrate device
    fn calibrate(&mut self) -> HardwareResult<()>;

    /// Get device status
    fn status(&self) -> DeviceStatus;
}

/// Device status
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeviceStatus {
    /// Not connected
    Disconnected,
    /// Connected and ready
    Ready,
    /// Busy processing
    Busy,
    /// Error state
    Error(String),
    /// Calibrating
    Calibrating,
}

/// Trait for hardware that can convert quaternions to optical states
pub trait QuaternionToOptical {
    /// Convert quaternion to optical state
    fn quaternion_to_optical(&self, q: &Quaternion) -> OpticalState;

    /// Convert optical state to quaternion
    fn optical_to_quaternion(&self, state: &OpticalState) -> Quaternion;
}

/// Trait for hardware-in-the-loop testing
pub trait LoopbackTest {
    /// Send quaternion through hardware and receive back
    fn loopback(&mut self, q: &Quaternion) -> HardwareResult<Quaternion>;

    /// Measure round-trip error
    fn measure_error(&mut self, q: &Quaternion) -> HardwareResult<f64>;

    /// Run full loopback test suite
    fn run_test_suite(&mut self) -> HardwareResult<LoopbackTestResults>;
}

/// Results from loopback testing
#[derive(Clone, Debug)]
pub struct LoopbackTestResults {
    /// Number of tests run
    pub tests_run: usize,
    /// Number of tests passed
    pub tests_passed: usize,
    /// Average error (quaternion distance)
    pub avg_error: f64,
    /// Maximum error observed
    pub max_error: f64,
    /// Error standard deviation
    pub error_std: f64,
    /// Per-vertex results
    pub per_vertex_error: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_config_default() {
        let config = HardwareConfig::default();
        assert_eq!(config.device_id, "simulated");
    }

    #[test]
    fn test_device_status() {
        let status = DeviceStatus::Ready;
        assert_eq!(status, DeviceStatus::Ready);
    }
}

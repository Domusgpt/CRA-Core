//! TRACE integration for CSPM physical layer.
//!
//! Provides event logging, replay, and audit capabilities for CSPM links.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::crypto::ChainState;
use crate::modulation::{OpticalState, StokesVector};
use crate::quaternion::Quaternion;

/// TRACE event types for CSPM
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CspmEventType {
    /// Link initialization
    LinkInit,
    /// Link synchronization
    LinkSync,
    /// Packet transmission
    PacketTx,
    /// Packet reception
    PacketRx,
    /// Geometric snap correction applied
    ErrorSnap,
    /// Hash chain discontinuity detected
    ErrorChain,
    /// Tamper detection alert
    SecurityTamper,
    /// Periodic statistics
    StatsWindow,
}

impl std::fmt::Display for CspmEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CspmEventType::LinkInit => write!(f, "cspm.link.init"),
            CspmEventType::LinkSync => write!(f, "cspm.link.sync"),
            CspmEventType::PacketTx => write!(f, "cspm.packet.tx"),
            CspmEventType::PacketRx => write!(f, "cspm.packet.rx"),
            CspmEventType::ErrorSnap => write!(f, "cspm.error.snap"),
            CspmEventType::ErrorChain => write!(f, "cspm.error.chain"),
            CspmEventType::SecurityTamper => write!(f, "cspm.security.tamper"),
            CspmEventType::StatsWindow => write!(f, "cspm.stats.window"),
        }
    }
}

/// Lattice state for TRACE logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LatticeState {
    /// Current rotation quaternion
    pub rotation: [f64; 4],
    /// Chain hash (hex encoded)
    pub chain_hash: String,
    /// Chain depth
    pub chain_depth: u64,
}

impl LatticeState {
    pub fn from_chain_state(state: &ChainState) -> Self {
        Self {
            rotation: state.rotation.to_array(),
            chain_hash: hex::encode(&state.hash),
            chain_depth: state.depth,
        }
    }
}

/// Optical state for TRACE logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpticalStateLog {
    /// Stokes parameters [S1, S2, S3]
    pub stokes: [f64; 3],
    /// OAM mode
    pub oam_mode: i32,
    /// Power in dBm
    pub power_dbm: f64,
}

impl From<&OpticalState> for OpticalStateLog {
    fn from(state: &OpticalState) -> Self {
        Self {
            stokes: state.stokes.to_array(),
            oam_mode: state.oam_mode,
            power_dbm: state.power_dbm,
        }
    }
}

/// Data payload for TRACE logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataPayload {
    /// Vertex index
    pub vertex_index: usize,
    /// Encoded bits (binary string)
    pub bits: String,
    /// Payload size in bytes
    pub payload_bytes: usize,
}

/// Error correction info for TRACE logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorrectionInfo {
    /// Measured quaternion before snap
    pub measured: [f64; 4],
    /// Snapped quaternion
    pub snapped: [f64; 4],
    /// Correction angle in degrees
    pub correction_angle_deg: f64,
    /// Estimated SNR in dB
    pub snr_estimate_db: f64,
}

/// CSPM-specific event payload
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CspmPayload {
    /// Lattice state
    pub lattice_state: LatticeState,
    /// Optical state (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optical_state: Option<OpticalStateLog>,
    /// Data payload (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<DataPayload>,
    /// Correction info (optional, for error events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction: Option<CorrectionInfo>,
}

/// Event integrity information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventIntegrity {
    /// Hash of this event
    pub event_hash: String,
    /// Hash of previous event
    pub prev_hash: String,
}

/// Complete TRACE event for CSPM
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceEvent {
    /// TRACE protocol version
    pub trace_version: String,
    /// Event type
    pub event_type: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Sequence number
    pub sequence_number: u64,
    /// CSPM-specific payload
    pub cspm_payload: CspmPayload,
    /// Event integrity
    pub integrity: EventIntegrity,
}

impl TraceEvent {
    /// Serialize event to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize event to pretty JSON
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize event from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// CSPM TRACE event emitter
pub struct CspmTraceEmitter {
    /// Previous event hash
    prev_hash: String,
    /// Current sequence number
    sequence: u64,
    /// Event buffer
    events: Vec<TraceEvent>,
}

impl CspmTraceEmitter {
    /// Create a new trace emitter
    pub fn new() -> Self {
        Self {
            prev_hash: "genesis".to_string(),
            sequence: 0,
            events: Vec::new(),
        }
    }

    /// Emit a packet transmission event
    pub fn emit_packet_tx(
        &mut self,
        chain_state: &ChainState,
        optical: &OpticalState,
        vertex_index: usize,
        bits: u8,
        payload_bytes: usize,
    ) -> TraceEvent {
        let event = self.create_event(
            CspmEventType::PacketTx,
            CspmPayload {
                lattice_state: LatticeState::from_chain_state(chain_state),
                optical_state: Some(OpticalStateLog::from(optical)),
                data: Some(DataPayload {
                    vertex_index,
                    bits: format!("{:07b}", bits),
                    payload_bytes,
                }),
                correction: None,
            },
        );

        self.events.push(event.clone());
        event
    }

    /// Emit a packet reception event
    pub fn emit_packet_rx(
        &mut self,
        chain_state: &ChainState,
        vertex_index: usize,
        bits: u8,
        correction_distance: f64,
    ) -> TraceEvent {
        let correction = if correction_distance > 1e-6 {
            Some(CorrectionInfo {
                measured: [0.0; 4], // Would be filled with actual measured values
                snapped: [0.0; 4], // Would be filled with snapped values
                correction_angle_deg: correction_distance.asin().to_degrees(),
                snr_estimate_db: 20.0, // Placeholder
            })
        } else {
            None
        };

        let event = self.create_event(
            CspmEventType::PacketRx,
            CspmPayload {
                lattice_state: LatticeState::from_chain_state(chain_state),
                optical_state: None,
                data: Some(DataPayload {
                    vertex_index,
                    bits: format!("{:07b}", bits),
                    payload_bytes: 1,
                }),
                correction,
            },
        );

        self.events.push(event.clone());
        event
    }

    /// Emit an error snap event
    pub fn emit_error_snap(
        &mut self,
        chain_state: &ChainState,
        measured: &Quaternion,
        snapped: &Quaternion,
        vertex_index: usize,
        correction_distance: f64,
    ) -> TraceEvent {
        let event = self.create_event(
            CspmEventType::ErrorSnap,
            CspmPayload {
                lattice_state: LatticeState::from_chain_state(chain_state),
                optical_state: None,
                data: None,
                correction: Some(CorrectionInfo {
                    measured: measured.to_array(),
                    snapped: snapped.to_array(),
                    correction_angle_deg: correction_distance.asin().to_degrees(),
                    snr_estimate_db: -10.0 * correction_distance.log10(),
                }),
            },
        );

        self.events.push(event.clone());
        event
    }

    /// Emit a link initialization event
    pub fn emit_link_init(&mut self, chain_state: &ChainState) -> TraceEvent {
        let event = self.create_event(
            CspmEventType::LinkInit,
            CspmPayload {
                lattice_state: LatticeState::from_chain_state(chain_state),
                optical_state: None,
                data: None,
                correction: None,
            },
        );

        self.events.push(event.clone());
        event
    }

    /// Create an event with integrity info
    fn create_event(&mut self, event_type: CspmEventType, payload: CspmPayload) -> TraceEvent {
        use sha2::{Sha256, Digest};

        let event = TraceEvent {
            trace_version: "1.0".to_string(),
            event_type: event_type.to_string(),
            timestamp: Utc::now(),
            sequence_number: self.sequence,
            cspm_payload: payload,
            integrity: EventIntegrity {
                event_hash: String::new(), // Will be filled below
                prev_hash: self.prev_hash.clone(),
            },
        };

        // Compute event hash
        let event_json = serde_json::to_string(&event).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(event_json.as_bytes());
        let event_hash = hex::encode(hasher.finalize());

        let mut event = event;
        event.integrity.event_hash = event_hash.clone();

        // Update state
        self.prev_hash = event_hash;
        self.sequence += 1;

        event
    }

    /// Get all emitted events
    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    /// Export events as JSONL (one event per line)
    pub fn export_jsonl(&self) -> String {
        self.events
            .iter()
            .filter_map(|e| e.to_json().ok())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clear event buffer
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for CspmTraceEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let mut emitter = CspmTraceEmitter::new();
        let chain_state = ChainState::genesis(&[0u8; 32]);

        let event = emitter.emit_link_init(&chain_state);

        let json = event.to_json().unwrap();
        let parsed: TraceEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.event_type, "cspm.link.init");
        assert_eq!(parsed.sequence_number, 0);
    }

    #[test]
    fn test_event_chain() {
        let mut emitter = CspmTraceEmitter::new();
        let chain_state = ChainState::genesis(&[0u8; 32]);

        let event1 = emitter.emit_link_init(&chain_state);
        let event2 = emitter.emit_link_init(&chain_state);

        // Second event should reference first event's hash
        assert_eq!(event2.integrity.prev_hash, event1.integrity.event_hash);
    }

    #[test]
    fn test_packet_event() {
        let mut emitter = CspmTraceEmitter::new();
        let chain_state = ChainState::genesis(&[0u8; 32]);
        let optical = OpticalState::default();

        let event = emitter.emit_packet_tx(&chain_state, &optical, 42, 0b1010101, 128);

        assert_eq!(event.event_type, "cspm.packet.tx");
        assert!(event.cspm_payload.data.is_some());
        let data = event.cspm_payload.data.unwrap();
        assert_eq!(data.vertex_index, 42);
        assert_eq!(data.bits, "1010101");
    }

    #[test]
    fn test_jsonl_export() {
        let mut emitter = CspmTraceEmitter::new();
        let chain_state = ChainState::genesis(&[0u8; 32]);

        emitter.emit_link_init(&chain_state);
        emitter.emit_link_init(&chain_state);

        let jsonl = emitter.export_jsonl();
        let lines: Vec<&str> = jsonl.lines().collect();

        assert_eq!(lines.len(), 2);
    }
}

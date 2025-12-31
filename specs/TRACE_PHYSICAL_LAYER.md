# TRACE Physical Layer Integration — Specification

**Component of:** CSPM/1.0, TRACE/1.0
**Version:** 1.0

---

## Overview

This document specifies how **TRACE** (Telemetry & Replay Contract) extends into the physical layer for CSPM-enabled optical networks. TRACE events serve dual purposes:

1. **Audit Trail**: Standard event logging and replay
2. **Geometric Seed**: Hash chain driving lattice rotation

---

## 1. Extended TRACE Event Schema

### 1.1 Physical Layer Event Types

```json
{
  "event_types": {
    "cspm.link.init": "CSPM link initialization",
    "cspm.link.sync": "Lattice synchronization",
    "cspm.packet.tx": "Packet transmission",
    "cspm.packet.rx": "Packet reception",
    "cspm.error.snap": "Geometric snap correction",
    "cspm.error.chain": "Hash chain discontinuity",
    "cspm.security.tamper": "Tamper detection alert"
  }
}
```

### 1.2 CSPM Event Structure

```json
{
  "trace_version": "1.0",
  "event_type": "cspm.packet.tx",
  "timestamp": "2025-01-01T12:00:00.000000Z",
  "sequence_number": 42,

  "cspm_payload": {
    "lattice_state": {
      "rotation_quaternion": [0.5, 0.5, 0.5, 0.5],
      "chain_hash": "a3f2e8b1c9d4...",
      "chain_depth": 41
    },
    "optical_state": {
      "stokes": [0.707, 0.0, 0.707],
      "oam_mode": 3,
      "power_dbm": -5.2
    },
    "data": {
      "vertex_index": 73,
      "bits": "1001001",
      "payload_bytes": 128
    }
  },

  "integrity": {
    "event_hash": "sha256:...",
    "prev_hash": "sha256:...",
    "signature": "ed25519:..."
  }
}
```

---

## 2. Hash Chain Protocol

### 2.1 Chain Initialization

```
┌────────────────────────────────────────────────────────────┐
│                    CSPM LINK INIT                          │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  1. Genesis Event                                          │
│     ┌─────────────────────────────────────────────────┐   │
│     │ event_type: "cspm.link.init"                    │   │
│     │ genesis_hash: SHA256(shared_secret || timestamp) │   │
│     │ lattice_rotation: identity (1, 0, 0, 0)         │   │
│     └─────────────────────────────────────────────────┘   │
│                          │                                 │
│                          ▼                                 │
│  2. Both endpoints compute:                                │
│     R₀ = hash_to_quaternion(genesis_hash)                 │
│                          │                                 │
│                          ▼                                 │
│  3. Link synchronized, ready for data                      │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### 2.2 Chain Progression

For each packet n:

```python
def compute_chain_state(events: List[TraceEvent], n: int) -> ChainState:
    """
    Compute the chain state for packet n.
    """
    # Start with genesis
    current_hash = events[0].genesis_hash
    rotation = hash_to_quaternion(current_hash)

    # Accumulate all previous packet hashes
    for i in range(1, n):
        event = events[i]
        event_hash = sha256(event.serialize())

        # Chain the hashes
        current_hash = sha256(current_hash + event_hash)

        # Accumulate rotation
        delta_rotation = hash_to_quaternion(event_hash)
        rotation = quaternion_multiply(rotation, delta_rotation)
        rotation = normalize(rotation)

    return ChainState(
        depth=n,
        hash=current_hash,
        rotation=rotation
    )
```

### 2.3 Chain Verification

```rust
pub fn verify_chain_continuity(
    events: &[TraceEvent],
    expected_hash: &[u8; 32]
) -> Result<(), ChainError> {
    let mut running_hash = events[0].genesis_hash.clone();

    for event in events.iter().skip(1) {
        let event_hash = sha256(&event.serialize());

        // Verify chain linkage
        if event.prev_hash != running_hash {
            return Err(ChainError::Discontinuity {
                expected: running_hash,
                found: event.prev_hash.clone(),
            });
        }

        running_hash = sha256(&[&running_hash[..], &event_hash[..]].concat());
    }

    if running_hash != *expected_hash {
        return Err(ChainError::HashMismatch);
    }

    Ok(())
}
```

---

## 3. Synchronization Protocol

### 3.1 Initial Sync

```
    TRANSMITTER                         RECEIVER
        │                                   │
        │  ──── cspm.link.init ────────▶   │
        │       (genesis_hash)              │
        │                                   │
        │  ◀─── cspm.link.sync ─────────   │
        │       (ack + receiver_nonce)      │
        │                                   │
        │  ──── cspm.link.sync ────────▶   │
        │       (combined_hash)             │
        │                                   │
        │         [LINK ESTABLISHED]        │
        │                                   │
        │  ──── cspm.packet.tx ────────▶   │
        │       (packet 0)                  │
        │                                   │
```

### 3.2 Resynchronization

If the receiver detects a hash chain error:

```
    TRANSMITTER                         RECEIVER
        │                                   │
        │  ──── cspm.packet.tx ────────▶   │
        │       (packet N, corrupted)       │
        │                                   │
        │                          [CHAIN ERROR]
        │                                   │
        │  ◀─── cspm.error.chain ───────   │
        │       (last_good_seq: N-1)        │
        │                                   │
        │  ──── cspm.link.sync ────────▶   │
        │       (resync from N-1)           │
        │                                   │
        │  ──── cspm.packet.tx ────────▶   │
        │       (packet N, retry)           │
        │                                   │
```

### 3.3 Sync Event Schema

```json
{
  "trace_version": "1.0",
  "event_type": "cspm.link.sync",
  "timestamp": "2025-01-01T12:00:00.500Z",

  "sync_payload": {
    "sync_type": "resync",
    "last_good_sequence": 41,
    "chain_state": {
      "hash": "sha256:...",
      "rotation": [0.5, 0.5, 0.5, 0.5],
      "depth": 41
    },
    "proposed_rotation": [0.707, 0.0, 0.707, 0.0]
  }
}
```

---

## 4. Error Correction Events

### 4.1 Geometric Snap Logging

Every geometric correction generates a TRACE event:

```json
{
  "trace_version": "1.0",
  "event_type": "cspm.error.snap",
  "timestamp": "2025-01-01T12:00:01.123Z",

  "error_payload": {
    "sequence_number": 42,
    "measured_quaternion": [0.985, 0.105, 0.082, 0.015],
    "snapped_quaternion": [1.0, 0.0, 0.0, 0.0],
    "vertex_index": 0,
    "correction_angle_deg": 2.3,
    "snr_estimate_db": 18.5
  }
}
```

### 4.2 Error Statistics

```json
{
  "trace_version": "1.0",
  "event_type": "cspm.stats.window",
  "timestamp": "2025-01-01T12:01:00.000Z",

  "stats_payload": {
    "window_start": "2025-01-01T12:00:00.000Z",
    "window_packets": 1000,
    "corrections": {
      "total_snaps": 47,
      "avg_correction_angle_deg": 1.8,
      "max_correction_angle_deg": 12.3,
      "uncorrectable_errors": 0
    },
    "channel": {
      "avg_snr_db": 22.3,
      "min_snr_db": 15.1,
      "polarization_drift_deg": 0.5,
      "oam_crosstalk_db": -25.0
    }
  }
}
```

---

## 5. Security Events

### 5.1 Tamper Detection

```json
{
  "trace_version": "1.0",
  "event_type": "cspm.security.tamper",
  "timestamp": "2025-01-01T12:00:05.000Z",
  "severity": "critical",

  "security_payload": {
    "detection_type": "geometric_anomaly",
    "description": "Received quaternion outside all Voronoi cells",
    "evidence": {
      "sequence_number": 107,
      "measured_quaternion": [0.3, 0.3, 0.3, 0.85],
      "nearest_vertex_distance": 0.42,
      "expected_max_distance": 0.31,
      "chain_state_valid": true
    },
    "action_taken": "packet_dropped"
  }
}
```

### 5.2 Interception Indicators

```json
{
  "trace_version": "1.0",
  "event_type": "cspm.security.anomaly",
  "timestamp": "2025-01-01T12:00:10.000Z",

  "anomaly_payload": {
    "indicator_type": "statistical",
    "description": "Error rate spike suggests active probing",
    "metrics": {
      "baseline_error_rate": 0.001,
      "current_error_rate": 0.15,
      "window_size_packets": 100,
      "confidence": 0.99
    },
    "recommendation": "Consider key rotation"
  }
}
```

---

## 6. Replay and Verification

### 6.1 TRACE Replay for CSPM

```rust
pub struct CspmReplayEngine {
    vertices: Vec<Vertex>,
    gray_mapper: GrayCodeMapper,
}

impl CspmReplayEngine {
    pub fn replay(&self, events: &[TraceEvent]) -> ReplayResult {
        let mut chain_state = ChainState::genesis(&events[0]);
        let mut decoded_data = Vec::new();
        let mut errors = Vec::new();

        for event in events.iter().skip(1) {
            match event.event_type.as_str() {
                "cspm.packet.tx" => {
                    // Verify chain continuity
                    let expected_hash = self.compute_next_hash(&chain_state, event);
                    if event.integrity.prev_hash != chain_state.hash {
                        errors.push(ReplayError::ChainBreak(event.sequence_number));
                    }

                    // Verify quaternion matches vertex
                    let payload = &event.cspm_payload;
                    let vertex = &self.vertices[payload.data.vertex_index];
                    let expected_q = chain_state.rotation.rotate(vertex);

                    if !quaternion_approx_eq(&expected_q, &payload.lattice_state.rotation_quaternion) {
                        errors.push(ReplayError::QuaternionMismatch(event.sequence_number));
                    }

                    // Decode data
                    let bits = self.gray_mapper.decode(payload.data.vertex_index)?;
                    decoded_data.push(bits);

                    // Advance chain
                    chain_state = chain_state.advance(event);
                }

                "cspm.error.snap" => {
                    // Log correction but don't alter chain
                    // Chain is based on transmitted data, not received
                }

                _ => {}
            }
        }

        ReplayResult {
            decoded_data,
            errors,
            final_chain_state: chain_state,
        }
    }
}
```

### 6.2 Differential Replay

Compare two TRACE logs to identify divergence:

```rust
pub fn diff_traces(
    trace_a: &[TraceEvent],
    trace_b: &[TraceEvent]
) -> Vec<TraceDiff> {
    let mut diffs = Vec::new();

    // Align by sequence number
    for (a, b) in trace_a.iter().zip(trace_b.iter()) {
        if a.sequence_number != b.sequence_number {
            diffs.push(TraceDiff::SequenceMismatch);
            continue;
        }

        if a.integrity.event_hash != b.integrity.event_hash {
            diffs.push(TraceDiff::ContentDiff {
                seq: a.sequence_number,
                field_diffs: compute_field_diffs(a, b),
            });
        }

        // Compare CSPM-specific fields
        if let (Some(cspm_a), Some(cspm_b)) = (&a.cspm_payload, &b.cspm_payload) {
            if cspm_a.lattice_state.rotation_quaternion != cspm_b.lattice_state.rotation_quaternion {
                diffs.push(TraceDiff::RotationDiff {
                    seq: a.sequence_number,
                    rotation_a: cspm_a.lattice_state.rotation_quaternion.clone(),
                    rotation_b: cspm_b.lattice_state.rotation_quaternion.clone(),
                });
            }
        }
    }

    diffs
}
```

---

## 7. CARP Policy Integration

### 7.1 Physical Layer Policies

```json
{
  "carp_version": "1.0",
  "policy_type": "cspm_physical_layer",
  "policy_id": "cspm-secure-fiber",

  "constraints": {
    "min_snr_db": 15.0,
    "max_oam_modes": 16,
    "hash_algorithm": "SHA-256",
    "chain_verification": "strict",
    "resync_max_attempts": 3
  },

  "security": {
    "tamper_detection": true,
    "anomaly_threshold_sigma": 3.0,
    "key_rotation_interval_packets": 1000000
  },

  "logging": {
    "log_every_packet": false,
    "log_corrections": true,
    "log_stats_interval_sec": 60,
    "archive_retention_days": 90
  }
}
```

### 7.2 CARP Resolution for CSPM Operations

```json
{
  "carp_version": "1.0",
  "operation": "resolve",
  "context": "cspm_transmission",
  "task": {
    "goal": "Transmit classified data over CSPM link",
    "data_classification": "secret",
    "destination": "remote_site_alpha"
  }
}
```

Resolution:
```json
{
  "resolution": {
    "decision": "allow_with_constraints",
    "context_blocks": ["cspm_policy", "classification_rules"],
    "allowed_actions": ["cspm.packet.tx"],
    "constraints": {
      "require_chain_verification": true,
      "require_tamper_logging": true,
      "min_chain_depth_before_data": 10,
      "mandatory_key_rotation_packets": 10000
    },
    "audit": {
      "log_level": "verbose",
      "real_time_monitoring": true
    }
  }
}
```

---

## 8. Implementation API

### 8.1 TraceEmitter for CSPM

```rust
pub struct CspmTraceEmitter {
    chain_state: ChainState,
    policy: CspmPolicy,
    output: Box<dyn TraceOutput>,
}

impl CspmTraceEmitter {
    pub fn emit_packet_tx(
        &mut self,
        vertex_index: usize,
        optical_state: &OpticalState,
        payload_size: usize,
    ) -> Result<(), TraceError> {
        let event = TraceEvent {
            trace_version: "1.0".to_string(),
            event_type: "cspm.packet.tx".to_string(),
            timestamp: Utc::now(),
            sequence_number: self.chain_state.depth,

            cspm_payload: Some(CspmPayload {
                lattice_state: LatticeState {
                    rotation_quaternion: self.chain_state.rotation.to_array(),
                    chain_hash: hex::encode(&self.chain_state.hash),
                    chain_depth: self.chain_state.depth,
                },
                optical_state: optical_state.clone(),
                data: DataPayload {
                    vertex_index,
                    bits: format!("{:07b}", self.gray_mapper.decode(vertex_index)?),
                    payload_bytes: payload_size,
                },
            }),

            integrity: self.compute_integrity(),
        };

        // Advance chain state
        let event_hash = sha256(&event.serialize());
        self.chain_state = self.chain_state.advance(&event_hash);

        // Emit event
        self.output.emit(&event)?;

        Ok(())
    }

    pub fn emit_error_snap(
        &mut self,
        sequence: u64,
        measured: Quaternion,
        snapped: Quaternion,
        vertex: usize,
    ) -> Result<(), TraceError> {
        let correction_angle = measured.angular_distance(&snapped).to_degrees();

        let event = TraceEvent {
            event_type: "cspm.error.snap".to_string(),
            // ... fill in fields ...
        };

        self.output.emit(&event)?;
        Ok(())
    }
}
```

### 8.2 TraceVerifier for CSPM

```rust
pub struct CspmTraceVerifier {
    vertices: Vec<Vertex>,
    policy: CspmPolicy,
}

impl CspmTraceVerifier {
    pub fn verify_event(&self, event: &TraceEvent, expected_state: &ChainState) -> VerifyResult {
        let mut issues = Vec::new();

        // Verify chain linkage
        if event.integrity.prev_hash != expected_state.hash {
            issues.push(VerifyIssue::ChainBreak);
        }

        // Verify CSPM payload if present
        if let Some(cspm) = &event.cspm_payload {
            // Verify rotation matches chain
            let expected_rotation = expected_state.rotation.to_array();
            if !arrays_approx_eq(&cspm.lattice_state.rotation_quaternion, &expected_rotation) {
                issues.push(VerifyIssue::RotationMismatch);
            }

            // Verify vertex exists
            if cspm.data.vertex_index >= 120 {
                issues.push(VerifyIssue::InvalidVertex);
            }
        }

        if issues.is_empty() {
            VerifyResult::Valid
        } else {
            VerifyResult::Invalid(issues)
        }
    }
}
```

---

## 9. Storage and Archival

### 9.1 CSPM TRACE Log Format

```
cspm-trace-2025-01-01.jsonl
─────────────────────────
Line 1: {"event_type": "cspm.link.init", ...}
Line 2: {"event_type": "cspm.packet.tx", "sequence_number": 0, ...}
Line 3: {"event_type": "cspm.packet.tx", "sequence_number": 1, ...}
...
Line N: {"event_type": "cspm.stats.window", ...}
```

### 9.2 Compact Binary Format

For high-throughput logging:

```rust
pub struct CompactCspmEvent {
    timestamp_us: u64,           // 8 bytes
    sequence: u64,               // 8 bytes
    event_type: u8,              // 1 byte
    vertex_index: u8,            // 1 byte
    rotation: [f16; 4],          // 8 bytes (half precision)
    stokes: [f16; 3],            // 6 bytes
    oam_mode: i8,                // 1 byte
    correction_angle_cdeg: u8,   // 1 byte (0.01° resolution)
    chain_hash_prefix: [u8; 8],  // 8 bytes
    // Total: 42 bytes per event
}
```

At 10 Gbps with 7 bits/symbol:
- ~1.43 billion symbols/second
- ~60 GB/second raw trace (full)
- ~6 GB/second compact trace

---

## 10. Compliance and Audit

### 10.1 Audit Trail Completeness

CSPM TRACE logs provide:

1. **Cryptographic Continuity**: Every packet linked by hash chain
2. **Physical State History**: Complete optical state at each point
3. **Error Documentation**: Every geometric correction logged
4. **Security Events**: All anomalies and tamper attempts recorded
5. **Replayability**: Sufficient data to reconstruct entire session

### 10.2 Regulatory Compliance

CSPM TRACE supports:
- **Financial (MiFID II)**: Microsecond timestamps, full audit trail
- **Healthcare (HIPAA)**: Encryption verification, access logging
- **Government (FISMA)**: Tamper evidence, chain-of-custody proof

---

*End of TRACE Physical Layer Integration Specification*

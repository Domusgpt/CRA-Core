//! Synchronization protocol for CSPM.
//!
//! This module provides frame synchronization, sequence numbering,
//! hash chain checkpointing, and resync capabilities for reliable
//! CSPM communication.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        CSPM Frame                          │
//! ├──────────┬──────────┬─────────────────┬──────────┬─────────┤
//! │ Preamble │ Header   │     Payload     │Checkpoint│  Guard  │
//! │(8 symb.) │(4 symb.) │  (N symbols)    │ (2 symb.)│(2 symb.)│
//! └──────────┴──────────┴─────────────────┴──────────┴─────────┘
//! ```
//!
//! ## Features
//!
//! - **Preamble**: Unique word for frame boundary detection
//! - **Sequence numbers**: Frame ordering and gap detection
//! - **Hash chain checkpoints**: Periodic state snapshots for resync
//! - **Recovery protocol**: Re-establish sync after packet loss

pub mod preamble;
pub mod framing;
pub mod checkpoint;
pub mod recovery;

pub use preamble::{Preamble, PreambleDetector, PreambleConfig};
pub use framing::{Frame, FrameHeader, FrameBuilder, FrameParser};
pub use checkpoint::{Checkpoint, CheckpointManager, CheckpointConfig};
pub use recovery::{SyncState, SyncRecovery, RecoveryConfig};

use crate::quaternion::Quaternion;
use crate::crypto::ChainState;

/// Frame synchronization status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncStatus {
    /// No synchronization - searching for preamble
    Searching,
    /// Preamble detected, acquiring frame timing
    Acquiring,
    /// Synchronized and tracking
    Synchronized,
    /// Lost sync, attempting recovery
    Recovering,
}

/// Synchronization event for tracking
#[derive(Clone, Debug)]
pub enum SyncEvent {
    /// Preamble detected at position
    PreambleFound { position: usize },
    /// Frame successfully received
    FrameReceived { sequence: u64 },
    /// Frame sequence gap detected
    SequenceGap { expected: u64, received: u64 },
    /// Sync lost
    SyncLost { last_sequence: u64 },
    /// Sync recovered using checkpoint
    SyncRecovered { checkpoint_sequence: u64, current_sequence: u64 },
}

/// Statistics for sync protocol
#[derive(Clone, Debug, Default)]
pub struct SyncStats {
    /// Total frames received
    pub frames_received: u64,
    /// Frames with errors
    pub frames_errored: u64,
    /// Sequence gaps detected
    pub sequence_gaps: u64,
    /// Successful recoveries
    pub recoveries: u64,
    /// Current sync status
    pub status: SyncStatus,
    /// Last valid sequence number
    pub last_sequence: u64,
}

impl Default for SyncStatus {
    fn default() -> Self {
        SyncStatus::Searching
    }
}

/// Configuration for the sync protocol
#[derive(Clone, Debug)]
pub struct SyncConfig {
    /// Preamble configuration
    pub preamble: PreambleConfig,
    /// Checkpoint configuration
    pub checkpoint: CheckpointConfig,
    /// Recovery configuration
    pub recovery: RecoveryConfig,
    /// Maximum payload symbols per frame
    pub max_payload_symbols: usize,
    /// Enable sequence number validation
    pub enable_sequence_check: bool,
    /// Maximum allowed sequence gap before resync
    pub max_sequence_gap: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            preamble: PreambleConfig::default(),
            checkpoint: CheckpointConfig::default(),
            recovery: RecoveryConfig::default(),
            max_payload_symbols: 256,
            enable_sequence_check: true,
            max_sequence_gap: 16,
        }
    }
}

/// Main synchronization manager
pub struct SyncManager {
    config: SyncConfig,
    preamble_detector: PreambleDetector,
    checkpoint_manager: CheckpointManager,
    recovery: SyncRecovery,
    stats: SyncStats,
}

impl SyncManager {
    /// Create new sync manager
    pub fn new(config: SyncConfig) -> Self {
        Self {
            preamble_detector: PreambleDetector::new(config.preamble.clone()),
            checkpoint_manager: CheckpointManager::new(config.checkpoint.clone()),
            recovery: SyncRecovery::new(config.recovery.clone()),
            stats: SyncStats::default(),
            config,
        }
    }

    /// Get current sync status
    pub fn status(&self) -> SyncStatus {
        self.stats.status
    }

    /// Get statistics
    pub fn stats(&self) -> &SyncStats {
        &self.stats
    }

    /// Process incoming symbol stream
    pub fn process_symbols(&mut self, symbols: &[Quaternion]) -> Vec<SyncEvent> {
        let mut events = Vec::new();

        match self.stats.status {
            SyncStatus::Searching => {
                // Look for preamble
                if let Some(pos) = self.preamble_detector.detect(symbols) {
                    events.push(SyncEvent::PreambleFound { position: pos });
                    self.stats.status = SyncStatus::Acquiring;
                }
            }
            SyncStatus::Acquiring => {
                // Try to parse frame header
                // If successful, transition to Synchronized
                self.stats.status = SyncStatus::Synchronized;
            }
            SyncStatus::Synchronized => {
                // Normal operation - parse frames
            }
            SyncStatus::Recovering => {
                // Attempt to recover using checkpoints
            }
        }

        events
    }

    /// Record a checkpoint
    pub fn create_checkpoint(&mut self, chain_state: &ChainState) {
        self.checkpoint_manager.record(chain_state);
    }

    /// Attempt recovery from checkpoint
    pub fn recover_from_checkpoint(&mut self, target_sequence: u64) -> Option<ChainState> {
        let checkpoint = self.checkpoint_manager.find_nearest(target_sequence)?;
        self.stats.recoveries += 1;
        Some(checkpoint.chain_state.clone())
    }

    /// Reset sync state
    pub fn reset(&mut self) {
        self.stats = SyncStats::default();
        self.preamble_detector.reset();
        self.recovery.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_config_default() {
        let config = SyncConfig::default();
        assert_eq!(config.max_payload_symbols, 256);
        assert!(config.enable_sequence_check);
    }

    #[test]
    fn test_sync_manager_initial_state() {
        let manager = SyncManager::new(SyncConfig::default());
        assert_eq!(manager.status(), SyncStatus::Searching);
    }
}

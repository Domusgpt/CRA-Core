//! Resync protocol for recovering from packet loss.
//!
//! When packets are lost, the hash chain becomes desynchronized.
//! This module provides mechanisms to recover:
//!
//! 1. **Checkpoint recovery**: Find nearest checkpoint, replay chain
//! 2. **Blind resync**: Search for preamble, estimate chain offset
//! 3. **Negotiated resync**: Request retransmission of checkpoint
//!
//! ## Recovery States
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Recovery State Machine                    │
//! └─────────────────────────────────────────────────────────────┘
//!
//!   ┌──────────┐      loss      ┌───────────┐
//!   │  Normal  │ ─────────────> │ Detecting │
//!   └──────────┘                └───────────┘
//!        ▲                           │
//!        │                           ▼
//!        │                    ┌─────────────┐
//!        │    resync ok       │  Searching  │
//!        └─────────────────── │ Checkpoint  │
//!        │                    └─────────────┘
//!        │                           │
//!        │                           ▼ not found
//!        │                    ┌─────────────┐
//!        │    resync ok       │   Blind     │
//!        └─────────────────── │   Resync    │
//!                             └─────────────┘
//! ```

use crate::quaternion::Quaternion;
use crate::crypto::{ChainState, HashChain};
use super::checkpoint::{Checkpoint, CheckpointManager};
use super::framing::FrameParseError;

/// Recovery configuration
#[derive(Clone, Debug)]
pub struct RecoveryConfig {
    /// Maximum frames to search for resync
    pub max_search_frames: usize,
    /// Number of consecutive good frames to confirm sync
    pub sync_confirm_count: usize,
    /// Enable blind resync (without checkpoints)
    pub enable_blind_resync: bool,
    /// Maximum replay distance from checkpoint
    pub max_replay_distance: u64,
    /// Timeout for recovery attempt (in frames)
    pub recovery_timeout: usize,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_search_frames: 1000,
            sync_confirm_count: 3,
            enable_blind_resync: true,
            max_replay_distance: 256,
            recovery_timeout: 500,
        }
    }
}

/// Current synchronization state
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncState {
    /// Normal operation
    Synchronized,
    /// Loss detected, searching for recovery
    LossDetected,
    /// Searching for checkpoint
    SearchingCheckpoint,
    /// Attempting blind resync
    BlindResync,
    /// Replaying chain from checkpoint
    Replaying,
    /// Recovery failed
    Failed,
}

/// Result of a recovery attempt
#[derive(Clone, Debug)]
pub enum RecoveryResult {
    /// Successfully recovered
    Success {
        /// Sequence number after recovery
        sequence: u64,
        /// Chain state after recovery
        chain_state: ChainState,
        /// Frames lost during outage
        frames_lost: u64,
    },
    /// Recovery in progress
    InProgress {
        /// Current recovery state
        state: SyncState,
        /// Frames searched
        frames_searched: usize,
    },
    /// Recovery failed
    Failed {
        /// Reason for failure
        reason: RecoveryFailure,
    },
}

/// Reason for recovery failure
#[derive(Clone, Debug)]
pub enum RecoveryFailure {
    /// No checkpoint found
    NoCheckpoint,
    /// Checkpoint too old
    CheckpointTooOld { age: u64 },
    /// Replay failed
    ReplayFailed,
    /// Timeout
    Timeout,
    /// Chain verification failed
    VerificationFailed,
}

/// Synchronization recovery manager
pub struct SyncRecovery {
    config: RecoveryConfig,
    state: SyncState,
    /// Last known good sequence
    last_good_sequence: u64,
    /// Last known good chain state
    last_good_state: Option<ChainState>,
    /// Frames since loss detected
    frames_since_loss: usize,
    /// Good frames counter for sync confirm
    consecutive_good: usize,
    /// Recovery attempts
    recovery_attempts: u32,
}

impl SyncRecovery {
    /// Create new recovery manager
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            state: SyncState::Synchronized,
            last_good_sequence: 0,
            last_good_state: None,
            frames_since_loss: 0,
            consecutive_good: 0,
            recovery_attempts: 0,
        }
    }

    /// Get current state
    pub fn state(&self) -> SyncState {
        self.state
    }

    /// Report successful frame reception
    pub fn report_success(&mut self, sequence: u64, chain_state: &ChainState) {
        self.last_good_sequence = sequence;
        self.last_good_state = Some(chain_state.clone());

        match self.state {
            SyncState::Synchronized => {
                // Normal operation
                self.consecutive_good += 1;
            }
            SyncState::Replaying |
            SyncState::BlindResync |
            SyncState::SearchingCheckpoint => {
                // Confirming recovery
                self.consecutive_good += 1;
                if self.consecutive_good >= self.config.sync_confirm_count {
                    self.state = SyncState::Synchronized;
                    self.recovery_attempts = 0;
                }
            }
            _ => {
                self.consecutive_good = 1;
            }
        }
    }

    /// Report frame error or loss
    pub fn report_loss(&mut self, _error: &FrameParseError) {
        self.consecutive_good = 0;

        if self.state == SyncState::Synchronized {
            self.state = SyncState::LossDetected;
            self.frames_since_loss = 1;
        } else {
            self.frames_since_loss += 1;
        }

        // Check timeout
        if self.frames_since_loss > self.config.recovery_timeout {
            self.state = SyncState::Failed;
        }
    }

    /// Attempt recovery using checkpoint
    pub fn recover_from_checkpoint(
        &mut self,
        target_sequence: u64,
        checkpoints: &CheckpointManager,
        chain: &mut HashChain,
    ) -> RecoveryResult {
        self.state = SyncState::SearchingCheckpoint;
        self.recovery_attempts += 1;

        // Find nearest checkpoint
        let checkpoint = match checkpoints.find_nearest(target_sequence) {
            Some(cp) => cp,
            None => {
                if self.config.enable_blind_resync {
                    self.state = SyncState::BlindResync;
                    return RecoveryResult::InProgress {
                        state: SyncState::BlindResync,
                        frames_searched: self.frames_since_loss,
                    };
                } else {
                    self.state = SyncState::Failed;
                    return RecoveryResult::Failed {
                        reason: RecoveryFailure::NoCheckpoint,
                    };
                }
            }
        };

        // Check checkpoint age
        let distance = target_sequence.saturating_sub(checkpoint.sequence);
        if distance > self.config.max_replay_distance {
            self.state = SyncState::Failed;
            return RecoveryResult::Failed {
                reason: RecoveryFailure::CheckpointTooOld { age: distance },
            };
        }

        // Replay from checkpoint
        self.state = SyncState::Replaying;
        chain.resync(checkpoint.chain_state.clone());

        RecoveryResult::Success {
            sequence: checkpoint.sequence,
            chain_state: checkpoint.chain_state.clone(),
            frames_lost: self.last_good_sequence.saturating_sub(checkpoint.sequence),
        }
    }

    /// Attempt blind resync by searching for valid frame
    pub fn blind_resync(
        &mut self,
        received_symbols: &[Quaternion],
        expected_chain_depth: u64,
    ) -> RecoveryResult {
        self.state = SyncState::BlindResync;
        self.frames_since_loss += 1;

        // Search logic would go here
        // For now, return in-progress

        if self.frames_since_loss > self.config.max_search_frames {
            self.state = SyncState::Failed;
            return RecoveryResult::Failed {
                reason: RecoveryFailure::Timeout,
            };
        }

        RecoveryResult::InProgress {
            state: SyncState::BlindResync,
            frames_searched: self.frames_since_loss,
        }
    }

    /// Check if we're in a state that requires recovery
    pub fn needs_recovery(&self) -> bool {
        matches!(
            self.state,
            SyncState::LossDetected |
            SyncState::SearchingCheckpoint |
            SyncState::BlindResync |
            SyncState::Replaying
        )
    }

    /// Check if recovery has failed
    pub fn has_failed(&self) -> bool {
        self.state == SyncState::Failed
    }

    /// Reset recovery state
    pub fn reset(&mut self) {
        self.state = SyncState::Synchronized;
        self.frames_since_loss = 0;
        self.consecutive_good = 0;
        self.recovery_attempts = 0;
    }

    /// Get recovery statistics
    pub fn stats(&self) -> RecoveryStats {
        RecoveryStats {
            state: self.state,
            last_good_sequence: self.last_good_sequence,
            frames_since_loss: self.frames_since_loss,
            recovery_attempts: self.recovery_attempts,
            consecutive_good: self.consecutive_good,
        }
    }
}

/// Recovery statistics
#[derive(Clone, Debug)]
pub struct RecoveryStats {
    /// Current state
    pub state: SyncState,
    /// Last good sequence number
    pub last_good_sequence: u64,
    /// Frames since loss
    pub frames_since_loss: usize,
    /// Total recovery attempts
    pub recovery_attempts: u32,
    /// Consecutive good frames
    pub consecutive_good: usize,
}

/// Chain replayer for recovery from checkpoint
pub struct ChainReplayer {
    /// Starting chain state
    start_state: ChainState,
    /// Current position
    current_position: u64,
    /// Target position
    target_position: u64,
}

impl ChainReplayer {
    /// Create new replayer from checkpoint
    pub fn new(checkpoint: &Checkpoint, target: u64) -> Self {
        Self {
            start_state: checkpoint.chain_state.clone(),
            current_position: checkpoint.sequence,
            target_position: target,
        }
    }

    /// Replay one step with packet data
    pub fn replay_step(&mut self, packet_data: &[u8]) -> ChainState {
        let new_state = self.start_state.advance(packet_data);
        self.start_state = new_state.clone();
        self.current_position += 1;
        new_state
    }

    /// Check if replay is complete
    pub fn is_complete(&self) -> bool {
        self.current_position >= self.target_position
    }

    /// Get current state
    pub fn current_state(&self) -> &ChainState {
        &self.start_state
    }

    /// Get progress
    pub fn progress(&self) -> f64 {
        if self.target_position <= self.current_position {
            return 1.0;
        }
        let total = self.target_position - self.current_position;
        let done = self.current_position - self.target_position;
        done as f64 / total as f64
    }
}

/// Request for resync (for bidirectional links)
#[derive(Clone, Debug)]
pub struct ResyncRequest {
    /// Requested checkpoint sequence
    pub checkpoint_sequence: u64,
    /// Hash prefix for verification
    pub hash_prefix: [u8; 4],
    /// Current receiver sequence
    pub current_sequence: u64,
}

impl ResyncRequest {
    /// Create resync request
    pub fn new(checkpoint: &Checkpoint, current: u64) -> Self {
        Self {
            checkpoint_sequence: checkpoint.sequence,
            hash_prefix: checkpoint.hash_prefix(),
            current_sequence: current,
        }
    }

    /// Encode to bytes for transmission
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(20);
        bytes.extend(&self.checkpoint_sequence.to_le_bytes());
        bytes.extend(&self.hash_prefix);
        bytes.extend(&self.current_sequence.to_le_bytes());
        bytes
    }

    /// Decode from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 20 {
            return None;
        }

        let checkpoint_sequence = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let hash_prefix = [bytes[8], bytes[9], bytes[10], bytes[11]];
        let current_sequence = u64::from_le_bytes(bytes[12..20].try_into().ok()?);

        Some(Self {
            checkpoint_sequence,
            hash_prefix,
            current_sequence,
        })
    }
}

/// Response to resync request
#[derive(Clone, Debug)]
pub struct ResyncResponse {
    /// Checkpoint data
    pub checkpoint_state: ChainState,
    /// Sequence at checkpoint
    pub sequence: u64,
    /// Success flag
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::sha256;
    use super::super::checkpoint::CheckpointConfig;

    fn make_chain_state(depth: u64) -> ChainState {
        let hash = sha256(&depth.to_le_bytes());
        ChainState {
            hash,
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            depth,
        }
    }

    #[test]
    fn test_recovery_initial_state() {
        let recovery = SyncRecovery::new(RecoveryConfig::default());
        assert_eq!(recovery.state(), SyncState::Synchronized);
        assert!(!recovery.needs_recovery());
    }

    #[test]
    fn test_report_success() {
        let mut recovery = SyncRecovery::new(RecoveryConfig::default());
        let state = make_chain_state(10);

        recovery.report_success(10, &state);

        assert_eq!(recovery.state(), SyncState::Synchronized);
        assert_eq!(recovery.last_good_sequence, 10);
    }

    #[test]
    fn test_report_loss() {
        let mut recovery = SyncRecovery::new(RecoveryConfig::default());

        recovery.report_loss(&FrameParseError::PayloadTruncated);

        assert_eq!(recovery.state(), SyncState::LossDetected);
        assert!(recovery.needs_recovery());
    }

    #[test]
    fn test_recovery_from_checkpoint() {
        let config = RecoveryConfig::default();
        let mut recovery = SyncRecovery::new(config);

        // Set up checkpoint manager
        let mut checkpoints = CheckpointManager::new(CheckpointConfig::default());
        let state = make_chain_state(100);
        checkpoints.set_frame_count(100);
        checkpoints.force_checkpoint(&state, 1);

        // Create hash chain
        let mut chain = HashChain::from_secret(b"test");

        // Simulate loss
        recovery.report_loss(&FrameParseError::SequenceGap {
            expected: 100,
            received: 110,
        });

        // Attempt recovery
        let result = recovery.recover_from_checkpoint(105, &checkpoints, &mut chain);

        match result {
            RecoveryResult::Success { sequence, .. } => {
                assert_eq!(sequence, 100);
            }
            _ => panic!("Expected success"),
        }
    }

    #[test]
    fn test_sync_confirm() {
        let config = RecoveryConfig {
            sync_confirm_count: 3,
            ..Default::default()
        };
        let mut recovery = SyncRecovery::new(config);

        // Simulate loss and recovery
        recovery.report_loss(&FrameParseError::PayloadTruncated);
        recovery.state = SyncState::Replaying;

        // Report successes
        for i in 0..3 {
            let state = make_chain_state(i);
            recovery.report_success(i, &state);
        }

        // Should be synchronized after 3 good frames
        assert_eq!(recovery.state(), SyncState::Synchronized);
    }

    #[test]
    fn test_recovery_timeout() {
        let config = RecoveryConfig {
            recovery_timeout: 10,
            ..Default::default()
        };
        let mut recovery = SyncRecovery::new(config);

        // Report many losses
        for _ in 0..15 {
            recovery.report_loss(&FrameParseError::PayloadTruncated);
        }

        assert!(recovery.has_failed());
    }

    #[test]
    fn test_chain_replayer() {
        let state = make_chain_state(100);
        let checkpoint = Checkpoint::new(100, state, 1);

        let mut replayer = ChainReplayer::new(&checkpoint, 105);

        assert!(!replayer.is_complete());

        // Replay 5 steps
        for i in 0..5 {
            let data = format!("packet_{}", i);
            replayer.replay_step(data.as_bytes());
        }

        assert!(replayer.is_complete());
    }

    #[test]
    fn test_resync_request_encoding() {
        let state = make_chain_state(100);
        let checkpoint = Checkpoint::new(100, state, 1);

        let request = ResyncRequest::new(&checkpoint, 110);
        let bytes = request.to_bytes();
        let decoded = ResyncRequest::from_bytes(&bytes).unwrap();

        assert_eq!(request.checkpoint_sequence, decoded.checkpoint_sequence);
        assert_eq!(request.hash_prefix, decoded.hash_prefix);
        assert_eq!(request.current_sequence, decoded.current_sequence);
    }

    #[test]
    fn test_recovery_stats() {
        let mut recovery = SyncRecovery::new(RecoveryConfig::default());

        let state = make_chain_state(50);
        recovery.report_success(50, &state);
        recovery.report_loss(&FrameParseError::TooShort);

        let stats = recovery.stats();
        assert_eq!(stats.last_good_sequence, 50);
        assert_eq!(stats.frames_since_loss, 1);
    }
}

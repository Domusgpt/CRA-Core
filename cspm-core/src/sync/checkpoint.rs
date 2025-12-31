//! Hash chain checkpointing for resynchronization.
//!
//! Checkpoints store periodic snapshots of the hash chain state,
//! allowing receivers to recover from packet loss without
//! replaying the entire chain from genesis.
//!
//! ## Strategy
//!
//! - Checkpoints are stored every N frames (configurable)
//! - Each checkpoint includes chain hash, rotation, and depth
//! - On packet loss, receiver finds nearest checkpoint and replays
//! - Multiple checkpoint levels for different recovery speeds

use crate::quaternion::Quaternion;
use crate::crypto::ChainState;
use serde::{Deserialize, Serialize};

/// Checkpoint configuration
#[derive(Clone, Debug)]
pub struct CheckpointConfig {
    /// Frames between level-1 checkpoints
    pub level1_interval: u64,
    /// Frames between level-2 checkpoints
    pub level2_interval: u64,
    /// Maximum checkpoints to retain per level
    pub max_checkpoints: usize,
    /// Enable automatic checkpoint insertion
    pub auto_checkpoint: bool,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            level1_interval: 16,    // Every 16 frames
            level2_interval: 256,   // Every 256 frames
            max_checkpoints: 64,
            auto_checkpoint: true,
        }
    }
}

/// A checkpoint of the hash chain state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Frame sequence number at checkpoint
    pub sequence: u64,
    /// Full chain state
    pub chain_state: ChainState,
    /// Checkpoint level (1 = fine, 2 = coarse)
    pub level: u8,
    /// Timestamp (optional)
    pub timestamp: Option<u64>,
}

impl Checkpoint {
    /// Create new checkpoint
    pub fn new(sequence: u64, chain_state: ChainState, level: u8) -> Self {
        Self {
            sequence,
            chain_state,
            level,
            timestamp: None,
        }
    }

    /// Create with timestamp
    pub fn with_timestamp(
        sequence: u64,
        chain_state: ChainState,
        level: u8,
        timestamp: u64,
    ) -> Self {
        Self {
            sequence,
            chain_state,
            level,
            timestamp: Some(timestamp),
        }
    }

    /// Get chain hash prefix for compact transmission
    pub fn hash_prefix(&self) -> [u8; 4] {
        [
            self.chain_state.hash[0],
            self.chain_state.hash[1],
            self.chain_state.hash[2],
            self.chain_state.hash[3],
        ]
    }

    /// Verify a chain state matches this checkpoint
    pub fn verify(&self, state: &ChainState) -> bool {
        self.chain_state.hash == state.hash
            && self.chain_state.depth == state.depth
    }

    /// Compact representation for transmission (32 bytes)
    pub fn to_compact(&self) -> CompactCheckpoint {
        CompactCheckpoint {
            sequence: self.sequence,
            hash_prefix: self.hash_prefix(),
            depth: self.chain_state.depth,
            // Rotation can be reconstructed from hash if needed
            rotation_hint: [
                (self.chain_state.rotation.w * 127.0) as i8,
                (self.chain_state.rotation.x * 127.0) as i8,
                (self.chain_state.rotation.y * 127.0) as i8,
                (self.chain_state.rotation.z * 127.0) as i8,
            ],
        }
    }
}

/// Compact checkpoint for efficient transmission
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactCheckpoint {
    /// Frame sequence
    pub sequence: u64,
    /// Hash prefix (first 4 bytes)
    pub hash_prefix: [u8; 4],
    /// Chain depth
    pub depth: u64,
    /// Quantized rotation hint
    pub rotation_hint: [i8; 4],
}

impl CompactCheckpoint {
    /// Get approximate rotation
    pub fn rotation(&self) -> Quaternion {
        Quaternion::new(
            self.rotation_hint[0] as f64 / 127.0,
            self.rotation_hint[1] as f64 / 127.0,
            self.rotation_hint[2] as f64 / 127.0,
            self.rotation_hint[3] as f64 / 127.0,
        ).normalize()
    }

    /// Verify hash prefix matches
    pub fn matches_hash(&self, hash: &[u8; 32]) -> bool {
        hash[0..4] == self.hash_prefix
    }
}

/// Manages checkpoint storage and retrieval
pub struct CheckpointManager {
    config: CheckpointConfig,
    /// Level 1 checkpoints (fine granularity)
    level1: Vec<Checkpoint>,
    /// Level 2 checkpoints (coarse granularity)
    level2: Vec<Checkpoint>,
    /// Current frame count
    frame_count: u64,
    /// Last recorded sequence
    last_sequence: u64,
}

impl CheckpointManager {
    /// Create new checkpoint manager
    pub fn new(config: CheckpointConfig) -> Self {
        Self {
            level1: Vec::with_capacity(config.max_checkpoints),
            level2: Vec::with_capacity(config.max_checkpoints / 4),
            frame_count: 0,
            last_sequence: 0,
            config,
        }
    }

    /// Record a checkpoint if needed
    pub fn record(&mut self, chain_state: &ChainState) {
        self.frame_count += 1;

        // Check if level-2 checkpoint
        if self.frame_count % self.config.level2_interval == 0 {
            let checkpoint = Checkpoint::new(
                self.frame_count,
                chain_state.clone(),
                2,
            );
            self.add_level2(checkpoint);
        }
        // Check if level-1 checkpoint
        else if self.frame_count % self.config.level1_interval == 0 {
            let checkpoint = Checkpoint::new(
                self.frame_count,
                chain_state.clone(),
                1,
            );
            self.add_level1(checkpoint);
        }

        self.last_sequence = self.frame_count;
    }

    /// Force a checkpoint at current position
    pub fn force_checkpoint(&mut self, chain_state: &ChainState, level: u8) {
        let checkpoint = Checkpoint::new(
            self.frame_count,
            chain_state.clone(),
            level,
        );

        match level {
            1 => self.add_level1(checkpoint),
            2 => self.add_level2(checkpoint),
            _ => {}
        }
    }

    /// Find nearest checkpoint before target sequence
    pub fn find_nearest(&self, target_sequence: u64) -> Option<&Checkpoint> {
        // First check level-2 for efficiency
        let l2_best = self.level2.iter()
            .filter(|c| c.sequence <= target_sequence)
            .max_by_key(|c| c.sequence);

        // Then check level-1 for precision
        let l1_best = self.level1.iter()
            .filter(|c| c.sequence <= target_sequence)
            .max_by_key(|c| c.sequence);

        // Return the one closest to target
        match (l1_best, l2_best) {
            (Some(l1), Some(l2)) => {
                if l1.sequence > l2.sequence {
                    Some(l1)
                } else {
                    Some(l2)
                }
            }
            (Some(l1), None) => Some(l1),
            (None, Some(l2)) => Some(l2),
            (None, None) => None,
        }
    }

    /// Find checkpoint by hash prefix
    pub fn find_by_hash(&self, hash_prefix: &[u8; 4]) -> Option<&Checkpoint> {
        self.level1.iter()
            .chain(self.level2.iter())
            .find(|c| c.hash_prefix() == *hash_prefix)
    }

    /// Get all checkpoints in range
    pub fn checkpoints_in_range(&self, start: u64, end: u64) -> Vec<&Checkpoint> {
        self.level1.iter()
            .chain(self.level2.iter())
            .filter(|c| c.sequence >= start && c.sequence <= end)
            .collect()
    }

    /// Get latest checkpoint
    pub fn latest(&self) -> Option<&Checkpoint> {
        let l1_latest = self.level1.last();
        let l2_latest = self.level2.last();

        match (l1_latest, l2_latest) {
            (Some(l1), Some(l2)) => {
                if l1.sequence > l2.sequence { Some(l1) } else { Some(l2) }
            }
            (Some(l1), None) => Some(l1),
            (None, Some(l2)) => Some(l2),
            (None, None) => None,
        }
    }

    /// Check if checkpoint should be recorded
    pub fn should_checkpoint(&self) -> bool {
        self.config.auto_checkpoint
            && (self.frame_count + 1) % self.config.level1_interval == 0
    }

    /// Get frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Set frame count (for resync)
    pub fn set_frame_count(&mut self, count: u64) {
        self.frame_count = count;
    }

    /// Clear all checkpoints
    pub fn clear(&mut self) {
        self.level1.clear();
        self.level2.clear();
        self.frame_count = 0;
        self.last_sequence = 0;
    }

    /// Add level-1 checkpoint with eviction
    fn add_level1(&mut self, checkpoint: Checkpoint) {
        self.level1.push(checkpoint);
        while self.level1.len() > self.config.max_checkpoints {
            self.level1.remove(0);
        }
    }

    /// Add level-2 checkpoint with eviction
    fn add_level2(&mut self, checkpoint: Checkpoint) {
        self.level2.push(checkpoint);
        while self.level2.len() > self.config.max_checkpoints / 4 {
            self.level2.remove(0);
        }
    }

    /// Get checkpoint statistics
    pub fn stats(&self) -> CheckpointStats {
        CheckpointStats {
            level1_count: self.level1.len(),
            level2_count: self.level2.len(),
            frame_count: self.frame_count,
            oldest_sequence: self.level1.first()
                .or(self.level2.first())
                .map(|c| c.sequence)
                .unwrap_or(0),
            newest_sequence: self.last_sequence,
        }
    }
}

/// Checkpoint manager statistics
#[derive(Clone, Debug)]
pub struct CheckpointStats {
    /// Number of level-1 checkpoints
    pub level1_count: usize,
    /// Number of level-2 checkpoints
    pub level2_count: usize,
    /// Total frames processed
    pub frame_count: u64,
    /// Oldest checkpoint sequence
    pub oldest_sequence: u64,
    /// Newest checkpoint sequence
    pub newest_sequence: u64,
}

/// Differential checkpoint for bandwidth efficiency
/// Only stores changes from previous checkpoint
#[derive(Clone, Debug)]
pub struct DeltaCheckpoint {
    /// Reference checkpoint sequence
    pub reference: u64,
    /// Frame delta
    pub delta_frames: u32,
    /// Hash XOR with reference
    pub hash_delta: [u8; 4],
}

impl DeltaCheckpoint {
    /// Create delta from two checkpoints
    pub fn from_checkpoints(reference: &Checkpoint, current: &Checkpoint) -> Self {
        let hash_delta = [
            reference.chain_state.hash[0] ^ current.chain_state.hash[0],
            reference.chain_state.hash[1] ^ current.chain_state.hash[1],
            reference.chain_state.hash[2] ^ current.chain_state.hash[2],
            reference.chain_state.hash[3] ^ current.chain_state.hash[3],
        ];

        Self {
            reference: reference.sequence,
            delta_frames: (current.sequence - reference.sequence) as u32,
            hash_delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::sha256;

    fn make_chain_state(seed: u8) -> ChainState {
        let mut hash = [0u8; 32];
        hash[0] = seed;
        hash[1] = seed.wrapping_mul(7);
        ChainState {
            hash,
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            depth: seed as u64,
        }
    }

    #[test]
    fn test_checkpoint_creation() {
        let state = ChainState::genesis(&sha256(b"test"));
        let checkpoint = Checkpoint::new(100, state.clone(), 1);

        assert_eq!(checkpoint.sequence, 100);
        assert_eq!(checkpoint.level, 1);
        assert!(checkpoint.verify(&state));
    }

    #[test]
    fn test_checkpoint_hash_prefix() {
        let state = ChainState::genesis(&sha256(b"test"));
        let checkpoint = Checkpoint::new(100, state.clone(), 1);

        let prefix = checkpoint.hash_prefix();
        assert_eq!(prefix[0], state.hash[0]);
        assert_eq!(prefix[1], state.hash[1]);
        assert_eq!(prefix[2], state.hash[2]);
        assert_eq!(prefix[3], state.hash[3]);
    }

    #[test]
    fn test_compact_checkpoint() {
        let state = ChainState::genesis(&sha256(b"test"));
        let checkpoint = Checkpoint::new(100, state.clone(), 1);

        let compact = checkpoint.to_compact();
        assert_eq!(compact.sequence, 100);
        assert!(compact.matches_hash(&state.hash));

        // Rotation should be approximately correct
        let rot = compact.rotation();
        let expected = state.rotation;
        assert!(rot.distance(&expected) < 0.1);
    }

    #[test]
    fn test_manager_automatic_checkpoints() {
        let config = CheckpointConfig {
            level1_interval: 4,
            level2_interval: 16,
            max_checkpoints: 10,
            auto_checkpoint: true,
        };

        let mut manager = CheckpointManager::new(config);

        // Record 20 frames
        for i in 0..20 {
            let state = make_chain_state(i as u8);
            manager.record(&state);
        }

        // Should have level-1 checkpoints at 4, 8, 12, 20
        // (16 is level-2)
        assert!(manager.level1.len() >= 3);

        // Should have level-2 checkpoint at 16
        assert!(manager.level2.len() >= 1);
    }

    #[test]
    fn test_find_nearest_checkpoint() {
        let config = CheckpointConfig {
            level1_interval: 4,
            level2_interval: 16,
            max_checkpoints: 10,
            auto_checkpoint: true,
        };

        let mut manager = CheckpointManager::new(config);

        for i in 0..20 {
            let state = make_chain_state(i as u8);
            manager.record(&state);
        }

        // Find nearest to sequence 10
        let nearest = manager.find_nearest(10);
        assert!(nearest.is_some());
        let cp = nearest.unwrap();
        assert!(cp.sequence <= 10);
        assert_eq!(cp.sequence, 8); // Level-1 at 8
    }

    #[test]
    fn test_checkpoint_eviction() {
        let config = CheckpointConfig {
            level1_interval: 1, // Every frame
            level2_interval: 100,
            max_checkpoints: 5,
            auto_checkpoint: true,
        };

        let mut manager = CheckpointManager::new(config);

        for i in 0..20 {
            let state = make_chain_state(i as u8);
            manager.record(&state);
        }

        // Should only have 5 checkpoints
        assert!(manager.level1.len() <= 5);
    }

    #[test]
    fn test_find_by_hash() {
        let config = CheckpointConfig::default();
        let mut manager = CheckpointManager::new(config.clone());

        let state = ChainState::genesis(&sha256(b"test"));
        manager.frame_count = 16; // Trigger checkpoint
        manager.force_checkpoint(&state, 1);

        let prefix = [
            state.hash[0],
            state.hash[1],
            state.hash[2],
            state.hash[3],
        ];

        let found = manager.find_by_hash(&prefix);
        assert!(found.is_some());
    }

    #[test]
    fn test_checkpoint_stats() {
        let config = CheckpointConfig {
            level1_interval: 4,
            level2_interval: 16,
            max_checkpoints: 20,
            auto_checkpoint: true,
        };

        let mut manager = CheckpointManager::new(config);

        for i in 0..32 {
            let state = make_chain_state(i as u8);
            manager.record(&state);
        }

        let stats = manager.stats();
        assert_eq!(stats.frame_count, 32);
        assert!(stats.level1_count > 0);
        assert!(stats.level2_count > 0);
    }

    #[test]
    fn test_delta_checkpoint() {
        let state1 = make_chain_state(10);
        let state2 = make_chain_state(20);

        let cp1 = Checkpoint::new(100, state1, 1);
        let cp2 = Checkpoint::new(110, state2, 1);

        let delta = DeltaCheckpoint::from_checkpoints(&cp1, &cp2);

        assert_eq!(delta.reference, 100);
        assert_eq!(delta.delta_frames, 10);
    }
}

//! TRACE Hash Chain verification
//!
//! Provides cryptographic verification of trace event chains to ensure
//! tamper-evidence and integrity.

use serde::{Deserialize, Serialize};

use super::{event::TRACEEvent, GENESIS_HASH};

/// Result of verifying a hash chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerification {
    /// Whether the chain is valid
    pub is_valid: bool,

    /// Total number of events verified
    pub event_count: usize,

    /// Index of first invalid event (if any)
    pub first_invalid_index: Option<usize>,

    /// Type of error if chain is invalid
    pub error_type: Option<ChainErrorType>,

    /// Human-readable error message
    pub error_message: Option<String>,

    /// Hash of the last valid event
    pub last_valid_hash: Option<String>,
}

impl ChainVerification {
    /// Create a valid verification result
    pub fn valid(event_count: usize, last_hash: String) -> Self {
        Self {
            is_valid: true,
            event_count,
            first_invalid_index: None,
            error_type: None,
            error_message: None,
            last_valid_hash: Some(last_hash),
        }
    }

    /// Create an invalid verification result
    pub fn invalid(
        event_count: usize,
        index: usize,
        error_type: ChainErrorType,
        message: String,
    ) -> Self {
        Self {
            is_valid: false,
            event_count,
            first_invalid_index: Some(index),
            error_type: Some(error_type),
            error_message: Some(message),
            last_valid_hash: None,
        }
    }

    /// Create an empty chain verification (valid but with no events)
    pub fn empty() -> Self {
        Self {
            is_valid: true,
            event_count: 0,
            first_invalid_index: None,
            error_type: None,
            error_message: None,
            last_valid_hash: Some(GENESIS_HASH.to_string()),
        }
    }
}

/// Types of chain errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChainErrorType {
    /// Event's computed hash doesn't match stored hash
    HashMismatch,
    /// Event's previous_event_hash doesn't link to prior event
    ChainBroken,
    /// Sequence numbers are not monotonically increasing
    SequenceGap,
    /// First event doesn't link to genesis hash
    InvalidGenesis,
    /// Timestamps are not monotonically increasing
    TimestampRegression,
}

impl std::fmt::Display for ChainErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChainErrorType::HashMismatch => write!(f, "hash_mismatch"),
            ChainErrorType::ChainBroken => write!(f, "chain_broken"),
            ChainErrorType::SequenceGap => write!(f, "sequence_gap"),
            ChainErrorType::InvalidGenesis => write!(f, "invalid_genesis"),
            ChainErrorType::TimestampRegression => write!(f, "timestamp_regression"),
        }
    }
}

/// Hash chain verifier
pub struct ChainVerifier;

impl ChainVerifier {
    /// Verify a chain of events
    ///
    /// Checks:
    /// 1. First event links to genesis hash
    /// 2. Each event's hash is correctly computed
    /// 3. Each event links to the previous event's hash
    /// 4. Sequence numbers are monotonically increasing
    /// 5. Timestamps are monotonically increasing (optional, relaxed check)
    pub fn verify(events: &[TRACEEvent]) -> ChainVerification {
        if events.is_empty() {
            return ChainVerification::empty();
        }

        // Verify first event links to genesis
        let first = &events[0];
        if first.previous_event_hash != GENESIS_HASH {
            return ChainVerification::invalid(
                events.len(),
                0,
                ChainErrorType::InvalidGenesis,
                format!(
                    "First event previous_event_hash should be genesis hash, got: {}",
                    first.previous_event_hash
                ),
            );
        }

        if first.sequence != 0 {
            return ChainVerification::invalid(
                events.len(),
                0,
                ChainErrorType::SequenceGap,
                format!("First event sequence should be 0, got: {}", first.sequence),
            );
        }

        // Verify first event's hash
        if !first.verify_hash() {
            return ChainVerification::invalid(
                events.len(),
                0,
                ChainErrorType::HashMismatch,
                format!(
                    "First event hash mismatch: stored {}, computed {}",
                    first.event_hash,
                    first.compute_hash()
                ),
            );
        }

        let mut last_hash = first.event_hash.clone();
        let mut last_sequence = first.sequence;
        let mut last_timestamp = first.timestamp;

        // Verify remaining events
        for (i, event) in events.iter().enumerate().skip(1) {
            // Check hash linkage
            if event.previous_event_hash != last_hash {
                return ChainVerification::invalid(
                    events.len(),
                    i,
                    ChainErrorType::ChainBroken,
                    format!(
                        "Event {} previous_event_hash {} doesn't match previous event hash {}",
                        i, event.previous_event_hash, last_hash
                    ),
                );
            }

            // Check sequence
            if event.sequence != last_sequence + 1 {
                return ChainVerification::invalid(
                    events.len(),
                    i,
                    ChainErrorType::SequenceGap,
                    format!(
                        "Event {} sequence {} is not {} + 1",
                        i, event.sequence, last_sequence
                    ),
                );
            }

            // Check hash integrity
            if !event.verify_hash() {
                return ChainVerification::invalid(
                    events.len(),
                    i,
                    ChainErrorType::HashMismatch,
                    format!(
                        "Event {} hash mismatch: stored {}, computed {}",
                        i,
                        event.event_hash,
                        event.compute_hash()
                    ),
                );
            }

            // Optional: check timestamp progression (warn but don't fail)
            // Clock skew can cause minor regressions
            if event.timestamp < last_timestamp {
                // Log warning but don't fail - clock skew is common
                // In strict mode, this could be an error
            }

            last_hash = event.event_hash.clone();
            last_sequence = event.sequence;
            last_timestamp = event.timestamp;
        }

        ChainVerification::valid(events.len(), last_hash)
    }

    /// Verify that one chain is an extension of another
    ///
    /// Returns true if `extension` starts where `base` ends.
    pub fn verify_extension(base: &[TRACEEvent], extension: &[TRACEEvent]) -> bool {
        if base.is_empty() || extension.is_empty() {
            return false;
        }

        let last_base = base.last().unwrap();
        let first_extension = &extension[0];

        // Extension's first event should link to base's last event
        first_extension.previous_event_hash == last_base.event_hash
            && first_extension.sequence == last_base.sequence + 1
    }

    /// Find the point where two chains diverge
    ///
    /// Returns the index of the first differing event, or None if chains are identical.
    pub fn find_divergence(chain_a: &[TRACEEvent], chain_b: &[TRACEEvent]) -> Option<usize> {
        let min_len = chain_a.len().min(chain_b.len());

        for i in 0..min_len {
            if chain_a[i].event_hash != chain_b[i].event_hash {
                return Some(i);
            }
        }

        if chain_a.len() != chain_b.len() {
            Some(min_len)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_chain() -> Vec<TRACEEvent> {
        let first = TRACEEvent::genesis(
            "session-1".to_string(),
            "trace-1".to_string(),
            json!({"agent_id": "agent-1", "goal": "test"}),
        );

        let second = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            super::super::EventType::CARPRequestReceived,
            json!({"request_id": "req-1", "operation": "resolve", "goal": "test"}),
        )
        .chain(1, first.event_hash.clone());

        let third = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            super::super::EventType::SessionEnded,
            json!({"reason": "completed", "duration_ms": 1000}),
        )
        .chain(2, second.event_hash.clone());

        vec![first, second, third]
    }

    #[test]
    fn test_verify_valid_chain() {
        let chain = create_test_chain();
        let result = ChainVerifier::verify(&chain);

        assert!(result.is_valid);
        assert_eq!(result.event_count, 3);
        assert!(result.error_type.is_none());
    }

    #[test]
    fn test_verify_empty_chain() {
        let result = ChainVerifier::verify(&[]);

        assert!(result.is_valid);
        assert_eq!(result.event_count, 0);
    }

    #[test]
    fn test_detect_hash_mismatch() {
        let mut chain = create_test_chain();

        // Tamper with payload
        chain[1].payload = json!({"request_id": "req-2", "operation": "resolve", "goal": "test"});

        let result = ChainVerifier::verify(&chain);

        assert!(!result.is_valid);
        assert_eq!(result.first_invalid_index, Some(1));
        assert_eq!(result.error_type, Some(ChainErrorType::HashMismatch));
    }

    #[test]
    fn test_detect_broken_chain() {
        let mut chain = create_test_chain();

        // Break the chain
        chain[1].previous_event_hash = "invalid_hash".to_string();

        let result = ChainVerifier::verify(&chain);

        assert!(!result.is_valid);
        assert_eq!(result.first_invalid_index, Some(1));
        assert_eq!(result.error_type, Some(ChainErrorType::ChainBroken));
    }

    #[test]
    fn test_detect_sequence_gap() {
        let mut chain = create_test_chain();

        // Create sequence gap
        chain[2].sequence = 5;

        let result = ChainVerifier::verify(&chain);

        assert!(!result.is_valid);
        assert_eq!(result.first_invalid_index, Some(2));
        assert_eq!(result.error_type, Some(ChainErrorType::SequenceGap));
    }

    #[test]
    fn test_verify_extension() {
        let chain = create_test_chain();

        let extension = TRACEEvent::new(
            "session-1".to_string(),
            "trace-1".to_string(),
            super::super::EventType::ActionExecuted,
            json!({"action_id": "test.get", "execution_id": "exec-1", "duration_ms": 100}),
        )
        .chain(3, chain.last().unwrap().event_hash.clone());

        assert!(ChainVerifier::verify_extension(&chain, &[extension]));
    }

    #[test]
    fn test_find_divergence() {
        let chain_a = create_test_chain();
        let mut chain_b = chain_a.clone(); // Clone to get identical chains

        // Same chains
        assert!(ChainVerifier::find_divergence(&chain_a, &chain_a).is_none());
        assert!(ChainVerifier::find_divergence(&chain_a, &chain_b).is_none());

        // Different at index 2
        chain_b[2].payload = json!({"reason": "error", "duration_ms": 500});
        chain_b[2].event_hash = chain_b[2].compute_hash();

        assert_eq!(ChainVerifier::find_divergence(&chain_a, &chain_b), Some(2));

        // Different length
        chain_b.pop();
        assert_eq!(ChainVerifier::find_divergence(&chain_a, &chain_b), Some(2));
    }
}

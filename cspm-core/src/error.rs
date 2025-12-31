//! CSPM error types

use thiserror::Error;

/// Errors that can occur in CSPM operations
#[derive(Debug, Error)]
pub enum CSPMError {
    /// Hash length is invalid
    #[error("Invalid hash length: expected 32 bytes, got {0}")]
    InvalidHashLength(usize),

    /// Vertex index out of range
    #[error("Vertex index {0} out of range (max 119)")]
    VertexIndexOutOfRange(usize),

    /// Quaternion normalization failed
    #[error("Failed to normalize quaternion (near-zero magnitude)")]
    NormalizationFailed,

    /// Decoding error
    #[error("Decode error: {0}")]
    DecodeError(String),

    /// Channel measurement error
    #[error("Channel measurement error: {0}")]
    ChannelError(String),

    /// Hash chain verification failed
    #[error("Hash chain verification failed: expected {expected}, got {actual}")]
    HashChainMismatch { expected: String, actual: String },

    /// Genesis hash not set
    #[error("Genesis hash not initialized")]
    NoGenesisHash,
}

/// Result type for CSPM operations
pub type CSPMResult<T> = Result<T, CSPMError>;

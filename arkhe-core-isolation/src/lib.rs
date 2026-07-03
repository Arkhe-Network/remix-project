pub mod audit;
pub mod cache;
pub mod credential_proxy;
pub mod isolation_barrier;
pub mod memory;
pub mod safe_text;
pub mod secure_context;

pub use safe_text::LlmSafeText;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IsolationError {
    #[error("Session isolation breach detected! Scope collision between request token and frame.")]
    IsolationBreach,

    #[error("Session isolation breach: {session_id} attempted to access {target_session}")]
    CrossSessionBreach {
        session_id: String,
        target_session: String,
    },

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session expired or destroyed: {0}")]
    SessionExpired(String),

    #[error("Invalid session ID")]
    InvalidSessionId,

    #[error("Iteration limit exceeded: {iterations} > {max}")]
    IterationLimitExceeded { iterations: u32, max: u32 },

    #[error("Session token has expired or is no longer present in the active pool.")]
    InvalidSession,

    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),

    #[error("Vault access error: {0}")]
    VaultError(String),

    #[error("Serialization failure: {0}")]
    SerializationError(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SessionId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct NamespaceId(pub String);

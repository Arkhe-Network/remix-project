// src/error.rs
use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("gRPC error: {0}")]
    Grpc(#[from] Status),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid AssetRef: {0}")]
    InvalidAssetRef(String),

    #[error("DID conversion error")]
    DidConversion,

    #[error("Macaroon error: {0}")]
    Macaroon(String),

    #[error("Proof verification failed: {0}")]
    ProofVerification(String),

    #[error("Universe sync failed: {0}")]
    UniverseSync(String),

    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    #[error("Insufficient balance: need {need} have {have}")]
    InsufficientBalance { need: u64, have: u64 },

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Auth error: {0}")]
    Auth(#[from] crate::auth::AuthError),
}

// Em `client.rs` é chamado `fs::read` que pode falhar com `std::io::Error`.
// Outros módulos usam `BridgeError` com `From<tonic::Status>`.
// O #[from] Macro handle those conversions.

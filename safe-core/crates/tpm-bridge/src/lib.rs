//! Safe-Core TPM Bridge
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TpmError {
    #[error("TPM context creation failed: {0}")]
    ContextCreation(String),
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("PCR read failed: {0}")]
    PcrReadFailed(String),
    #[error("TPM not available: {0}")]
    TpmNotAvailable(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpmConfig {
    pub tcti: String,
    pub owner_auth: Vec<u8>,
    pub endorsement_auth: Vec<u8>,
}

impl Default for TpmConfig {
    fn default() -> Self {
        Self {
            tcti: "device:/dev/tpm0".to_string(),
            owner_auth: vec![],
            endorsement_auth: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TpmKeyHandle {
    pub handle: u32,
    pub public_key: Vec<u8>,
    pub algorithm: String,
}

#[cfg(not(feature = "tss-esapi"))]
pub struct TpmBridge;

#[cfg(not(feature = "tss-esapi"))]
impl TpmBridge {
    pub fn new(_config: &TpmConfig) -> Result<Self, TpmError> {
        Err(TpmError::TpmNotAvailable("tss-esapi feature not enabled".into()))
    }
}

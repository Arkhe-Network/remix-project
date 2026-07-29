//! Safe-Core YubiHSM Bridge

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Error)]
pub enum YubiHsmError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("YubiHSM not available: {0}")]
    NotAvailable(String),
    #[error("Mock mode: {0}")]
    MockMode(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YubiHsmConfig {
    pub connector_url: String,
    pub auth_key_id: u16,
    pub password: String,
    pub timeout_ms: u64,
}

impl Default for YubiHsmConfig {
    fn default() -> Self {
        Self {
            connector_url: "http://localhost:12345".to_string(),
            auth_key_id: 1,
            password: String::new(),
            timeout_ms: 5000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YubiHsmKeyHandle {
    pub key_id: u16,
    pub algorithm: String,
    pub public_key: Vec<u8>,
}

#[cfg(feature = "mock")]
pub struct YubiHsmMockClient {
    keys: std::collections::HashMap<u16, Vec<u8>>,
}

#[cfg(feature = "mock")]
impl YubiHsmMockClient {
    pub fn new() -> Self {
        Self {
            keys: std::collections::HashMap::new(),
        }
    }

    pub fn connect(_config: &YubiHsmConfig) -> Result<Self, YubiHsmError> {
        Ok(Self::new())
    }

    pub fn authenticate(&mut self, _config: &YubiHsmConfig) -> Result<(), YubiHsmError> {
        Ok(())
    }

    pub fn generate_ed25519_key(&mut self, key_id: u16, _label: &str) -> Result<YubiHsmKeyHandle, YubiHsmError> {
        let mut rng = rand::thread_rng();
        let mut public_key = vec![0u8; 32];
        rand::RngCore::fill_bytes(&mut rng, &mut public_key);
        self.keys.insert(key_id, public_key.clone());

        Ok(YubiHsmKeyHandle {
            key_id,
            algorithm: "Ed25519".to_string(),
            public_key,
        })
    }

    pub fn sign_ed25519(&mut self, key_id: u16, data: &[u8]) -> Result<Vec<u8>, YubiHsmError> {
        let _ = self.keys.get(&key_id)
            .ok_or_else(|| YubiHsmError::KeyNotFound(format!("Key {} not found", key_id)))?;

        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(&key_id.to_le_bytes());
        let result = hasher.finalize();
        Ok(result.to_vec())
    }
}

pub trait HsmBackend: Send + Sync {
    fn sign(&self, key_id: &str, payload: &[u8]) -> Result<Vec<u8>, YubiHsmError>;
    fn export_public_key(&self, key_id: &str) -> Result<Vec<u8>, YubiHsmError>;
}

use serde::{Serialize, Deserialize};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct Ed25519Signer {
    // Mock for now
}

impl Ed25519Signer {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }
    pub fn public_key(&self) -> [u8; 32] {
        [0u8; 32]
    }
    pub fn public_key_hex(&self) -> String {
        "mock_hex_key".to_string()
    }
    pub fn sign(&self, _msg: &[u8]) -> Result<[u8; 64], String> {
        Ok([0u8; 64])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAttestation {
    pub agent_id: String,
    pub test_type: String,
    pub passed: bool,
    pub commitment: [u8; 32],
    pub receipt_hash: [u8; 32],
    pub metadata: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub signature: Option<Vec<u8>>,
    pub public_key: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAttestation {
    // Mock
}

pub type AttestationId = String;

#[async_trait]
pub trait AttestationStore: Send + Sync {
    type Error;
    async fn store(
        &self,
        agent_id: String,
        test_type: String,
        commitment: [u8; 32],
        receipt_hash: [u8; 32],
        metadata: serde_json::Value,
    ) -> Result<(), Self::Error>;
}

pub struct AttestationBackend {
    // Mock
}

impl AttestationBackend {
    pub fn new_memory() -> Self {
        Self {}
    }
}

#[async_trait]
impl AttestationStore for AttestationBackend {
    type Error = String;
    async fn store(
        &self,
        _agent_id: String,
        _test_type: String,
        _commitment: [u8; 32],
        _receipt_hash: [u8; 32],
        _metadata: serde_json::Value,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

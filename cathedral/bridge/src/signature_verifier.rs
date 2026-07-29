use anyhow::{anyhow, Result};
use common::crypto_config::{CryptoConfig, SignatureAlgorithm};
use cathedral_sdk_rs::crypto::{VerifyingKeyWrapper, CryptoFactory};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// Mock do AgentIdentity and Event para compilar
pub struct AgentIdentity {
    pub ed25519_public_key: Option<Vec<u8>>,
    pub ml_dsa_public_key: Option<Vec<u8>>,
}

pub struct Event {
    pub event_id: String,
    pub payload_json: String,
}

pub struct PublicKeyRegistry {
    keys: Arc<RwLock<HashMap<String, Vec<VerifyingKeyWrapper>>>>,
    config: CryptoConfig,
    factory: CryptoFactory,
}

impl PublicKeyRegistry {
    pub fn new(config: CryptoConfig) -> Self {
        let factory = CryptoFactory::new(config.clone());
        Self {
            keys: Arc::new(RwLock::new(HashMap::new())),
            config,
            factory,
        }
    }

    pub async fn register_key(&self, agent_id: &str, key_bytes: &[u8], alg: SignatureAlgorithm) -> Result<()> {
        let key = VerifyingKeyWrapper::from_bytes(alg, key_bytes)?;
        let mut cache = self.keys.write().await;
        let entry = cache.entry(agent_id.to_string()).or_insert_with(Vec::new);
        if !entry.iter().any(|k| k.algorithm() == alg) {
            entry.push(key);
            info!("Registrada chave {} para agente {}", alg.as_str(), agent_id);
        }
        Ok(())
    }

    pub async fn verify_signature(&self, agent_id: &str, message: &[u8], signature: &[u8]) -> Result<bool> {
        let cache = self.keys.read().await;
        let keys = cache.get(agent_id)
            .ok_or_else(|| anyhow!("Agente {} não possui chave registrada", agent_id))?;

        for key in keys {
            if key.verify(message, signature)? {
                debug!("Assinatura verificada com algoritmo {}", key.algorithm().as_str());
                return Ok(true);
            }
        }
        warn!("Nenhuma chave verificou a assinatura para agente {}", agent_id);
        Ok(false)
    }

    pub async fn register_from_identity(&self, agent_id: &str, identity: &AgentIdentity) -> Result<()> {
        if let Some(ref ed25519_key) = identity.ed25519_public_key {
            self.register_key(agent_id, ed25519_key, SignatureAlgorithm::Ed25519).await?;
        }
        if let Some(ref ml_dsa_key) = identity.ml_dsa_public_key {
            self.register_key(agent_id, ml_dsa_key, SignatureAlgorithm::MlDsa).await?;
        }
        Ok(())
    }
}

pub struct SignatureVerifier {
    registry: Arc<PublicKeyRegistry>,
    config: CryptoConfig,
}

impl SignatureVerifier {
    pub fn new(registry: Arc<PublicKeyRegistry>, config: CryptoConfig) -> Self {
        Self { registry, config }
    }

    pub async fn verify_batch(&self, agent_id: &str, events: &[Event], signature: &[u8]) -> Result<bool> {
        let mut hasher = blake3::Hasher::new();
        for event in events {
            hasher.update(event.event_id.as_bytes());
            hasher.update(event.payload_json.as_bytes());
        }
        let hash = hasher.finalize().as_bytes().to_vec();
        self.registry.verify_signature(agent_id, &hash, signature).await
    }
}

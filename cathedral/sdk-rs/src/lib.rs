#![allow(warnings)]
pub mod crypto;
pub mod grpc_client;

use common::crypto_config::SignatureAlgorithm;
use anyhow::Result;
use serde_json::Value;

use crate::grpc_client::CathedralGrpcClient;
use crate::grpc_client::cathedral_v1::{Event, EventType, EventMetadata, IngestRequest};
use common::crypto_config::CryptoConfig;
use crate::crypto::{CryptoFactory, SigningKeyWrapper};

pub struct CathedralSdkConfig {
    pub endpoint: String,
    pub project_id: String,
    pub agent_id: String,
    pub crypto: CryptoConfig,
    pub private_key_bytes: Option<Vec<u8>>,
}

impl Default for CathedralSdkConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50051".to_string(),
            project_id: "default".to_string(),
            agent_id: "default".to_string(),
            crypto: CryptoConfig::default(),
            private_key_bytes: None,
        }
    }
}

pub struct CathedralSdk {
    client: CathedralGrpcClient,
    project_id: String,
    agent_id: String,
    pub config: CathedralSdkConfig,
    pub signing_key: SigningKeyWrapper,
    pub fallback_key: Option<SigningKeyWrapper>,
    factory: CryptoFactory,
}

impl CathedralSdk {
    pub async fn new(config: CathedralSdkConfig) -> Result<Self> {
        let client = CathedralGrpcClient::connect(config.endpoint.clone()).await?;

        let crypto_config = config.crypto.clone();
        let factory = CryptoFactory::new(crypto_config.clone());

        let signing_key = if let Some(ref bytes) = config.private_key_bytes {
            SigningKeyWrapper::from_bytes(crypto_config.signature_algorithm, bytes)?
        } else {
            factory.generate_signing_key()?
        };

        let fallback_key = if crypto_config.dual_stack_mode {
            if let Some(fallback_alg) = crypto_config.fallback_signature_algorithm {
                if let Some(ref bytes) = config.private_key_bytes {
                    Some(SigningKeyWrapper::from_bytes(fallback_alg, bytes)?)
                } else {
                    factory.generate_fallback_key()?
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            client,
            project_id: config.project_id.clone(),
            agent_id: config.agent_id.clone(),
            config,
            signing_key,
            fallback_key,
            factory,
        })
    }

    pub async fn emit_design_proposed(
        &mut self,
        design_hash: String,
        parent_hashes: Vec<String>,
        payload: Value,
    ) -> Result<()> {
        let event = Event {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: chrono::Utc::now().timestamp_subsec_nanos() as i32,
            }),
            event_type: EventType::DesignProposed.into(),
            design_hash,
            parent_hashes,
            payload_json: payload.to_string(),
            metadata: Some(EventMetadata {
                domain: "test".to_string(),
                confidence: 0.5,
                compute_cost_usd: 0.0,
                tags: vec![],
            }),
        };

        // Serialize the payload for signing
        let batch_hash = {
            let mut hasher = blake3::Hasher::new();
            hasher.update(event.event_id.as_bytes());
            hasher.update(event.design_hash.as_bytes());
            hasher.finalize().as_bytes().to_vec()
        };

        let (agent_signature, signature_algorithm) = match &self.signing_key {
            SigningKeyWrapper::Ed25519(_sk) => {
                let sig = self.factory.sign(&self.signing_key, &batch_hash).unwrap();
                (Some(sig.to_vec()), Some(SignatureAlgorithm::Ed25519 as i32))
            },
            SigningKeyWrapper::MlDsa(_sk) => {
                let sig = self.factory.sign(&self.signing_key, &batch_hash).unwrap(); // Sign empty context for now
                (Some(sig.to_vec()), Some(SignatureAlgorithm::MlDsa as i32))
            },

        };

        let request = IngestRequest {
            project_id: self.project_id.clone(),
            agent_id: self.agent_id.clone(),
            events: vec![event],
            batch_id: None,
            agent_signature,
            batch_hash: Some(batch_hash),
            signature_algorithm,
        };

        self.client.ingest(request).await?;
        Ok(())
    }
}

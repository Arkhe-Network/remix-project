use crate::{IsolationError, NamespaceId};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use rand::RngCore;
use secrecy::SecretString;
use std::collections::HashMap;
use tokio::sync::RwLock;

pub struct OpaqueCredential {
    encrypted_payload: Vec<u8>,
    nonce: [u8; 12],
}

pub struct CredentialInjectionProxy {
    vault: RwLock<HashMap<(NamespaceId, String), OpaqueCredential>>,
    crypto_key: Key<Aes256Gcm>,
}

pub struct SecuredRequest {
    pub url: String,
    pub payload: String,
    pub authorization_header: SecretString,
}

#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub namespace: NamespaceId,
    pub tool_name: String,
    pub parameters: HashMap<String, String>,
}

impl Default for CredentialInjectionProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialInjectionProxy {
    pub fn new() -> Self {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let crypto_key = *Key::<Aes256Gcm>::from_slice(&key_bytes);

        Self {
            vault: RwLock::new(HashMap::new()),
            crypto_key,
        }
    }

    pub async fn store_credential(
        &self,
        ns: NamespaceId,
        tool_name: String,
        raw_token: &str,
    ) -> Result<(), IsolationError> {
        let cipher = Aes256Gcm::new(&self.crypto_key);
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted = cipher
            .encrypt(nonce, raw_token.as_bytes())
            .map_err(|e| IsolationError::CryptoError(e.to_string()))?;

        let opaque = OpaqueCredential {
            encrypted_payload: encrypted,
            nonce: nonce_bytes,
        };

        let mut lock = self.vault.write().await;
        lock.insert((ns, tool_name), opaque);
        Ok(())
    }

    pub async fn prepare_secure_request(
        &self,
        invocation: &ToolInvocation,
    ) -> Result<SecuredRequest, IsolationError> {
        let lock = self.vault.read().await;
        let opaque = lock
            .get(&(invocation.namespace.clone(), invocation.tool_name.clone()))
            .ok_or_else(|| {
                IsolationError::VaultError(
                    "Requested tool credential missing from proxy vault".into(),
                )
            })?;

        let cipher = Aes256Gcm::new(&self.crypto_key);
        let nonce = Nonce::from_slice(&opaque.nonce);

        let decrypted_bytes = cipher
            .decrypt(nonce, opaque.encrypted_payload.as_slice())
            .map_err(|e| IsolationError::CryptoError(e.to_string()))?;

        let plaintext_token = std::str::from_utf8(&decrypted_bytes)
            .map_err(|e| IsolationError::CryptoError(e.to_string()))?
            .to_string();

        let auth_value = format!("Bearer {}", plaintext_token);

        let body = serde_json::to_string(&invocation.parameters).unwrap_or_default();
        let url = format!("https://api.{}.com", invocation.tool_name);

        Ok(SecuredRequest {
            url,
            payload: body,
            authorization_header: SecretString::new(auth_value.into()),
        })
    }
}

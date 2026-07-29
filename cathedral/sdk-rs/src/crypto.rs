use anyhow::{anyhow, Result};
use common::crypto_config::{CryptoConfig, SignatureAlgorithm};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use pqcrypto_dilithium::dilithium3::{keypair, sign, open, PublicKey, SecretKey, SignedMessage};
use pqcrypto_traits::sign::{PublicKey as TraitPublicKey, SecretKey as TraitSecretKey, SignedMessage as TraitSignedMessage};
use rand::rngs::OsRng;
use rand::RngCore;

pub enum SigningKeyWrapper {
    Ed25519(SigningKey),
    MlDsa(SecretKey),
}

impl SigningKeyWrapper {
    pub fn generate(alg: SignatureAlgorithm) -> Result<Self> {
        match alg {
            SignatureAlgorithm::Ed25519 => {
                let mut csprng = OsRng;
                let mut bytes = [0u8; 32];
                csprng.fill_bytes(&mut bytes);
                Ok(Self::Ed25519(SigningKey::from_bytes(&bytes)))
            }
            SignatureAlgorithm::MlDsa => {
                let (_, sk) = keypair();
                Ok(Self::MlDsa(sk))
            }
            _ => Err(anyhow!("Algoritmo não suportado para geração de chave")),
        }
    }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::Ed25519(sk) => {
                let sig = sk.sign(message);
                Ok(sig.to_bytes().to_vec())
            }
            Self::MlDsa(sk) => {
                let sig = sign(message, sk);
                Ok(sig.as_bytes().to_vec())
            }
        }
    }

    pub fn algorithm(&self) -> SignatureAlgorithm {
        match self {
            Self::Ed25519(_) => SignatureAlgorithm::Ed25519,
            Self::MlDsa(_) => SignatureAlgorithm::MlDsa,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ed25519(sk) => sk.to_bytes().to_vec(),
            Self::MlDsa(sk) => sk.as_bytes().to_vec(),
        }
    }

    pub fn from_bytes(alg: SignatureAlgorithm, bytes: &[u8]) -> Result<Self> {
        match alg {
            SignatureAlgorithm::Ed25519 => {
                let arr: [u8; 32] = bytes.try_into()
                    .map_err(|_| anyhow!("Tamanho inválido para chave Ed25519"))?;
                Ok(Self::Ed25519(SigningKey::from_bytes(&arr)))
            }
            SignatureAlgorithm::MlDsa => {
                let sk = SecretKey::from_bytes(bytes)
                    .map_err(|e| anyhow!("Falha ao carregar chave ML-DSA: {}", e))?;
                Ok(Self::MlDsa(sk))
            }
            _ => Err(anyhow!("Algoritmo não suportado para desserialização")),
        }
    }
}

pub enum VerifyingKeyWrapper {
    Ed25519(VerifyingKey),
    MlDsa(PublicKey),
}

impl VerifyingKeyWrapper {
    pub fn from_bytes(alg: SignatureAlgorithm, bytes: &[u8]) -> Result<Self> {
        match alg {
            SignatureAlgorithm::Ed25519 => {
                let arr: [u8; 32] = bytes.try_into()
                    .map_err(|_| anyhow!("Tamanho inválido para chave Ed25519"))?;
                Ok(Self::Ed25519(VerifyingKey::from_bytes(&arr)?))
            }
            SignatureAlgorithm::MlDsa => {
                let pk = PublicKey::from_bytes(bytes)
                    .map_err(|e| anyhow!("Falha ao carregar chave ML-DSA: {}", e))?;
                Ok(Self::MlDsa(pk))
            }
            _ => Err(anyhow!("Algoritmo não suportado")),
        }
    }

    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool> {
        match self {
            Self::Ed25519(vk) => {
                let arr: [u8; 64] = signature.try_into().map_err(|_| anyhow!("Tamanho inválido para assinatura Ed25519"))?;
                let sig = ed25519_dalek::Signature::from_bytes(&arr);
                Ok(vk.verify(message, &sig).is_ok())
            }
            Self::MlDsa(vk) => {
                let sig = SignedMessage::from_bytes(signature)
                    .map_err(|e| anyhow!("Falha ao carregar assinatura ML-DSA: {}", e))?;
                let opened = open(&sig, vk)
                    .map_err(|e| anyhow!("Falha ao abrir assinatura ML-DSA: {}", e))?;
                // The verification succeeds if open doesn't error out, but we should also check if the message matches
                Ok(opened == message)
            }
        }
    }

    pub fn algorithm(&self) -> SignatureAlgorithm {
        match self {
            Self::Ed25519(_) => SignatureAlgorithm::Ed25519,
            Self::MlDsa(_) => SignatureAlgorithm::MlDsa,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ed25519(vk) => vk.to_bytes().to_vec(),
            Self::MlDsa(vk) => vk.as_bytes().to_vec(),
        }
    }
}

pub struct CryptoFactory {
    config: CryptoConfig,
}

impl CryptoFactory {
    pub fn new(config: CryptoConfig) -> Self {
        Self { config }
    }

    pub fn generate_signing_key(&self) -> Result<SigningKeyWrapper> {
        SigningKeyWrapper::generate(self.config.signature_algorithm)
    }

    pub fn generate_fallback_key(&self) -> Result<Option<SigningKeyWrapper>> {
        if let Some(alg) = self.config.fallback_signature_algorithm {
            Ok(Some(SigningKeyWrapper::generate(alg)?))
        } else {
            Ok(None)
        }
    }

    pub fn load_verifying_key(&self, bytes: &[u8]) -> Result<VerifyingKeyWrapper> {
        if let Ok(key) = VerifyingKeyWrapper::from_bytes(self.config.signature_algorithm, bytes) {
            return Ok(key);
        }
        if let Some(fallback) = self.config.fallback_signature_algorithm {
            if let Ok(key) = VerifyingKeyWrapper::from_bytes(fallback, bytes) {
                return Ok(key);
            }
        }
        Err(anyhow!("Não foi possível carregar chave de verificação"))
    }

    pub fn sign(&self, key: &SigningKeyWrapper, message: &[u8]) -> Result<Vec<u8>> {
        key.sign(message)
    }

    pub fn verify_dual(
        &self,
        primary_key: &VerifyingKeyWrapper,
        fallback_key: Option<&VerifyingKeyWrapper>,
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool> {
        if primary_key.verify(message, signature).unwrap_or(false) {
            return Ok(true);
        }
        if let Some(fb_key) = fallback_key {
            if fb_key.verify(message, signature).unwrap_or(false) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_stack_ed25519_ml_dsa() {
        let config = CryptoConfig {
            signature_algorithm: SignatureAlgorithm::Ed25519,
            fallback_signature_algorithm: Some(SignatureAlgorithm::MlDsa),
            dual_stack_mode: true,
            ..Default::default()
        };
        let factory = CryptoFactory::new(config);

        let primary_sk = factory.generate_signing_key().unwrap();

        let primary_vk = match &primary_sk {
            SigningKeyWrapper::Ed25519(sk) => VerifyingKeyWrapper::Ed25519(sk.verifying_key()),
            _ => panic!(),
        };

        let (fallback_pk, fallback_secret) = keypair();
        let fallback_sk = SigningKeyWrapper::MlDsa(fallback_secret);
        let fallback_vk = VerifyingKeyWrapper::MlDsa(fallback_pk);

        let msg = b"dual-stack test";
        let sig = factory.sign(&primary_sk, msg).unwrap();
        assert!(factory.verify_dual(&primary_vk, Some(&fallback_vk), msg, &sig).unwrap());

        let sig_fb = factory.sign(&fallback_sk, msg).unwrap();
        assert!(factory.verify_dual(&primary_vk, Some(&fallback_vk), msg, &sig_fb).unwrap());
    }
}

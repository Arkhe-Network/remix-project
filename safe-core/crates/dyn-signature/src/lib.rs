//! Safe-Core DynSignature
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Algorithm mismatch: expected {expected}, got {actual}")]
    AlgorithmMismatch { expected: String, actual: String },
    #[error("Feature not enabled: {0}")]
    FeatureNotEnabled(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    #[serde(rename = "P256")]
    P256,
    #[serde(rename = "Ed25519")]
    Ed25519,
}

impl SignatureAlgorithm {
    pub fn as_str(&self) -> &str {
        match self {
            SignatureAlgorithm::P256 => "P256",
            SignatureAlgorithm::Ed25519 => "Ed25519",
        }
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub enum DynPrivateKey {
    #[cfg(feature = "p256")]
    #[zeroize(skip)]
    P256(p256::ecdsa::SigningKey),
    #[cfg(feature = "ed25519")]
    #[zeroize(skip)]
    Ed25519(ed25519_dalek::SigningKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynVerifyingKey {
    #[cfg(feature = "p256")]
    P256(p256::ecdsa::VerifyingKey),
    #[cfg(feature = "ed25519")]
    Ed25519(ed25519_dalek::VerifyingKey),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DynSignature {
    #[cfg(feature = "p256")]
    P256(Vec<u8>),
    #[cfg(feature = "ed25519")]
    Ed25519(Vec<u8>),
}

impl DynSignature {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            #[cfg(feature = "p256")]
            DynSignature::P256(sig) => sig.to_vec(),
            #[cfg(feature = "ed25519")]
            DynSignature::Ed25519(sig) => sig.to_vec(),
        }
    }
}

pub fn verify_dyn_signature(sig: &DynSignature, key: &DynVerifyingKey, payload: &[u8]) -> Result<(), SignatureError> {
    match (sig, key) {
        #[cfg(feature = "p256")]
        (DynSignature::P256(sig_bytes), DynVerifyingKey::P256(vk)) => {
            use p256::ecdsa::signature::Verifier;
            let signature = p256::ecdsa::Signature::from_slice(sig_bytes)
                .map_err(|e| SignatureError::InvalidKey(e.to_string()))?;
            vk.verify(payload, &signature).map_err(|e| SignatureError::VerificationFailed(e.to_string()))
        }
        #[cfg(feature = "ed25519")]
        (DynSignature::Ed25519(sig_bytes), DynVerifyingKey::Ed25519(vk)) => {
            use ed25519_dalek::Verifier;
            let bytes_arr: [u8; 64] = sig_bytes.as_slice().try_into()
                .map_err(|_| SignatureError::InvalidKey("Ed25519 sig must be 64 bytes".into()))?;
            let signature = ed25519_dalek::Signature::from_bytes(&bytes_arr);
            vk.verify(payload, &signature).map_err(|e| SignatureError::VerificationFailed(e.to_string()))
        }
        _ => Err(SignatureError::AlgorithmMismatch { expected: "Matching algorithms".into(), actual: "Mismatched algorithms".into() }),
    }
}

impl serde::Serialize for DynVerifyingKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            #[cfg(feature = "p256")]
            DynVerifyingKey::P256(vk) => {
                let bytes = vk.to_sec1_bytes();
                let mut state = serializer.serialize_struct("DynVerifyingKey", 2)?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "type", "P256")?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "data", bytes.as_ref())?;
                serde::ser::SerializeStruct::end(state)
            }
            #[cfg(feature = "ed25519")]
            DynVerifyingKey::Ed25519(vk) => {
                let bytes = vk.as_bytes();
                let mut state = serializer.serialize_struct("DynVerifyingKey", 2)?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "type", "Ed25519")?;
                serde::ser::SerializeStruct::serialize_field(&mut state, "data", bytes.as_ref())?;
                serde::ser::SerializeStruct::end(state)
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for DynVerifyingKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct KeyData {
            #[serde(rename = "type")]
            key_type: String,
            data: Vec<u8>,
        }

        let key_data = KeyData::deserialize(deserializer)?;
        match key_data.key_type.as_str() {
            #[cfg(feature = "p256")]
            "P256" => {
                let vk = p256::ecdsa::VerifyingKey::from_sec1_bytes(&key_data.data)
                    .map_err(serde::de::Error::custom)?;
                Ok(DynVerifyingKey::P256(vk))
            }
            #[cfg(feature = "ed25519")]
            "Ed25519" => {
                let arr: [u8; 32] = key_data.data.try_into().map_err(|_| serde::de::Error::custom("Invalid Ed25519 length"))?;
                let vk = ed25519_dalek::VerifyingKey::from_bytes(&arr)
                    .map_err(serde::de::Error::custom)?;
                Ok(DynVerifyingKey::Ed25519(vk))
            }
            _ => Err(serde::de::Error::custom("Unknown key type")),
        }
    }
}

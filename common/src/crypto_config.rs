use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    Ed25519,
    MlDsa,
    SlhDsa,
}

impl SignatureAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ed25519 => "Ed25519",
            Self::MlDsa => "ML-DSA",
            Self::SlhDsa => "SLH-DSA",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyExchangeAlgorithm {
    EcdheP256,
    MlKem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    pub signature_algorithm: SignatureAlgorithm,
    pub fallback_signature_algorithm: Option<SignatureAlgorithm>,
    pub key_exchange_algorithm: KeyExchangeAlgorithm,
    pub dual_stack_mode: bool,
    pub key_size_bits: u32,
    pub performance_thresholds: HashMap<String, f64>,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        let mut perf = HashMap::new();
        perf.insert("max_signature_size_bytes".to_string(), 4096.0);
        perf.insert("max_verification_time_ms".to_string(), 10.0);
        Self {
            signature_algorithm: SignatureAlgorithm::Ed25519,
            fallback_signature_algorithm: None,
            key_exchange_algorithm: KeyExchangeAlgorithm::EcdheP256,
            dual_stack_mode: false,
            key_size_bits: 256,
            performance_thresholds: perf,
        }
    }
}

pub fn crypto_config_from_env() -> CryptoConfig {
    let sig = std::env::var("CATHEDRAL_SIGNATURE_ALG")
        .unwrap_or_else(|_| "Ed25519".to_string());
    let sig_alg = match sig.as_str() {
        "MlDsa" => SignatureAlgorithm::MlDsa,
        "SlhDsa" => SignatureAlgorithm::SlhDsa,
        _ => SignatureAlgorithm::Ed25519,
    };
    let fallback = std::env::var("CATHEDRAL_FALLBACK_SIGNATURE_ALG")
        .ok()
        .and_then(|s| match s.as_str() {
            "MlDsa" => Some(SignatureAlgorithm::MlDsa),
            "SlhDsa" => Some(SignatureAlgorithm::SlhDsa),
            "Ed25519" => Some(SignatureAlgorithm::Ed25519),
            _ => None,
        });
    let dual = std::env::var("CATHEDRAL_DUAL_STACK")
        .unwrap_or_else(|_| "false".to_string()) == "true";
    CryptoConfig {
        signature_algorithm: sig_alg,
        fallback_signature_algorithm: fallback,
        dual_stack_mode: dual,
        ..Default::default()
    }
}

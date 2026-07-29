//! SEI GigaChain — ancoragem on-chain de datasets DeSci
//!
//! SEI é uma blockchain L1 com CosmWasm. Este módulo fornece:
//! - Tipos para interação com contratos DesciAnchor.sol / DesciAnchor.wasm
//! - Serialização de mensagens para o contrato
//! - Stubs para chamadas on-chain (requer implementação real com cosmwasm-std
//!   ou ethers-rs se usar EVM sidechain)
//!
//! NOTA: Em produção, usar cosmwasm-std + cw-multi-test para testes.

use serde::{Deserialize, Serialize};




/// Mensagem para ancorar um dataset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorMsg {
    pub cid: String,
    pub checksum_sha256: String,
    pub author_did: String,
    pub orcid_id: Option<String>,
    pub trace_id: Option<String>,
    pub metadata_uri: Option<String>,
    pub license: String,
}

/// Mensagem para registrar identidade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterIdentityMsg {
    pub did: String,
    pub orcid_id: Option<String>,
    pub controller: String,
}

/// Resposta de query: anchor info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorInfo {
    pub cid: String,
    pub owner: String,
    pub author_did: String,
    pub orcid_id: Option<String>,
    pub trace_id: Option<String>,
    pub checksum_sha256: String,
    pub anchored_at: u64,  // blockchain timestamp
    pub block_height: u64,
    pub tx_hash: String,
}

/// Resposta de query: identidade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityInfo {
    pub did: String,
    pub orcid_id: Option<String>,
    pub controller: String,
    pub anchor_count: u64,
    pub registered_at: u64,
}

/// Evento emitido pelo contrato
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorEvent {
    pub event_type: String,
    pub cid: String,
    pub author_did: String,
    pub block_height: u64,
    pub tx_hash: String,
}

/// Cliente SEI (stub — requer feature `sei-giga` para implementação real)
#[cfg(feature = "sei-giga")]
pub struct SeiGigaClient {
    chain_id: String,
    contract_address: String,
    rpc_url: String,
    http: reqwest::Client,
}

#[cfg(feature = "sei-giga")]
impl SeiGigaClient {
    pub fn new(chain_id: &str, contract_address: &str, rpc_url: &str) -> Self {
        Self {
            chain_id: chain_id.into(),
            contract_address: contract_address.into(),
            rpc_url: rpc_url.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Ancora dataset (stub — em produção: cosmwasm execute)
    pub async fn anchor_dataset(&self, msg: &AnchorMsg) -> Result<AnchorEvent> {
        info!(
            cid = %msg.cid, did = %msg.author_did,
            "Anchoring dataset on SEI (stub)"
        );

        // Stub: em produção seria uma transação cosmwasm real
        Ok(AnchorEvent {
            event_type: "wasm-anchor".into(),
            cid: msg.cid.clone(),
            author_did: msg.author_did.clone(),
            block_height: 0,
            tx_hash: format!("0x{}", blake3::hash(msg.cid.as_bytes()).to_string()[..16].to_string()),
        })
    }

    /// Query anchor info (stub)
    pub async fn query_anchor(&self, cid: &str) -> Result<AnchorInfo> {
        Err(DesciError::AnchorNotFound { cid: cid.into() })
    }

    /// Registra identidade (stub)
    pub async fn register_identity(&self, msg: &RegisterIdentityMsg) -> Result<String> {
        info!(did = %msg.did, "Registering identity on SEI (stub)");
        Ok(format!("0x{}", blake3::hash(msg.did.as_bytes()).to_string()[..16].to_string()))
    }

    pub fn chain_id(&self) -> &str { &self.chain_id }
    pub fn contract_address(&self) -> &str { &self.contract_address }
}

/// Calcula hash do payload de ancoragem (para verificação off-chain)
pub fn compute_anchor_hash(msg: &AnchorMsg) -> String {
    let payload = format!(
        "{}:{}:{}:{}:{}",
        msg.cid, msg.checksum_sha256, msg.author_did,
        msg.orcid_id.as_deref().unwrap_or(""),
        msg.license,
    );
    blake3::hash(payload.as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_anchor_msg() -> AnchorMsg {
        AnchorMsg {
            cid: "QmBRCA1Dataset".into(),
            checksum_sha256: "sha256:abc123def456".into(),
            author_did: "did:arkhe:orcid:abc12345".into(),
            orcid_id: Some("0000-0001-2345-6789".into()),
            trace_id: Some("trace-xyz-789".into()),
            metadata_uri: Some("ipfs://QmMeta".into()),
            license: "CC-BY-4.0".into(),
        }
    }

    #[test]
    fn test_anchor_msg_serialization() {
        let msg = sample_anchor_msg();
        let json = serde_json::to_string_pretty(&msg).unwrap();
        let msg2: AnchorMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.cid, msg2.cid);
        assert_eq!(msg.orcid_id, msg2.orcid_id);
        assert_eq!(msg.trace_id, msg2.trace_id);
    }

    #[test]
    fn test_anchor_hash_deterministic() {
        let msg = sample_anchor_msg();
        let h1 = compute_anchor_hash(&msg);
        let h2 = compute_anchor_hash(&msg);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_anchor_hash_differs_on_change() {
        let mut msg = sample_anchor_msg();
        let h1 = compute_anchor_hash(&msg);
        msg.license = "MIT".into();
        let h2 = compute_anchor_hash(&msg);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_register_identity_msg() {
        let msg = RegisterIdentityMsg {
            did: "did:arkhe:orcid:abc12345".into(),
            orcid_id: Some("0000-0001-2345-6789".into()),
            controller: "did:arkhe:orcid:abc12345".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: RegisterIdentityMsg = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.did, msg2.did);
    }

    #[test]
    fn test_anchor_info_serialization() {
        let info = AnchorInfo {
            cid: "QmTest".into(),
            owner: "sei1abc...".into(),
            author_did: "did:arkhe:x".into(),
            orcid_id: Some("0000-0001-2345-6789".into()),
            trace_id: Some("trace-1".into()),
            checksum_sha256: "sha256:x".into(),
            anchored_at: 1719792000,
            block_height: 12345678,
            tx_hash: "0xABC".into(),
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        let info2: AnchorInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info.block_height, info2.block_height);
        assert_eq!(info.trace_id, info2.trace_id);
    }

    #[test]
    fn test_anchor_event_serialization() {
        let ev = AnchorEvent {
            event_type: "wasm-anchor".into(),
            cid: "QmX".into(),
            author_did: "did:a".into(),
            block_height: 100,
            tx_hash: "0x123".into(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        let ev2: AnchorEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev.tx_hash, ev2.tx_hash);
    }
}

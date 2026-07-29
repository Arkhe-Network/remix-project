//! Publicação descentralizada: IPFS + WormGraph gRPC
//!
//! NOTA: chainlink_ccip crate não existe como cliente Rust.
//! Integração CCIP real = ethers-rs/alloy + smart contracts Solidity.
//! Para notificações internas ARKHE, usamos WormGraph gRPC.

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{DesciError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub name: String,
    pub description: String,
    pub format: String,
    pub version: String,
    pub author_did: String,
    pub orcid_id: Option<String>,
    pub license: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub checksum_sha256: String,
    /// CID do trace IC16 associado
    pub trace_id: Option<String>,
    /// Referência ao node.desci de origem
    pub node_desci_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsPublishResult {
    pub cid: String,
    pub gateway_url: String,
    pub size_bytes: u64,
}

/// Cliente IPFS (requer feature `ipfs`)
#[cfg(feature = "ipfs")]
pub struct IpfsClient {
    api_url: String,
    gateway_url: String,
    http: reqwest::Client,
}

#[cfg(feature = "ipfs")]
impl IpfsClient {
    pub fn local() -> Self {
        Self {
            api_url: "http://127.0.0.1:5001/api/v0".into(),
            gateway_url: "http://127.0.0.1:8080/ipfs".into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn new(api_url: &str, gateway_url: &str) -> Self {
        Self {
            api_url: api_url.into(),
            gateway_url: gateway_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn add_bytes(&self, data: &[u8], filename: &str) -> Result<IpfsPublishResult> {
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(data.to_vec())
                .file_name(filename.to_string()));

        let resp = self.http
            .post(format!("{}/add", self.api_url))
            .multipart(form)
            .send().await
            .map_err(|e| DesciError::IpfsError(e.to_string()))?
            .error_for_status()
            .map_err(|e| DesciError::IpfsError(e.to_string()))?
            .json::<serde_json::Value>().await
            .map_err(|e| DesciError::IpfsError(e.to_string()))?;

        let cid = resp["Hash"].as_str()
            .ok_or_else(|| DesciError::IpfsError("No CID".into()))?
            .to_string();
        let size = resp["Size"].as_u64().unwrap_or(data.len() as u64);

        Ok(IpfsPublishResult {
            cid: cid.clone(),
            gateway_url: format!("{}/{}", self.gateway_url, cid),
            size_bytes: size,
        })
    }

    pub fn api_url(&self) -> &str { &self.api_url }
    pub fn gateway_url(&self) -> &str { &self.gateway_url }
}

/// Stub WormGraph (gRPC real requer proto compilado)
pub struct WormGraphNotifier {
    _endpoint: String,
}

impl WormGraphNotifier {
    pub fn new(endpoint: &str) -> Self {
        Self { _endpoint: endpoint.into() }
    }

    pub async fn notify_publication(
        &self, cid: &str, metadata: &DatasetMetadata,
    ) -> Result<String> {
        let notif_id = blake3::hash(
            format!("{}:{}:{}", cid, metadata.name, chrono::Utc::now().timestamp_millis()).as_bytes()
        ).to_string();

        info!(
            notif_id = %notif_id, cid = %cid, dataset = %metadata.name,
            "WormGraph notification sent (stub)"
        );
        Ok(notif_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub cid: String,
    pub gateway_url: String,
    pub size_bytes: u64,
    pub notification_id: String,
    pub metadata: DatasetMetadata,
}

/// Publicador orquestrado
#[cfg(feature = "ipfs")]
pub struct DeSciPublisher {
    ipfs: IpfsClient,
    wormgraph: WormGraphNotifier,
}

#[cfg(feature = "ipfs")]
impl DeSciPublisher {
    pub fn local() -> Self {
        Self {
            ipfs: IpfsClient::local(),
            wormgraph: WormGraphNotifier::new("http://localhost:50051"),
        }
    }

    pub async fn publish_bytes(
        &self, data: &[u8], filename: &str, metadata: DatasetMetadata,
    ) -> Result<PublishResult> {
        let ipfs_r = self.ipfs.add_bytes(data, filename).await?;
        let notif_id = self.wormgraph.notify_publication(&ipfs_r.cid, &metadata).await?;
        Ok(PublishResult {
            cid: ipfs_r.cid, gateway_url: ipfs_r.gateway_url,
            size_bytes: ipfs_r.size_bytes, notification_id: notif_id,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_meta() -> DatasetMetadata {
        DatasetMetadata {
            name: "BRCA1 Variants".into(),
            description: "Curated BRCA1".into(),
            format: "vcf".into(),
            version: "1.0.0".into(),
            author_did: "did:arkhe:r-001".into(),
            orcid_id: Some("0000-0001-2345-6789".into()),
            license: "CC-BY-4.0".into(),
            tags: vec!["genomics".into()],
            created_at: "2026-07-01T12:00:00Z".into(),
            checksum_sha256: "abc".into(),
            trace_id: Some("trace-123".into()),
            node_desci_url: Some("https://nodes.desci.com/node/42".into()),
        }
    }

    #[test]
    fn test_metadata_serialization() {
        let m = sample_meta();
        let json = serde_json::to_string(&m).unwrap();
        let m2: DatasetMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(m.name, m2.name);
        assert_eq!(m.orcid_id, m2.orcid_id);
        assert_eq!(m.trace_id, m2.trace_id);
        assert_eq!(m.node_desci_url, m2.node_desci_url);
    }

    #[test]
    fn test_publish_result_serialization() {
        let r = PublishResult {
            cid: "QmTest".into(),
            gateway_url: "http://gw/ipfs/QmTest".into(),
            size_bytes: 1024,
            notification_id: "n-123".into(),
            metadata: sample_meta(),
        };
        let json = serde_json::to_string_pretty(&r).unwrap();
        assert!(json.contains("QmTest"));
        assert!(json.contains("BRCA1"));
    }
}

//! Integração com nodes.desci — rede de nós científicos descentralizados
//!
//! nodes.desci é a infraestrutura de nós que hospeda datasets, executa
//! workflows e fornece provenance. Este módulo oferece:
//! - Descoberta de nós disponíveis
//! - Query de datasets por CID ou metadados
//! - Registro de provedores de dados
//! - Healthcheck de nós

use serde::{Deserialize, Serialize};


use crate::error::{DesciError, Result};

/// Informação de um nó nodes.desci
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub url: String,
    pub name: String,
    pub region: String,
    pub status: NodeStatus,
    pub capabilities: Vec<String>,
    pub datasets_count: u64,
    pub last_seen: String,
    pub owner_did: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeStatus {
    Online,
    Offline,
    Degraded,
    Unknown,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online => write!(f, "online"),
            Self::Offline => write!(f, "offline"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Dataset em um nó
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDataset {
    pub cid: String,
    pub name: String,
    pub format: String,
    pub size_bytes: u64,
    pub uploaded_by: String,
    pub uploaded_at: String,
    pub metadata: serde_json::Value,
    /// Trace IC16 associado
    pub trace_id: Option<String>,
    /// ORCID do uploader
    pub orcid_id: Option<String>,
}

/// Resultado de busca em nós
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSearchResult {
    pub node_id: String,
    pub node_url: String,
    pub datasets: Vec<NodeDataset>,
    pub total_matching: u64,
}

/// Cliente nodes.desci (requer feature `ipfs` para HTTP real)
#[cfg(feature = "ipfs")]
pub struct NodesDesciClient {
    base_url: String,
    http: reqwest::Client,
}

#[cfg(feature = "ipfs")]
impl NodesDesciClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Healthcheck de um nó
    pub async fn healthcheck(&self) -> Result<NodeInfo> {
        let url = format!("{}/api/v1/health", self.base_url);
        let resp = self.http.get(&url)
            .send().await
            .map_err(|_e| DesciError::NodeUnreachable { url: self.base_url.clone() })?
            .error_for_status()
            .map_err(|e| DesciError::NodesDesciError(e.to_string()))?;

        let mut info: NodeInfo = resp.json().await
            .map_err(|e| DesciError::NodesDesciError(e.to_string()))?;
        info.status = NodeStatus::Online;
        Ok(info)
    }

    /// Busca datasets por query textual
    pub async fn search_datasets(&self, query: &str, limit: u32) -> Result<NodeSearchResult> {
        let url = format!("{}/api/v1/datasets/search", self.base_url);
        let resp = self.http.get(&url)
            .query(&[("q", query), ("limit", &limit.to_string())])
            .send().await
            .map_err(|e| DesciError::NodesDesciError(e.to_string()))?
            .error_for_status()
            .map_err(|e| DesciError::NodesDesciError(e.to_string()))?;

        let mut result: NodeSearchResult = resp.json().await
            .map_err(|e| DesciError::NodesDesciError(e.to_string()))?;
        result.node_url = self.base_url.clone();
        Ok(result)
    }

    /// Resolve CID para download URL
    pub fn download_url(&self, cid: &str) -> String {
        format!("{}/api/v1/datasets/{}/download", self.base_url, cid)
    }

    pub fn base_url(&self) -> &str { &self.base_url }
}

/// Gerenciador de múltiplos nós
pub struct NodeRegistry {
    nodes: Vec<NodeInfo>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Registra um nó manualmente
    pub fn register(&mut self, node: NodeInfo) {
        if let Some(existing) = self.nodes.iter_mut().find(|n| n.node_id == node.node_id) {
            *existing = node;
        } else {
            self.nodes.push(node);
        }
    }

    /// Retorna nós online
    pub fn online_nodes(&self) -> Vec<&NodeInfo> {
        self.nodes.iter().filter(|n| n.status == NodeStatus::Online).collect()
    }

    /// Retorna nós com capability específica
    pub fn nodes_with_capability(&self, cap: &str) -> Vec<&NodeInfo> {
        self.nodes.iter()
            .filter(|n| n.capabilities.iter().any(|c| c == cap))
            .collect()
    }

    /// Busca em todos os nós online (stub — em produção, paralelo)
    pub fn search_all(&self, _query: &str) -> Vec<NodeSearchResult> {
        self.online_nodes().iter().map(|node| NodeSearchResult {
            node_id: node.node_id.clone(),
            node_url: node.url.clone(),
            datasets: Vec::new(), // Em produção: HTTP request
            total_matching: 0,
        }).collect()
    }

    pub fn all_nodes(&self) -> &[NodeInfo] { &self.nodes }
    pub fn len(&self) -> usize { self.nodes.len() }
    pub fn is_empty(&self) -> bool { self.nodes.is_empty() }
}

impl Default for NodeRegistry {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node() -> NodeInfo {
        NodeInfo {
            node_id: "node-br-sp-01".into(),
            url: "https://nodes.desci.com/node/1".into(),
            name: "São Paulo Research Node".into(),
            region: "br-south".into(),
            status: NodeStatus::Online,
            capabilities: vec!["storage".into(), "nextflow".into(), "jupyter".into()],
            datasets_count: 1247,
            last_seen: "2026-07-01T10:00:00Z".into(),
            owner_did: Some("did:arkhe:node-operator-01".into()),
            metadata: serde_json::json!({"tier": "premium"}),
        }
    }

    fn sample_dataset() -> NodeDataset {
        NodeDataset {
            cid: "QmBRCA1Dataset".into(),
            name: "BRCA1 Variant Dataset v2".into(),
            format: "vcf.gz".into(),
            size_bytes: 15_000_000,
            uploaded_by: "did:arkhe:researcher-001".into(),
            uploaded_at: "2026-06-15T14:30:00Z".into(),
            metadata: serde_json::json!({"genes": ["BRCA1"], "variants": 4200}),
            trace_id: Some("trace-abc-123".into()),
            orcid_id: Some("0000-0001-2345-6789".into()),
        }
    }

    #[test]
    fn test_node_serialization() {
        let n = sample_node();
        let json = serde_json::to_string(&n).unwrap();
        let n2: NodeInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(n.node_id, n2.node_id);
        assert_eq!(n.capabilities, n2.capabilities);
    }

    #[test]
    fn test_dataset_serialization() {
        let d = sample_dataset();
        let json = serde_json::to_string(&d).unwrap();
        let d2: NodeDataset = serde_json::from_str(&json).unwrap();
        assert_eq!(d.cid, d2.cid);
        assert_eq!(d.trace_id, d2.trace_id);
        assert_eq!(d.orcid_id, d2.orcid_id);
    }

    #[test]
    fn test_registry_register_and_filter() {
        let mut reg = NodeRegistry::new();

        let mut n2 = sample_node();
        n2.node_id = "node-us-west-02".into();
        n2.status = NodeStatus::Offline;
        n2.capabilities = vec!["storage".into()];

        reg.register(sample_node());
        reg.register(n2);

        assert_eq!(reg.len(), 2);
        assert_eq!(reg.online_nodes().len(), 1);
        assert_eq!(reg.nodes_with_capability("nextflow").len(), 1);
        assert_eq!(reg.nodes_with_capability("storage").len(), 2);
    }

    #[test]
    fn test_node_status_display() {
        assert_eq!(NodeStatus::Online.to_string(), "online");
        assert_eq!(NodeStatus::Degraded.to_string(), "degraded");
    }

    #[test]
    fn test_search_result_serialization() {
        let r = NodeSearchResult {
            node_id: "node-1".into(),
            node_url: "https://x.com".into(),
            datasets: vec![sample_dataset()],
            total_matching: 1,
        };
        let json = serde_json::to_string_pretty(&r).unwrap();
        assert!(json.contains("QmBRCA1Dataset"));
        assert!(json.contains("trace-abc-123"));
    }
}

use crate::evolution::desci_node_resource::{ContributorRole, NodeStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityResource {
    pub desci_profile: Option<DeSciProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeSciProfile {
    pub nodes: Vec<DeSciNodeRef>,
    pub total_publications: u32,
    pub total_reviews: u32,
    pub total_citations: u32,
    pub reputation_score: f32,
    pub orcid_synced: bool,
    pub last_sync: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeSciNodeRef {
    pub node_id: String,
    pub dpid: Option<String>,
    pub title: String,
    pub status: NodeStatus,
    pub version: String,
    pub role: ContributorRole,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Default for DeSciProfile {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            total_publications: 0,
            total_reviews: 0,
            total_citations: 0,
            reputation_score: 0.0,
            orcid_synced: false,
            last_sync: None,
        }
    }
}

// src/skill/okf/opencut_asset_okf.rs
//! OKF Bundle para projetos OpenCut (multi-track, assets, provenance)

use crate::integrations::opencut_script::OpenCutProject;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ─── Tipos ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAsset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub content: Vec<u8>,
    pub mime_type: String,
    pub duration_seconds: f64,
    pub resolution: Option<(u32, u32)>,
    pub metadata: HashMap<String, String>,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProjectBundle {
    pub project: OpenCutProject,
    pub assets: Vec<VideoAsset>,
    pub output_hash: Option<String>,
    pub provenance: Vec<ProvenanceEntry>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub timestamp: u64,
    pub agent_id: [u8; 32],
    pub action: String,
    pub description: String,
    pub signature: Option<Vec<u8>>,
}

// ─── OKF Conversion ────────────────────────────────────────────────

impl VideoProjectBundle {
    pub fn new(project: OpenCutProject) -> Self {
        Self {
            project,
            assets: Vec::new(),
            output_hash: None,
            provenance: Vec::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
            updated_at: chrono::Utc::now().timestamp() as u64,
        }
    }

    pub fn add_asset(&mut self, name: &str, content: Vec<u8>, mime_type: &str) -> Result<(), String> {
        let content_hash = format!("{:x}", md5::compute(&content));
        let asset = VideoAsset {
            id: format!("asset-{}", self.assets.len() + 1),
            name: name.to_string(),
            description: String::new(),
            content,
            mime_type: mime_type.to_string(),
            duration_seconds: 0.0,
            resolution: None,
            metadata: HashMap::new(),
            content_hash,
        };
        self.assets.push(asset);
        Ok(())
    }

    pub fn add_provenance(
        &mut self,
        agent_id: [u8; 32],
        action: &str,
        description: &str,
        signature: Option<Vec<u8>>,
    ) {
        self.provenance.push(ProvenanceEntry {
            timestamp: chrono::Utc::now().timestamp() as u64,
            agent_id,
            action: action.to_string(),
            description: description.to_string(),
            signature,
        });
    }
}

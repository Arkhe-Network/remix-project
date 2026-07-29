// src/identity.rs
use cathedral_identity::Did;
use serde::{Deserialize, Serialize};

/// Representação de um AssetRef do Taproot Assets.
/// Pode ser um asset_id (UUID) ou group_key (hex).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssetRef {
    AssetId(String),    // UUID
    GroupKey(String),   // Hex string
}

impl AssetRef {
    pub fn from_string(s: &str) -> Self {
        if s.starts_with("group_key_") {
            AssetRef::GroupKey(s.trim_start_matches("group_key_").to_string())
        } else {
            AssetRef::AssetId(s.to_string())
        }
    }

    pub fn to_did(&self) -> Did {
        match self {
            AssetRef::AssetId(id) => Did::parse(&format!("did:cathedral:asset:{}", id)).unwrap_or_else(|_| Did::new("cathedral", "asset", id)),
            AssetRef::GroupKey(key) => Did::parse(&format!("did:cathedral:group:{}", key)).unwrap_or_else(|_| Did::new("cathedral", "group", key)),
        }
    }

    pub fn from_did(did: &Did) -> Option<Self> {
        let s = did.to_string();
        if let Some(id) = s.strip_prefix("did:cathedral:asset:") {
            Some(AssetRef::AssetId(id.to_string()))
        } else if let Some(key) = s.strip_prefix("did:cathedral:group:") {
            Some(AssetRef::GroupKey(key.to_string()))
        } else {
            None
        }
    }
}

pub fn asset_ref_to_did(s: &str) -> Did {
    AssetRef::from_string(s).to_did()
}

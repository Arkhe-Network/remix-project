use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub id: String,
    pub version: String,
    pub state: ResourceState,
    pub interface: ResourceInterface,
    pub created_at: u64,
    pub updated_at: u64,
    pub author: String,
    pub provenance: Vec<ProvenanceEntry>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceState {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceInterface {
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub side_effects: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub operator: String,
    pub timestamp: u64,
    pub agent_id: String,
    pub message: String,
    pub hash_before: Option<String>,
    pub hash_after: Option<String>,
}

pub trait Resource: std::any::Any {
    fn metadata(&self) -> &ResourceMetadata;
    fn metadata_mut(&mut self) -> &mut ResourceMetadata;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn to_bytes(&self) -> Result<Vec<u8>, String>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, String> where Self: Sized;
}

use cathedral_identity::Did;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MemoryEntry {
    pub did: String,
    pub content: String,
    pub thinking: Option<String>,
    pub signature: Vec<u8>,
    pub timestamp: i64,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecutionReceipt {
    pub id: String,
    pub merkle_root: String,
    pub timestamp: i64,
}

pub struct WormGraphClient {}

impl WormGraphClient {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn record_action(
        &self,
        did: &Did,
        action: &str,
        data: serde_json::Value,
    ) -> Result<String, String> {
        Ok("mock_action_id".to_string())
    }
}

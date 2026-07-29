// src/thread/okf_bundle.rs
//! Mock OKF Bundle for threads with provenance
use crate::thread::schema::{Thread, ThreadSource};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadProvenance {
    pub source: ThreadSource,
    pub source_id: String,
    pub ingested_at: u64,
    pub ingested_by: String,
    pub original_format: String,
    pub checksum: String,
    pub signature: Option<String>,
    pub previous_version: Option<String>,
}

pub struct ThreadOkfBuilder {
    pub provenance: ThreadProvenance,
}

impl ThreadOkfBuilder {
    pub fn new(thread: &Thread, ingested_by: &str, original_format: &str) -> Self {
        let provenance = ThreadProvenance {
            source: thread.source.clone(),
            source_id: thread.source_id.clone(),
            ingested_at: chrono::Utc::now().timestamp() as u64,
            ingested_by: ingested_by.to_string(),
            original_format: original_format.to_string(),
            checksum: thread.content_hash.clone(),
            signature: None,
            previous_version: None,
        };

        Self { provenance }
    }

    pub fn with_signature(mut self, signature: &str) -> Self {
        self.provenance.signature = Some(signature.to_string());
        self
    }

    pub fn with_previous_version(mut self, prev_hash: &str) -> Self {
        self.provenance.previous_version = Some(prev_hash.to_string());
        self
    }

    pub fn build(self) -> Result<(), String> {
        Ok(())
    }
}

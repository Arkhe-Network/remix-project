use arkhe_llm::engine::InferenceEngine;
use crate::types::Finding;

pub struct ValidationPhase {
    llm: std::sync::Arc<dyn InferenceEngine>,
}

impl ValidationPhase {
    pub fn new(llm: std::sync::Arc<dyn InferenceEngine>) -> Self {
        Self { llm }
    }

    pub async fn run(&self, findings: Vec<Finding>) -> Result<Vec<Finding>, arkhe_core::ArkheError> {
        Ok(findings)
    }
}

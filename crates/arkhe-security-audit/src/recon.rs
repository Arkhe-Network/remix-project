use arkhe_llm::engine::InferenceEngine;

pub struct ReconnaissancePhase {
    target_dir: String,
    llm: std::sync::Arc<dyn InferenceEngine>,
}

impl ReconnaissancePhase {
    pub fn new(target_dir: &str, llm: std::sync::Arc<dyn InferenceEngine>) -> Self {
        Self {
            target_dir: target_dir.to_string(),
            llm,
        }
    }

    pub async fn run(&self) -> Result<String, arkhe_core::ArkheError> {
        Ok("Architecture mock".to_string())
    }
}

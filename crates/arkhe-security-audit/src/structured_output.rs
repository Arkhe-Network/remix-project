use crate::types::Finding;

pub struct StructuredOutputPhase {}

impl StructuredOutputPhase {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn generate(&self, _findings: &[Finding]) -> Result<(), arkhe_core::ArkheError> {
        Ok(())
    }
}

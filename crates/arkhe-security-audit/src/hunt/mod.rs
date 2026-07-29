//! Fase 2: Caça a Vulnerabilidades (Hunt)
//!
//! 12 Ângulos de ataque adaptados da metodologia Cloudflare.

use arkhe_llm::engine::InferenceEngine;
use crate::types::Finding;

pub struct HuntPhase {
    llm: std::sync::Arc<dyn InferenceEngine>,
}

impl HuntPhase {
    pub fn new(llm: std::sync::Arc<dyn InferenceEngine>) -> Self {
        Self { llm }
    }

    pub async fn run(&self, architecture: &str) -> Result<Vec<Finding>, arkhe_core::ArkheError> {
        tracing::info!("🏹 Iniciando fase de caça");

        let attack_classes = vec![
            "Injection",
            "AccessControl",
            "ResourceManipulation",
            "CryptoAndSecrets",
            "BusinessLogic",
            "DataExfiltration",
            "ChainedAttack",
            "Wildcard",
            "ObviousStuff",
        ];

        let mut handles = vec![];
        for class in attack_classes {
            let llm = self.llm.clone();
            let arch = architecture.to_string();
            let class_str = class.to_string();

            handles.push(tokio::spawn(async move {
                hunt_class(&arch, &class_str, llm).await
            }));
        }

        let mut results = vec![];
        for handle in handles {
            if let Ok(Ok(mut findings)) = handle.await {
                results.append(&mut findings);
            }
        }

        tracing::info!("✅ Encontradas {} vulnerabilidades", results.len());
        Ok(results)
    }
}

async fn hunt_class(
    architecture: &str,
    class: &str,
    llm: std::sync::Arc<dyn InferenceEngine>,
) -> Result<Vec<Finding>, arkhe_core::ArkheError> {
    let prompt = format!(
        r#"
        Você é um caçador de vulnerabilidades especializado em {class}.
        Analise a arquitetura abaixo e encontre vulnerabilidades explotáveis.

        Arquitetura:
        {architecture}

        Classes de ataque específicas para {class}:
        Retorne um JSON array com cada achado.
        "#,
        class = class, architecture = architecture
    );

    let response = llm.generate(&prompt, 0.3, 8192).await?;
    let findings: Vec<Finding> = serde_json::from_str(&response).map_err(|_| arkhe_core::ArkheError("JSON Parse Error".to_string()))?;
    Ok(findings)
}
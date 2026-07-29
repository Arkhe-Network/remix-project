//! Skill para inferência local de LLMs via QVAC Fabric (Vulkan)

use crate::hashtree::adapter::HashTreeStorage;
use crate::observability::trace_manager::TraceManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QVACConfig {
    pub context_size: usize,
    pub threads: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: usize,
}

impl Default for QVACConfig {
    fn default() -> Self {
        Self {
            context_size: 4096,
            threads: 4,
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 1024,
        }
    }
}

/// Wrapper de alto nível para QVAC (abstração para futura integração FFI/C++)
pub struct QVACSession {
    model_path: PathBuf,
    config: QVACConfig,
}

impl QVACSession {
    pub async fn new(model_data: &[u8], config: QVACConfig) -> Result<Self, String> {
        let temp_dir = std::env::temp_dir().join("qvac_models");
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Erro ao criar diretório temporário: {}", e))?;

        let model_path = temp_dir.join(format!("model_{}.gguf", uuid::Uuid::new_v4()));
        std::fs::write(&model_path, model_data)
            .map_err(|e| format!("Erro ao escrever modelo: {}", e))?;

        // TODO: Inicializar sessão real do QVAC (FFI)
        info!("✅ Sessão QVAC inicializada: {}", model_path.display());

        Ok(Self { model_path, config })
    }

    pub async fn infer(&self, prompt: &str) -> Result<String, String> {
        // TODO: Chamada real para QVAC C++ / Vulkan
        info!("🧠 [QVAC] Inferindo localmente...");
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        Ok(format!("[QVAC Local] {}", &prompt[..prompt.len().min(80)]))
    }
}

impl Drop for QVACSession {
    fn drop(&mut self) {
        if self.model_path.exists() {
            let _ = std::fs::remove_file(&self.model_path);
        }
    }
}

// ─── Skill Definition ──────────────────────────────────────────────

pub fn qvac_inference_skill() -> crate::skill::types::Skill {
    let mut metadata = HashMap::new();
    metadata.insert("provider".to_string(), "qvac".to_string());
    metadata.insert("backend".to_string(), "vulkan".to_string());
    metadata.insert("offline".to_string(), "true".to_string());

    crate::skill::types::Skill {
        name: "qvac-inference".to_string(),
        description: "Inferência de LLMs local via QVAC Fabric (offline, privado)".to_string(),
        skill_type: crate::skill::types::SkillType::ModelInvoked,
        version: "1.0.0".to_string(),
        author: Some("Cathedral ARKHE + Tetherto QVAC".to_string()),
        tags: vec!["inference".to_string(), "local".to_string(), "qvac".to_string(), "offline".to_string()],
        triggers: vec!["inferir local".to_string(), "qvac".to_string()],
        instructions: "# Skill: QVAC Inference\n\nExecuta inferência local via QVAC Fabric.".to_string(),
        steps: vec![],
        examples: vec!["Inferir localmente sobre soberania".to_string()],
        dependencies: vec!["qvac".to_string()],
        metadata,
        okf_bundle_id: None,
    }
}

// ─── Executor ──────────────────────────────────────────────────────

pub struct QVACInferenceExecutor {
    storage: HashTreeStorage,
    trace_manager: Arc<TraceManager>,
    config: QVACConfig,
    session_cache: Arc<Mutex<Option<QVACSession>>>,
    default_model_hash: String,
}

impl QVACInferenceExecutor {
    pub fn new(
        storage: HashTreeStorage,
        trace_manager: Arc<TraceManager>,
        config: QVACConfig,
        default_model_hash: &str,
    ) -> Self {
        Self {
            storage,
            trace_manager,
            config,
            session_cache: Arc::new(Mutex::new(None)),
            default_model_hash: default_model_hash.to_string(),
        }
    }

    pub async fn infer(
        &self,
        prompt: &str,
        model_hash: Option<&str>,
        trace_id: Option<&str>,
    ) -> Result<String, String> {
        let model_hash = model_hash.unwrap_or(&self.default_model_hash);

        let mut cache = self.session_cache.lock().await;

        let session = match cache.as_mut() {
            Some(s) => s,
            None => {
                let model_data = self.storage
                    .get_by_path(&format!("models/{}", model_hash))
                    .await
                    .map_err(|e| format!("Modelo '{}' não encontrado no HashTree: {}", model_hash, e))?;

                let new_session = QVACSession::new(&model_data, self.config.clone()).await?;
                *cache = Some(new_session);
                cache.as_mut().unwrap()
            }
        };

        let result = session.infer(prompt).await?;

        if let Some(tid) = trace_id {
            let artifact = serde_json::json!({
                "prompt": prompt,
                "response": result,
                "model_hash": model_hash,
                "backend": "qvac"
            });
            let _ = self.trace_manager.add_artifact(
                tid,
                "qvac_inference.json",
                serde_json::to_vec(&artifact).unwrap(),
                "application/json",
                "Inferência QVAC local",
            ).await;
        }

        Ok(result)
    }
}

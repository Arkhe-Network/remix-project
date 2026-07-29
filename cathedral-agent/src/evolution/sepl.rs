use crate::integrations::eve::{EveClient, EveTask, EveStrategy};
use crate::skill::builtin::qvac_inference::QVACInferenceExecutor;
use crate::hashtree::adapter::HashTreeStorage;
use crate::observability::trace_manager::TraceManager;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionContext {
    pub resource_id: String,
    pub agent_id: String,
    pub goal: String,
    pub constraints: Vec<String>,
    pub available_tools: Vec<String>,
    pub memory_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub resource_id: String,
    pub current_version: String,
    pub performance_metrics: HashMap<String, f64>,
    pub usage_patterns: Vec<String>,
    pub errors: Vec<String>,
    pub context: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub change_type: ChangeType,
    pub path: String,
    pub before: Option<String>,
    pub after: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    PromptTuning,
    ParameterTuning,
    CodeRefactor,
    ArchitectureUpdate,
    DependencyUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub resource_id: String,
    pub target_version: String,
    pub changes: Vec<Change>,
    pub rationale: String,
    pub expected_improvement: HashMap<String, f64>,
    pub proposed_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub success: bool,
    pub validation_errors: Vec<String>,
    pub reward_score: RewardScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardScore {
    pub total: f64,
}

pub struct AutogenesisOperator {
    eve_client: EveClient,
    pub(crate) storage: HashTreeStorage,
    pub(crate) trace_manager: Arc<TraceManager>,
    pub(crate) qvac_executor: Option<QVACInferenceExecutor>,
    pub(crate) max_iterations: usize,
    pub(crate) use_qvac: bool,
}

impl AutogenesisOperator {
    /// Cria um novo operador com suporte a QVAC local
    pub async fn new_with_qvac(
        eve_client: EveClient,
        storage: HashTreeStorage,
        trace_manager: Arc<TraceManager>,
        default_model_hash: &str,
        qvac_config: crate::skill::builtin::qvac_inference::QVACConfig,
        max_iterations: usize,
    ) -> Result<Self, String> {
        let qvac_executor = QVACInferenceExecutor::new(storage.clone(), trace_manager.clone(),


            qvac_config,
            default_model_hash,
        );

        Ok(Self {
            eve_client,
            storage,
            trace_manager,
            qvac_executor: Some(qvac_executor),
            max_iterations,
            use_qvac: true,
        })
    }

    /// Força o uso apenas de Eve (desabilita QVAC)
    pub fn disable_qvac(&mut self) {
        self.use_qvac = false;
    }

    /// Decide qual backend usar para inferência (híbrido QVAC -> Eve)
    pub async fn infer_with_strategy(&self, prompt: &str, trace_id: Option<&str>) -> Result<String, String> {
        if self.use_qvac {
            if let Some(qvac) = &self.qvac_executor {
                match qvac.infer(prompt, None, trace_id).await {
                    Ok(result) => {
                        info!("✅ [QVAC] Inferência local bem-sucedida");
                        return Ok(result);
                    }
                    Err(e) => {
                        warn!("❌ [QVAC] Falha: {}, fallback para Eve", e);
                    }
                }
            }
        }

        // Fallback para Eve (cloud)
        info!("☁️ [Eve] Usando inferência na nuvem (fallback)");
        let task = EveTask::new(prompt).with_strategy(EveStrategy::Prototype);
        let result = self.eve_client.execute_task_blocking(&task, 60).await?;
        Ok(result.code.unwrap_or_default())
    }

    pub async fn reflect(
        &self,
        context: &EvolutionContext,
        resource: &dyn crate::evolution::resource::Resource,
    ) -> Result<Observation, String> {
        info!("🔍 [SEPL] Refletindo sobre recurso: {}", context.resource_id);

        let metrics = HashMap::new(); // Simulated metrics from ThreadIndex
        let prompt = format!(
            "Analyze resource '{}' (version {}). Metrics: {:?}. Goal: {}. Produce structured observation.",
            context.resource_id, resource.metadata().version, metrics, context.goal
        );

        let trace_id = self.trace_manager.start_trace(&context.resource_id).await.ok();
        let response = self.infer_with_strategy(&prompt, trace_id.as_deref()).await?;

        Ok(Observation {
            resource_id: context.resource_id.clone(),
            current_version: resource.metadata().version.clone(),
            performance_metrics: metrics,
            usage_patterns: Vec::new(),
            errors: Vec::new(),
            context: response,
            timestamp: chrono::Utc::now().timestamp() as u64,
        })
    }

    pub async fn propose(
        &self,
        observation: &Observation,
        context: &EvolutionContext,
    ) -> Result<Proposal, String> {
        info!("💡 [SEPL] Propondo evolução para: {}", observation.resource_id);

        let prompt = format!(
            "Based on observation: {:?}, propose concrete changes with rationale and expected improvement.",
            observation
        );

        let trace_id = self.trace_manager.start_trace(&context.resource_id).await.ok();
        let _response = self.infer_with_strategy(&prompt, trace_id.as_deref()).await?;

        Ok(Proposal {
            resource_id: observation.resource_id.clone(),
            target_version: format!("{}-proposed", observation.current_version),
            changes: Vec::new(),
            rationale: "Improvement needed".to_string(),
            expected_improvement: HashMap::new(),
            proposed_by: context.agent_id.clone(),
        })
    }
}

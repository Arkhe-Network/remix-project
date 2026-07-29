use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::collections::HashMap;

use crate::config_loader::{AgentConfigFile, MemoryConfig, TrustConfig, PlanningConfig};

#[derive(Debug, Clone)]
pub struct OrchestratorError(pub String);

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct SphincsSigner {}
impl SphincsSigner {
    pub fn new() -> Self { Self {} }
}

pub struct EventBus {}

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub id: String,
    pub role: String,
    pub planning: PlanningConfig,
    pub memory: MemoryConfig,
    pub trust: TrustConfig,
}

pub struct MultiAgentOrchestrator {
    pub event_bus: Option<Arc<EventBus>>,
    pub signer: Arc<SphincsSigner>,
    pub memory_config: Option<MemoryConfig>,
    pub trust_config: Option<TrustConfig>,
    pub planning_strategy: Option<String>,
    pub registered_agents: HashMap<String, AgentContext>,
}

impl MultiAgentOrchestrator {
    pub fn new(event_bus: Option<Arc<EventBus>>, signer: Arc<SphincsSigner>) -> Self {
        Self {
            event_bus,
            signer,
            memory_config: None,
            trust_config: None,
            planning_strategy: None,
            registered_agents: HashMap::new(),
        }
    }

    pub async fn from_config_files(
        config_path: &str,
        manifest_path: &str,
        event_bus: Option<Arc<EventBus>>,
        signer: Arc<SphincsSigner>,
    ) -> Result<Self, OrchestratorError> {
        // 1. Load config.yaml
        let agent_config = AgentConfigFile::from_yaml(config_path)
            .map_err(|e| OrchestratorError(format!("Config load error: {}", e)))?;

        // 2. Load manifest.json
        let manifest_content = fs::read_to_string(manifest_path)
            .map_err(|e| OrchestratorError(e.to_string()))?;
        let _manifest: serde_json::Value = serde_json::from_str(&manifest_content)
            .map_err(|e| OrchestratorError(e.to_string()))?;

        // 3. Configure memory and tools from config
        let memory_cfg = agent_config.agent.memory.clone();
        let trust_cfg = agent_config.agent.trust.clone();
        let planning_strategy = agent_config.agent.planning.strategy.clone();

        // 4. Initialize orchestrator
        let mut orchestrator = Self::new(event_bus, signer);
        orchestrator.memory_config = Some(memory_cfg.clone());
        orchestrator.trust_config = Some(trust_cfg.clone());
        orchestrator.planning_strategy = Some(planning_strategy.clone());

        // 5. Register the default agent with loaded properties
        let default_agent = AgentContext {
            id: agent_config.agent.id.clone(),
            role: agent_config.agent.role.clone(),
            planning: agent_config.agent.planning.clone(),
            memory: memory_cfg,
            trust: trust_cfg,
        };
        orchestrator.registered_agents.insert(agent_config.agent.id.clone(), default_agent);

        Ok(orchestrator)
    }
}

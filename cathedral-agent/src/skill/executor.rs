//! Executa skills via Subagents

use crate::skill::types::ExecutionStatus;
use crate::skill::manager::SkillManager;
// use crate::orchestrator::subagent_spawner::SubagentSpawner;
use tracing::{info, error};

pub struct SkillExecutor {
    // orchestrator: SubagentSpawner,
    skill_manager: SkillManager,
}

impl SkillExecutor {
    pub fn new(skill_manager: SkillManager) -> Self {
        Self { skill_manager }
    }

    /// Executa uma skill
    pub async fn execute_skill(&mut self, skill_name: &str) -> Result<String, String> {
        info!("⚡ Executando skill '{}'", skill_name);

        // Carrega a skill
        let _skill = self.skill_manager.load_skill(skill_name).await
            .ok_or_else(|| format!("Skill '{}' não encontrada", skill_name))?
            .clone();

        // Em uma implementação real:
        // 1. Converter a skill em SwarmSpec/tarefas
        // 2. Executar pelo orchestrator
        let result = format!("Skill {} executed successfully.", skill_name);

        // Registra execução
        self.skill_manager.record_execution(
            skill_name,
            ExecutionStatus::Completed,
            Some(result.clone().into_bytes()),
            None,
        );

        info!("✅ Skill '{}' executada com sucesso", skill_name);
        Ok(result)
    }

    /// Executa uma skill em background (sem retorno)
    pub async fn execute_skill_background(&mut self, skill_name: &str) {
        match self.execute_skill(skill_name).await {
            Ok(_result) => {
                info!("✅ Background skill '{}' concluída", skill_name);
            }
            Err(e) => {
                error!("❌ Background skill '{}' falhou: {}", skill_name, e);
                self.skill_manager.record_execution(
                    skill_name,
                    ExecutionStatus::Failed,
                    None,
                    Some(e),
                );
            }
        }
    }
}

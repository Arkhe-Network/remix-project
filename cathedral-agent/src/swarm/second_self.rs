//! SecondSelfOrchestrator — Integração Principal de Skills

use crate::skill::manager::SkillManager;
use crate::skill::types::SkillType;
use tracing::info;

pub struct SecondSelfOrchestrator {
    pub skill_manager: SkillManager,
    pub qvac_executor: Option<crate::skill::builtin::qvac_inference::QVACInferenceExecutor>,
}

use crate::integrations::flue::adapter::FlueAdapter;
use crate::integrations::eve::{EveTask, EveResult, EveStrategy, EveDeployTarget};

impl SecondSelfOrchestrator {
    pub fn new(skill_manager: SkillManager) -> Self {
        Self { skill_manager, qvac_executor: None }
    }

    pub async fn init_evolution_system_with_qvac(
        &mut self,
        default_model_hash: &str,
        qvac_config: crate::skill::builtin::qvac_inference::QVACConfig,
    ) -> Result<(), String> {
        let storage = crate::hashtree::adapter::HashTreeStorage::new();
        let trace_manager = std::sync::Arc::new(crate::observability::trace_manager::TraceManager::new());
        let eve_client = crate::integrations::eve::default_eve_client()?;

        let _operator = crate::evolution::sepl::AutogenesisOperator::new_with_qvac(
            eve_client,
            storage.clone(),
            trace_manager.clone(),
            default_model_hash,
            qvac_config.clone(),
            5,
        ).await?;

        self.qvac_executor = Some(crate::skill::builtin::qvac_inference::QVACInferenceExecutor::new(
            storage,
            trace_manager,
            qvac_config,
            default_model_hash,
        ));

        // At this point we would store the operator/pipeline and qvac_executor
        // but this orchestrator structure is simplified for now.
        info!("✅ Sistema de evolução inicializado com QVAC local");
        Ok(())
    }

    /// Gera e implanta uma Skill como Worker Flue
    pub async fn deploy_skill_to_flue(
        &mut self,
        skill_name: &str,
    ) -> Result<String, String> {
        let skill = self.skill_manager.load_skill(skill_name).await
            .ok_or_else(|| format!("Skill '{}' não encontrada", skill_name))?.clone();

        let adapter = FlueAdapter::new(std::path::PathBuf::from("./flue-workers"));
        let _worker_path = adapter.write_worker_files(&skill).await?;
        let url = adapter.deploy_worker(skill_name).await?;

        Ok(url)
    }

    /// Executa uma skill via Worker Flue (HTTP)
    pub async fn execute_skill_on_flue(
        &self,
        skill_name: &str,
        _input: &str,
        _params: &std::collections::HashMap<String, String>,
        flue_url: Option<&str>,
    ) -> Result<String, String> {
        let url = if let Some(url) = flue_url {
            url.to_string()
        } else {
            return Err("URL do Flue não fornecida e a integração de banco de dados não está configurada neste mock".to_string());
        };

        // Mock HTTP request to avoid external dependency for tests/example
        Ok(format!("Mock Execution of skill {} on flue via URL {}", skill_name, url))
    }

    /// Executa uma tarefa via Eve (skill eve-dev)
    pub async fn execute_eve_task(
        &self,
        description: &str,
        strategy: Option<EveStrategy>,
        deploy_target: Option<EveDeployTarget>,
        context: Option<&str>,
        timeout_secs: Option<u64>,
    ) -> Result<EveResult, String> {
        let client = crate::integrations::eve::default_eve_client()?;

        let mut task = EveTask::new(description);
        if let Some(s) = strategy {
            task = task.with_strategy(s);
        }
        if let Some(t) = deploy_target {
            task = task.with_deploy_target(t);
        }
        if let Some(c) = context {
            task = task.with_context(c);
        }

        let timeout = timeout_secs.unwrap_or(120);
        let result = client.execute_task_blocking(&task, timeout).await?;

        Ok(result)
    }

    /// Versão simplificada para uso no CLI
    pub async fn execute_eve_from_cli(
        &self,
        description: &str,
        params: &std::collections::HashMap<String, String>,
    ) -> Result<String, String> {
        let mut strategy = None;
        let mut deploy_target = None;
        let mut context = None;
        let mut timeout = None;

        if let Some(s) = params.get("strategy") {
            strategy = match s.as_str() {
                "tdd" => Some(EveStrategy::Tdd),
                "prototype" => Some(EveStrategy::Prototype),
                "refactor" => Some(EveStrategy::Refactor),
                "security_review" => Some(EveStrategy::SecurityReview),
                "performance" => Some(EveStrategy::Performance),
                "documentation" => Some(EveStrategy::Documentation),
                _ => None,
            };
        }

        if let Some(t) = params.get("deploy") {
            deploy_target = match t.as_str() {
                "preview" => Some(EveDeployTarget::Preview),
                "production" => Some(EveDeployTarget::Production),
                _ => Some(EveDeployTarget::Custom(t.clone())),
            };
        }

        if let Some(c) = params.get("context") {
            context = Some(c.as_str());
        }

        if let Some(t) = params.get("timeout") {
            if let Ok(secs) = t.parse() {
                timeout = Some(secs);
            }
        }

        let result = self.execute_eve_task(
            description,
            strategy,
            deploy_target,
            context,
            timeout,
        ).await?;

        // Formata a saída para o CLI
        let mut output = String::new();
        output.push_str("✅ Task Eve concluída!\n\n");

        if let Some(plan) = result.plan {
            output.push_str("📋 Plano:\n");
            output.push_str(&plan);
            output.push_str("\n\n");
        }

        if let Some(code) = result.code {
            output.push_str("💻 Código gerado:\n");
            output.push_str(&code);
            output.push_str("\n\n");
        }

        if let Some(tests) = result.test_results {
            output.push_str("🧪 Resultados dos testes:\n");
            for test in tests {
                let status = match test.status {
                    crate::integrations::eve::TestStatus::Passed => "✅",
                    crate::integrations::eve::TestStatus::Failed => "❌",
                    crate::integrations::eve::TestStatus::Skipped => "⏭️",
                    crate::integrations::eve::TestStatus::Error => "⚠️",
                };
                output.push_str(&format!("  {} {} ({}ms)\n", status, test.name, test.duration_ms));
                if let Some(err) = test.error {
                    output.push_str(&format!("    Erro: {}\n", err));
                }
            }
            output.push('\n');
        }

        if let Some(url) = result.deploy_url {
            output.push_str(&format!("🚀 Deploy: {}\n", url));
        }

        if let Some(mon) = result.monitoring {
            output.push_str(&format!("📊 Monitoramento: {} (uptime: {}%)\n", mon.url, mon.uptime_percent));
        }

        output.push_str(&format!("⏱️ Tempo de execução: {}ms\n", result.execution_time_ms));

        Ok(output)
    }

    /// Carrega as skills do diretório
    pub async fn load_skills_from_dir(&mut self, dir: &str) -> Result<Vec<String>, String> {
        self.skill_manager.import_from_dir(dir).await
    }

    /// Dispara skills baseadas em um trigger
    pub async fn apply_model_skills(&mut self, input: &str) -> Result<Vec<String>, String> {
        let mut applied = Vec::new();
        let triggers = self.skill_manager.find_by_trigger(input);

        for skill in triggers {
            if skill.skill_type == SkillType::ModelInvoked {
                info!("⚡ Aplicando skill model-invoked automaticamente: {}", skill.name);
                // Em produção, isso executaria a skill como SwarmSpec
                applied.push(skill.name.clone());
            }
        }

        Ok(applied)
    }
}

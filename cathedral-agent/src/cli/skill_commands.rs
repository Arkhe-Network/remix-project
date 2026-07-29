// src/cli/skill_commands.rs (extensão para mock environment)

use crate::swarm::second_self::SecondSelfOrchestrator;
use crate::skill::types::Skill;

#[derive(Debug, Clone)]
pub enum SkillCommand {
    GenerateFlue { skill_name: String },
    DeployFlue { skill_name: String },
    RunFlue { skill_name: String, input: String },
    ListFlue,
    Run { skill_name: String },
}

impl SkillCommand {
    pub fn parse(input: &str) -> Option<Self> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() { return None; }

        match parts[0] {
            "/generate-flue" | "generate-flue" => {
                if parts.len() >= 2 {
                    Some(Self::GenerateFlue { skill_name: parts[1].to_string() })
                } else { None }
            }
            "/deploy-flue" | "deploy-flue" => {
                if parts.len() >= 2 {
                    Some(Self::DeployFlue { skill_name: parts[1].to_string() })
                } else { None }
            }
            "/run-flue" | "run-flue" => {
                if parts.len() >= 3 {
                    let skill_name = parts[1].to_string();
                    let input = parts[2..].join(" ");
                    Some(Self::RunFlue { skill_name, input })
                } else { None }
            }
            "/list-flue" | "list-flue" => Some(Self::ListFlue),
            "/run" | "run" => {
                 if parts.len() >= 2 {
                     Some(Self::Run { skill_name: parts[1..].join(" ") })
                 } else { None }
            }
            _ => None,
        }
    }

    pub async fn execute(
        &self,
        orchestrator: &mut SecondSelfOrchestrator,
    ) -> Result<String, String> {
        match self {
            Self::GenerateFlue { skill_name } => {
                let skill: Skill = orchestrator.skill_manager.load_skill(skill_name).await
                    .ok_or_else(|| format!("Skill '{}' não encontrada", skill_name))?.clone();
                let adapter = crate::integrations::flue::adapter::FlueAdapter::new(
                    std::path::PathBuf::from("./flue-workers")
                );
                let path = adapter.write_worker_files(&skill).await
                    .map_err(|e| format!("Erro ao gerar Worker: {}", e))?;
                Ok(format!("✅ Worker Flue gerado em: {}", path.display()))
            }
            Self::DeployFlue { skill_name } => {
                let url = orchestrator.deploy_skill_to_flue(skill_name).await?;
                Ok(format!("✅ Skill '{}' implantada em: {}", skill_name, url))
            }
            Self::RunFlue { skill_name, input } => {
                let result = orchestrator.execute_skill_on_flue(
                    skill_name,
                    input,
                    &std::collections::HashMap::new(),
                    Some("https://mock.flue.url"),
                ).await?;
                Ok(format!("✅ Resultado do Flue:\n{}", result))
            }
            Self::ListFlue => {
                Ok("📡 Nenhum worker Flue implantado (Mock)".to_string())
            }
            Self::Run { skill_name } => {
                if skill_name.starts_with("eve-dev") {
                    return self.execute_eve_dev(orchestrator, skill_name).await;
                }

                Ok(format!("Executed {}", skill_name))
            }
        }
    }

    async fn execute_eve_dev(
        &self,
        orchestrator: &SecondSelfOrchestrator,
        full_command: &str
    ) -> Result<String, String> {
        let parts: Vec<&str> = full_command.split_whitespace().collect();
        if parts.len() < 2 {
            return Err("Uso: /run eve-dev \"<descrição>\" [--strategy <tipo>] [--deploy preview|production] [--context <path>] [--timeout <segundos>]".to_string());
        }

        let mut description_parts = Vec::new();
        let mut params = std::collections::HashMap::new();
        let mut i = 1;

        while i < parts.len() {
            let part = parts[i];
            if part.starts_with("--") {
                let key = part.trim_start_matches("--").to_string();
                if i + 1 < parts.len() {
                    let value = parts[i + 1].to_string();
                    params.insert(key, value);
                    i += 2;
                } else {
                    i += 1;
                }
            } else {
                description_parts.push(part);
                i += 1;
            }
        }

        let description = description_parts.join(" ");

        if description.is_empty() {
            return Err("Descrição da tarefa não fornecida".to_string());
        }

        orchestrator.execute_eve_from_cli(&description, &params).await
    }
}

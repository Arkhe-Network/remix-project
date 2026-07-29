use crate::skill::types::{Skill, SkillType, SkillExecution, ExecutionStatus};
use std::collections::HashMap;
use tracing::info;

pub struct SkillManager {
    skills: HashMap<String, Skill>,
    executions: Vec<SkillExecution>,
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillManager {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            executions: Vec::new(),
        }
    }

    /// Carrega uma skill local
    pub async fn load_skill(&mut self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// Salva uma skill local
    pub async fn save_skill(&mut self, skill: &Skill) -> Result<String, String> {
        info!("✅ Skill '{}' salva localmente", skill.name);
        self.skills.insert(skill.name.clone(), skill.clone());
        Ok(skill.name.clone())
    }

    /// Importa skill de um arquivo SKILL.md
    pub async fn import_from_file(&mut self, path: &str) -> Result<&Skill, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Erro ao ler {}: {}", path, e))?;
        let skill = Skill::from_markdown(&content, path)?;
        self.save_skill(&skill).await?;
        Ok(self.skills.get(&skill.name).unwrap())
    }

    /// Importa skills de um diretório (recursivo)
    pub async fn import_from_dir(&mut self, dir: &str) -> Result<Vec<String>, String> {
        let mut imported = Vec::new();
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Erro ao ler diretório {}: {}", dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_file() && path.file_name().map(|n| n == "SKILL.md").unwrap_or(false) {
                if let Ok(skill) = self.import_from_file(path.to_str().unwrap()).await {
                    imported.push(skill.name.clone());
                    info!("📥 Skill importada: {}", skill.name);
                }
            }
            if path.is_dir() {
                let sub = Box::pin(self.import_from_dir(path.to_str().unwrap())).await?;
                imported.extend(sub);
            }
        }
        Ok(imported)
    }

    /// Lista skills por tipo
    pub fn list_by_type(&self, skill_type: SkillType) -> Vec<&Skill> {
        self.skills.values()
            .filter(|s| s.skill_type == skill_type)
            .collect()
    }

    /// Encontra skills por trigger
    pub fn find_by_trigger(&self, text: &str) -> Vec<&Skill> {
        let lower = text.to_lowercase();
        self.skills.values()
            .filter(|s| s.triggers.iter().any(|t| lower.contains(&t.to_lowercase())))
            .collect()
    }

    /// Registra uma execução de skill
    pub fn record_execution(&mut self, skill_name: &str, status: ExecutionStatus, output: Option<Vec<u8>>, error: Option<String>) {
        self.executions.push(SkillExecution {
            skill_name: skill_name.to_string(),
            started_at: chrono::Utc::now().timestamp() as u64,
            completed_at: Some(chrono::Utc::now().timestamp() as u64),
            status,
            output,
            error,
        });
    }

    /// Gera CONTEXT.md (domain modeling)
    pub fn generate_context(&self) -> String {
        let mut context = String::new();
        context.push_str("# Domain Context — Skills\n\n");
        context.push_str("## Available Skills\n\n");

        for skill in self.skills.values() {
            context.push_str(&format!("### `{}` ({:?})\n", skill.name, skill.skill_type));
            context.push_str(&format!("> {}\n\n", skill.description));
            if !skill.triggers.is_empty() {
                context.push_str(&format!("**Triggers:** {}\n", skill.triggers.join(", ")));
            }
            context.push_str(&format!("**Steps:** {}\n\n", skill.steps.len()));
        }

        context.push_str("\n## Active Model-Invoked Skills\n\n");
        for skill in self.skills.values() {
            if skill.skill_type == SkillType::ModelInvoked && !skill.triggers.is_empty() {
                context.push_str(&format!("- `{}` (triggers: {})\n", skill.name, skill.triggers.join(", ")));
            }
        }

        context
    }
}

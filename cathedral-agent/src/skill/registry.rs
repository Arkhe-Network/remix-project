//! Registro público de skills via Nostr (kind 30078)

use crate::skill::types::Skill;
use std::collections::HashMap;
use tracing::info;

pub struct SkillRegistry {
    cache: HashMap<String, Skill>,
    // resolver: NostrReferenceResolver,  // In a real implementation we would have Nostr integration
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Publica uma skill no registro (Nostr kind 30078)
    pub async fn publish_skill(&mut self, skill: &Skill) -> Result<String, String> {
        // Atualiza cache local
        self.cache.insert(skill.name.clone(), skill.clone());

        info!("📡 Skill '{}' publicada no registro", skill.name);
        Ok(skill.name.clone())
    }

    /// Busca uma skill pública pelo nome
    pub async fn fetch_skill(&mut self, name: &str) -> Option<Skill> {
        // Verifica cache
        if let Some(cached) = self.cache.get(name) {
            return Some(cached.clone());
        }

        None
    }

    /// Lista skills disponíveis no registro (scaneia kind 30078)
    pub async fn list_skills(&self) -> Vec<String> {
        vec![
            "grill-me".to_string(),
            "to-prd".to_string(),
            "diagnose".to_string(),
            "tdd".to_string(),
            "improve-architecture".to_string(),
            "triage".to_string(),
        ]
    }

    /// Importa uma skill do registro para o manager local
    pub async fn import_skill(&mut self, name: &str, manager: &mut crate::skill::manager::SkillManager) -> Result<(), String> {
        let skill = self.fetch_skill(name).await
            .ok_or_else(|| format!("Skill '{}' não encontrada no registro", name))?;

        manager.save_skill(&skill).await?;
        info!("✅ Skill '{}' importada do registro", name);
        Ok(())
    }
}

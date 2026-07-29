//! Swarm Executor para Skills de tipo Background
//! Executa periodicamente as skills marcadas como Background.

use crate::skill::manager::SkillManager;
use crate::skill::types::SkillType;
use crate::skill::executor::SkillExecutor;
use std::time::Duration;
use tracing::info;
use tokio::time;

pub struct BackgroundSwarm {
    skill_manager: SkillManager,
}

impl BackgroundSwarm {
    pub fn new(skill_manager: SkillManager) -> Self {
        Self { skill_manager }
    }

    /// Roda como um processo em background, executando periodicamente
    pub async fn run_periodically(&mut self, interval_secs: u64) {
        info!("🕒 Iniciando BackgroundSwarm (intervalo: {}s)", interval_secs);
        let mut interval = time::interval(Duration::from_secs(interval_secs));

        loop {
            interval.tick().await;
            self.execute_all_background_skills().await;
        }
    }

    /// Executa todas as skills de background disponíveis
    pub async fn execute_all_background_skills(&mut self) {
        info!("🔄 Executando skills de background...");

        let mut executor = SkillExecutor::new(SkillManager::new());
        // Clona os nomes para evitar problemas de empréstimo (borrow checker)
        let skill_names: Vec<String> = self.skill_manager
            .list_by_type(SkillType::Background)
            .iter()
            .map(|s| s.name.clone())
            .collect();

        if skill_names.is_empty() {
            info!("ℹ️ Nenhuma skill de background encontrada.");
            return;
        }

        for name in skill_names {
            info!("⏳ Iniciando skill de background: {}", name);
            executor.execute_skill_background(&name).await;
        }

        info!("✅ Ciclo de background concluído.");
    }
}

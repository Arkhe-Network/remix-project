use cathedral_agent::swarm::second_self::SecondSelfOrchestrator;
use cathedral_agent::skill::manager::SkillManager;
use cathedral_agent::skill::builtin;
use cathedral_agent::integrations::eve::{EveStrategy, EveDeployTarget};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut skill_mgr = SkillManager::new();
    let _registered = builtin::register_all(&mut skill_mgr).await?;
    let orchestrator = SecondSelfOrchestrator::new(skill_mgr);

    let result = orchestrator.execute_eve_task(
        "Criar uma API REST para gerenciar produtos com autenticação JWT",
        Some(EveStrategy::Tdd),
        Some(EveDeployTarget::Preview),
        None,
        Some(120),
    ).await?;

    println!("✅ Demo de Execução Eve: {:?}", result);
    Ok(())
}

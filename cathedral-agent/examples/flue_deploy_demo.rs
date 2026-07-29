use cathedral_agent::swarm::second_self::SecondSelfOrchestrator;
use cathedral_agent::skill::manager::SkillManager;
use cathedral_agent::skill::builtin;
use cathedral_agent::cli::skill_commands::SkillCommand;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut skill_mgr = SkillManager::new();
    let _registered = builtin::register_all(&mut skill_mgr).await?;
    let mut orchestrator = SecondSelfOrchestrator::new(skill_mgr);

    let cmd = SkillCommand::GenerateFlue { skill_name: "eve-dev".to_string() };
    let output = cmd.execute(&mut orchestrator).await?;

    println!("✅ Flue Deploy Demo:\n{}", output);
    Ok(())
}

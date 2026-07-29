pub mod desci_publish;
pub mod desci_review;
pub mod grill_me;
pub mod to_prd;
pub mod diagnose;
pub mod tdd;
pub mod improve_architecture;
pub mod triage;
pub mod eve;
pub mod opencut_skill;
pub mod video_edit;
pub mod qvac_inference;
pub mod thread_ingest_claude;
pub mod thread_search;
pub mod thread_ingest_cursor;

use crate::skill::manager::SkillManager;
use crate::skill::types::Skill;

/// Registra todas as skills built-in
pub async fn register_all(skill_mgr: &mut SkillManager) -> Result<Vec<String>, String> {
    let skills: Vec<Skill> = vec![
        grill_me::grill_me_skill(),
        to_prd::to_prd_skill(),
        diagnose::diagnose_skill(),
        tdd::tdd_skill(),
        improve_architecture::improve_architecture_skill(),
        triage::triage_skill(),
        eve::eve_skill(),
        opencut_skill::opencut_skill(),
        video_edit::video_edit_skill(),
        qvac_inference::qvac_inference_skill(),
        thread_ingest_claude::thread_ingest_claude_skill(),
        thread_search::thread_search_skill(),
        thread_ingest_cursor::thread_ingest_cursor_skill(),
    ];

    let mut registered = Vec::new();
    for skill in skills {
        let name = skill_mgr.save_skill(&skill).await?;
        registered.push(name.clone());
        tracing::info!("✅ Skill '{}' registrada", name);
    }

    Ok(registered)
}

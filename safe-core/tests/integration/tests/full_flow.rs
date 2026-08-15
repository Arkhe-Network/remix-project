use safe_core_core::blake3_hash;
use safe_core_identity::ArkheDid;
use safe_core_governance::CapabilityToken;
use safe_core_pea::{Intent, TaskState};

#[tokio::test]
async fn full_end_to_end_flow() {
    // 1. DID
    let did = ArkheDid::new("safe-core", "agent-001");

    // 2. Token
    let key = b"supersecretkey";
    let _token = CapabilityToken::issue(
        did.clone(),
        did.clone(),
        vec!["tool:execute:risk_model".into()],
        std::time::Duration::from_secs(3600),
        key,
    ).unwrap();

    // 3. Intent
    let root_hash = blake3_hash(b"user_prompt");
    let mut intent = Intent::new_root(&root_hash, did.clone(), "risk_model", b"data");
    intent.advance_state(TaskState::Running, &()).unwrap();

    // 4. Policy Engine (Mock)
    // ... enforce

    // 5. Assertions
    // ...
}

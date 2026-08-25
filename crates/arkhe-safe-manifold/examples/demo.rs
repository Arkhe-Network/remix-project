//! Demonstration of the SafeManifold security projection.

use arkhe_safe_manifold::*;

fn main() {
    println!("  ARKHE-χ SafeManifold — Demonstração v0.2.1");

    let manifold = SafeManifold::new();
    let ideal = SystemState::safe(manifold.config.clone());

    let safe = SafeState::new(ideal.clone()).expect("estado deve ser válido");
    println!("   SafeState criado: check_all = {}\n", safe.as_inner().check_all());

    let mut actual = ideal.clone();
    actual.token_budget = 8000;
    actual.agent_count = 9;
    let defect = manifold.compute_observer_defect(&ideal, &actual);
    println!("   Defeito (ideal vs actual): {:.6}\n", defect);

    let degraded = manifold.neron_model(&actual);
    println!("   check_all() = {}\n", degraded.check_all());
}

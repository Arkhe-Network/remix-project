use pretty_assertions::assert_eq;
use crate::types::*;
use crate::delta::*;
use crate::invariants::*;
use crate::hash::IdentityHasher;

#[test]
fn test_invariant_violations_are_diagnosed() {
    let mut s = State::new();
    let hasher = IdentityHasher;
    s = apply(s, &Event::ArtifactAdded(1, "a".into(), "m".into()), &hasher).unwrap();
    s = apply(s, &Event::EvidenceAdded(10, 1, "e".into(), "s".into(), 1, None), &hasher).unwrap();
    s = apply(s, &Event::ClaimAdded(100, "p".into(), vec![10]), &hasher).unwrap();

    // Forçar violação: remover artifact referenciado
    s.artifacts.remove(&1);

    let result = check_invariants(&s);
    assert!(result.is_err());
    let violations = result.unwrap_err();
    assert_eq!(violations.len(), 1);
    assert!(matches!(violations[0], InvariantViolation::Ic6MissingArtifact { .. }));
    println!("Diagnóstico: {}", violations[0]); // "IC6: Evidence 10 references missing artifact 1"
}

#[test]
fn test_ic8_cycle_detection() {
    let mut s = State::new();
    let hasher = IdentityHasher;
    s = apply(s, &Event::ArtifactAdded(1, "a".into(), "m".into()), &hasher).unwrap();
    s = apply(s, &Event::EvidenceAdded(10, 1, "e1".into(), "s".into(), 1, None), &hasher).unwrap();

    // Forçar ciclo: evidence 20 aponta para evidence 10, e 10 aponta para 20
    let hash_10 = s.evidences.get(&10).unwrap().hash.clone();
    s = apply(s, &Event::EvidenceAdded(20, 1, "e2".into(), "s".into(), 2, Some(hash_10.clone())), &hasher).unwrap();
    let hash_20 = s.evidences.get(&20).unwrap().hash.clone();

    // Mutar para criar ciclo (simulação de ataque)
    s.evidences.get_mut(&10).unwrap().parent_hash = Some(hash_20.clone());

    let result = ic8_acyclic(&s);
    assert!(result.is_err());
    println!("Ciclo detectado: {:?}", result.unwrap_err());
}

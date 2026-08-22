// FILE: crates/safe-core-utils/src/lib.rs
//! Utilitarios do Safe-Core — Architectural Decision Log, Provenance, Helpers
//!
//! Implementa I76 (Architectural Decision Provenance) e invariantes I78-I83
//! para o Safe-Core.

pub mod architectural_decision;
pub mod architectural_audit;
pub mod persistence;

pub use architectural_decision::{
    ArchitecturalDecision, DecisionHash, DecisionStatus, DecisionDomain,
    Reference, ReferenceKind, RejectedAlternative, Negation,
    Rationale, ValidityWindow, SystemStateHash, VerificationStatus,
};
pub use architectural_audit::{
    ArchitecturalAudit, AuditError, QuiescenceState,
};

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_decision(id_suffix: u8, timestamp: u64, status: DecisionStatus) -> ArchitecturalDecision {
        ArchitecturalDecision {
            id: Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, id_suffix,
            ]),
            timestamp,
            status,
            domain: DecisionDomain::Core,
            title: format!("Decision {}", id_suffix),
            problem: "Test".to_string(),
            decision_outcome: "Test".to_string(),
            rationale: Rationale {
                metaphor: None,
                computational_translation: "Test translation".to_string(),
                metaphor_verified: false,
            },
            references: vec![],
            negation: Negation {
                description: "Test negation".to_string(),
                rationale: "Test rationale".to_string(),
                alternatives: vec![],
                foreclosure_references: vec![],
                refusal_timestamp: timestamp,
                refusal_hash: DecisionHash([0u8; 32]),
                precondition_for_reopening: None,
                evidence_that_would_reopen: None,
            },
            expected_validity: None,
            decision_makers: vec!["Test".to_string()],
            consulted: vec![],
            informed: vec![],
            system_state_hash: None,
            decision_hash: DecisionHash([0u8; 32]),
            previous_decision_hash: None,
            superseded_by: None,
        }
    }

    // ==================== TESTES BASICOS ====================

    #[test]
    fn test_empty_audit() {
        let audit = ArchitecturalAudit::new();
        assert!(audit.is_empty());
        assert_eq!(audit.len(), 0);
        assert_eq!(audit.quiescence, QuiescenceState::Active);
    }

    #[test]
    fn test_record_and_trail() {
        let mut audit = ArchitecturalAudit::new();
        let d = make_decision(1, 1724250600, DecisionStatus::Accepted);
        let _hash = audit.record(d.clone()).unwrap();
        assert_eq!(audit.len(), 1);
        assert!(audit.contains(&d.id));
        assert_eq!(audit.trail_backwards().len(), 1);
        assert_eq!(audit.trail_forwards().len(), 1);
        assert_eq!(audit.trail_current().len(), 1);
        audit.verify_chain().unwrap();
    }

    #[test]
    fn test_chain_integrity() {
        let mut audit = ArchitecturalAudit::new();
        for i in 1..=3 {
            let d = make_decision(i, 1724250600 + i as u64, DecisionStatus::Accepted);
            audit.record(d).unwrap();
        }
        assert_eq!(audit.len(), 3);
        audit.verify_chain().unwrap();
        assert_eq!(audit.trail_backwards().len(), 3);
        assert_eq!(audit.trail_forwards().len(), 3);
    }

    // ==================== TESTE I76.1 — FORK RESOLUTION ====================

    #[test]
    fn test_no_fork() {
        let mut audit = ArchitecturalAudit::new();
        for i in 1..=3 {
            let d = make_decision(i, 1724250600 + i as u64, DecisionStatus::Accepted);
            audit.record(d).unwrap();
        }
        let forks = audit.detect_forks();
        assert!(forks.is_empty(), "Nao deve haver fork em cadeia linear");
        assert_eq!(audit.quiescence, QuiescenceState::Active);
    }

    #[test]
    fn test_fork_detected() {
        let mut audit = ArchitecturalAudit::new();
        // Genesis
        let genesis = make_decision(1, 1724250600, DecisionStatus::Accepted);
        audit.record(genesis).unwrap();

        // Fork: duas decisoes apontando para o mesmo previous (simulado)
        // Na implementacao real, fork ocorre em sistemas distribuidos
        // Aqui simulamos criando duas cadeias separadas
        let mut audit2 = ArchitecturalAudit::new();
        let g2 = make_decision(1, 1724250600, DecisionStatus::Accepted);
        audit2.record(g2).unwrap();
        let b = make_decision(2, 1724250601, DecisionStatus::Accepted);
        audit2.record(b).unwrap();

        // Merge manual: adiciona as decisoes de audit2 em audit
        // Isso simula recebimento de cadeia concorrente
        for d in audit2.trail_forwards() {
            if !audit.contains(&d.id) {
                audit.record(d.clone()).unwrap();
            }
        }

        // Agora deve haver duas cabecas (fork)
        let forks = audit.detect_forks();
        assert_eq!(forks.len(), 2, "Deve detectar fork com duas cabecas");
        assert_eq!(audit.quiescence, QuiescenceState::ForkDetected);
    }

    #[test]
    fn test_resolve_fork() {
        let mut audit = ArchitecturalAudit::new();
        let d1 = make_decision(1, 1724250600, DecisionStatus::Accepted);
        audit.record(d1).unwrap();
        let d2 = make_decision(2, 1724250601, DecisionStatus::Accepted);
        audit.record(d2).unwrap();

        // Sem fork — resolve deve retornar a cadeia inteira
        let resolved = audit.resolve_fork().unwrap();
        assert_eq!(resolved.len(), 2);
        assert_eq!(audit.quiescence, QuiescenceState::Active);
    }

    // ==================== TESTE DE PERSISTENCIA ====================

    #[test]
    fn test_persistence_roundtrip() {
        use std::fs;
        let path = "/tmp/safe-core-test-log.bin";
        let _ = fs::remove_file(path); // limpa

        // Cria e registra
        {
            let mut audit = ArchitecturalAudit::with_persistence(path).unwrap();
            let d = make_decision(1, 1724250600, DecisionStatus::Accepted);
            audit.record(d).unwrap();
            assert_eq!(audit.len(), 1);
        }

        // Recupera
        {
            let audit = ArchitecturalAudit::with_persistence(path).unwrap();
            assert_eq!(audit.len(), 1);
            assert!(audit.contains(&Uuid::from_bytes([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            ])));
            audit.verify_chain().unwrap();
        }

        let _ = fs::remove_file(path); // limpa
    }

    // ==================== TESTE I83-001 — CONSTITUICAO COMO DECISAO ====================

    #[test]
    fn test_constitution_adr_i83_001() {
        let mut audit = ArchitecturalAudit::new();

        let constitution_adr = ArchitecturalDecision {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap(),
            timestamp: 1724250600,
            status: DecisionStatus::Accepted,
            domain: DecisionDomain::Hermeneutics,
            title: "ARKHE-HERMENEUTICS v3.0 como Constituicao".to_string(),
            problem: "A constituicao nao se aplica a si mesma; nao tem D-vector, nao tem negacao, nao tem traducao computacional.".to_string(),
            decision_outcome: "A constituicao e registrada como decisao I83-001, com D-vector [1,0,0,0] = 'a constituicao e o norte'.".to_string(),
            rationale: Rationale {
                metaphor: Some("A constituicao e o DNA do sistema".to_string()),
                computational_translation: "A constituicao e um documento versionado em git, com hash assinado, registrado no ArchitecturalAudit como decisao de dominio Hermeneutics. Seu D-vector e [1.0, 0.0, 0.0, 0.0], representando: [1] existencia da constituicao, [0] sem objetivo de otimizacao, [0] sem objetivo de aprendizado, [0] sem objetivo de criacao.".to_string(),
                metaphor_verified: true,
            },
            references: vec![Reference {
                kind: ReferenceKind::Conversation,
                identifier: "ARKHE-HERMENEUTICS-v3.0-2026-08-21".to_string(),
                verification_status: VerificationStatus::Verified {
                    by: "Arkhe(n) group".to_string(),
                    at: 1724250600,
                },
                verification_timestamp: Some(1724250600),
            }],
            negation: Negation {
                description: "A constituicao nao sera um documento mutavel sem registro de mudancas; nao sera uma teoria da mente; nao sera um substituto para julgamento humano.".to_string(),
                rationale: "Sem negacao, a constituicao se torna totalitaria — tudo e permitido se estiver no documento.".to_string(),
                alternatives: vec![RejectedAlternative {
                    description: "Deixar a constituicao como documento vivo, editavel sem registro".to_string(),
                    pros: vec!["Flexibilidade".to_string()],
                    cons: vec!["Perda de proveniencia".to_string(), "Risco de edicao retroativa".to_string()],
                    rejected_reason: "Viola P1 (append-only) de I76".to_string(),
                    refutation_references: vec![],
                    confidence_at_rejection: 0.95,
                }],
                foreclosure_references: vec![],
                refusal_timestamp: 1724250600,
                refusal_hash: DecisionHash([0u8; 32]),
                precondition_for_reopening: Some("Demonstracao de que uma constituicao mutavel pode manter proveniencia completa via I76".to_string()),
                evidence_that_would_reopen: Some("Sistema de constituicao que usa I76 para registrar cada emenda, com negacao e encadeamento".to_string()),
            },
            expected_validity: Some(ValidityWindow {
                years: 10,
                revisit_trigger: "major version bump ou inconsistencia detectada".to_string(),
                confidence_level: 0.90,
            }),
            decision_makers: vec!["Arkhe(n) group".to_string()],
            consulted: vec!["Safe-Core Engineering".to_string()],
            informed: vec!["Contributors".to_string(), "Users".to_string()],
            system_state_hash: Some(SystemStateHash([0u8; 32])),
            decision_hash: DecisionHash([0u8; 32]),
            previous_decision_hash: None,
            superseded_by: None,
        };

        let _hash = audit.record(constitution_adr).unwrap();
        assert_eq!(audit.len(), 1);
        assert!(audit.contains(&Uuid::parse_str("550e8400-e29b-41d4-a716-446655440001").unwrap()));

        // Verifica que a decisao esta no trail
        let trail = audit.trail_backwards();
        assert_eq!(trail.len(), 1);
        assert_eq!(trail[0].title, "ARKHE-HERMENEUTICS v3.0 como Constituicao");

        // Verifica negação
        assert!(!trail[0].negation.alternatives.is_empty());
        assert_eq!(trail[0].negation.alternatives[0].rejected_reason, "Viola P1 (append-only) de I76");

        // Verifica traducao computacional
        assert!(!trail[0].rationale.computational_translation.is_empty());
        assert!(trail[0].rationale.metaphor_verified);

        audit.verify_chain().unwrap();
    }
}

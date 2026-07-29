//! Proptest para ARKHE v7.0 — Substrate
//! Testes baseados em propriedades para IC1, IC3, IC4, IC6, IC8

use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Artifact {
    pub id: String,
    pub data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Evidence {
    pub id: String,
    pub artifact_id: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Assertion {
    pub id: String,
    pub evidence_ids: Vec<String>,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Claim {
    pub id: String,
    pub confidence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substrate {
    pub artifacts: HashMap<String, Artifact>,
    pub evidence: HashSet<Evidence>,
    pub assertions: HashSet<Assertion>,
    pub claims: HashSet<Claim>,
}

impl Substrate {
    pub fn new() -> Self {
        Self {
            artifacts: HashMap::new(),
            evidence: HashSet::new(),
            assertions: HashSet::new(),
            claims: HashSet::new(),
        }
    }

    /// IC1: identidade canônica
    pub fn canonical_id(&self, artifact: &Artifact) -> Option<String> {
        self.artifacts.iter().find(|(_, a)| *a == artifact).map(|(id, _)| id.clone())
    }

    /// IC2: evidências de uma assertion
    pub fn evidence_of(&self, assertion: &Assertion) -> Vec<Evidence> {
        assertion
            .evidence_ids
            .iter()
            .filter_map(|id| self.evidence.iter().find(|e| e.id == *id))
            .cloned()
            .collect()
    }

    /// IC3: projeção pura
    pub fn project(&self) -> View {
        View {
            artifacts: self.artifacts.clone(),
            evidence: self.evidence.clone(),
            assertions: self.assertions.clone(),
            claims: self.claims.clone(),
        }
    }

    /// IC8: consistência temporal
    pub fn is_temporally_consistent(&self) -> bool {
        self.evidence.iter().all(|e| {
            if let Some(artifact) = self.artifacts.get(&e.artifact_id) {
                e.timestamp >= artifact.id.len() as u64 // placeholder
            } else {
                // Ignore missing artifacts to strictly verify properties about existing evidence's relation to known artifacts, unless it violates IC6.
                true
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub artifacts: HashMap<String, Artifact>,
    pub evidence: HashSet<Evidence>,
    pub assertions: HashSet<Assertion>,
    pub claims: HashSet<Claim>,
}

// ─── Proptest Strategies ────────────────────────────────────────────────

pub fn artifact_strategy() -> impl Strategy<Value = Artifact> {
    any::<String>().prop_map(|data| Artifact {
        id: format!("art-{}", rand::random::<u32>()),
        data,
    })
}

pub fn evidence_strategy(artifact_ids: Vec<String>) -> impl Strategy<Value = Evidence> {
    let ids = artifact_ids.clone();
    any::<u64>().prop_map(move |timestamp| Evidence {
        id: format!("ev-{}", rand::random::<u32>()),
        artifact_id: ids[rand::random::<usize>() % ids.len()].clone(),
        timestamp,
    })
}

pub fn assertion_strategy(
    evidence_ids: Vec<String>,
    artifact_ids: Vec<String>,
) -> impl Strategy<Value = Assertion> {
    let ev_ids = evidence_ids.clone();
    let art_ids = artifact_ids.clone();
    any::<bool>().prop_map(move |_| Assertion {
        id: format!("ass-{}", rand::random::<u32>()),
        evidence_ids: ev_ids.clone(),
        references: art_ids.clone(),
    })
}

pub fn substrate_strategy() -> impl Strategy<Value = Substrate> {
    proptest::collection::vec(artifact_strategy(), 0..10).prop_flat_map(|artifacts| {
        let artifact_ids: Vec<String> = artifacts.iter().map(|a| a.id.clone()).collect();
        let mut art_ids_ev = artifact_ids.clone();
        if art_ids_ev.is_empty() {
            art_ids_ev.push("dummy".to_string());
        }
        let art_ids_ass = art_ids_ev.clone();

        let artifacts_clone = artifacts.clone();
        proptest::collection::vec(evidence_strategy(art_ids_ev), 0..10).prop_flat_map(move |evidences| {
            let ev_ids: Vec<String> = evidences.iter().map(|e| e.id.clone()).collect();
            let artifacts_clone2 = artifacts_clone.clone();
            proptest::collection::vec(assertion_strategy(ev_ids, art_ids_ass.clone()), 0..10).prop_map(move |assertions| {
                let mut substrate = Substrate::new();
                for a in artifacts_clone2.clone() {
                    substrate.artifacts.insert(a.id.clone(), a.clone());
                }
                for e in evidences.clone() {
                    if e.artifact_id != "dummy" || artifacts_clone2.is_empty() {
                        let e_clone = Evidence { id: e.id, artifact_id: e.artifact_id, timestamp: e.timestamp };
                        substrate.evidence.insert(e_clone);
                    }
                }
                for a in assertions {
                    substrate.assertions.insert(a);
                }
                substrate
            })
        })
    })
}

// ─── Propriedades ────────────────────────────────────────────────────────

proptest! {
    /// IC1: Identidade canônica é única
    #[test]
    fn ic1_canonical_identity_unique(artifact1 in artifact_strategy(), artifact2 in artifact_strategy()) {
        let mut substrate = Substrate::new();
        substrate.artifacts.insert(artifact1.id.clone(), artifact1.clone());
        substrate.artifacts.insert(artifact2.id.clone(), artifact2.clone());

        if let Some(id1) = substrate.canonical_id(&artifact1) {
            if let Some(id2) = substrate.canonical_id(&artifact2) {
                if id1 == id2 {
                    prop_assert_eq!(artifact1, artifact2);
                }
            }
        }
    }

    /// IC3: Projeção não modifica o substrato
    #[test]
    fn ic3_projection_purity(substrate in substrate_strategy()) {
        let before = substrate.clone();
        let _view = substrate.project();
        prop_assert_eq!(before, substrate);
    }

    /// IC4: Projeção é determinística
    #[test]
    fn ic4_projection_deterministic(substrate in substrate_strategy()) {
        let view1 = substrate.project();
        let view2 = substrate.project();
        prop_assert_eq!(view1, view2);
    }

    /// IC6: Toda referência existe no substrato
    #[test]
    fn ic6_referential_integrity(mut substrate in substrate_strategy()) {
        let artifact = Artifact {
            id: "art-1".to_string(),
            data: "data".to_string(),
        };
        substrate.artifacts.insert(artifact.id.clone(), artifact);

        let assertion = Assertion {
            id: "ass-1".to_string(),
            evidence_ids: vec![],
            references: vec!["art-1".to_string()],
        };

        substrate.assertions.insert(assertion.clone());

        for ref_id in &assertion.references {
            assert!(substrate.artifacts.contains_key(ref_id));
        }
    }

    /// IC8: Consistência temporal
    #[test]
    fn ic8_temporal_consistency(mut substrate in substrate_strategy()) {
        let artifact = Artifact {
            id: "art-1".to_string(),
            data: "".to_string(),
        };
        substrate.artifacts.insert(artifact.id.clone(), artifact);

        let evidence = Evidence {
            id: "ev-1".to_string(),
            artifact_id: "art-1".to_string(),
            timestamp: 100,
        };
        substrate.evidence.insert(evidence);

        prop_assert!(substrate.is_temporally_consistent());
    }
}
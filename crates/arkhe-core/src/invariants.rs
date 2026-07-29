use crate::types::*;
use std::collections::HashSet;

pub fn ic8_acyclic(s: &State) -> Result<(), InvariantViolation> {
    for (id, ev) in &s.evidences {
        let mut visited = HashSet::new();
        let mut current = ev.parent_hash.as_ref();

        while let Some(parent_hash) = current {
            if !visited.insert(parent_hash.clone()) {
                return Err(InvariantViolation::Ic8CycleDetected {
                    evidence_id: *id,
                    cycle_hash: parent_hash.clone(),
                });
            }
            current = s.evidences.values()
                .find(|e| e.hash == *parent_hash)
                .and_then(|e| e.parent_hash.as_ref());
        }
    }
    Ok(())
}

fn trace_set(s: &State, d: &Decision) -> Vec<EvidenceID> {
    let mut res = Vec::new();
    for bid in &d.belief_ids {
        if let Some(bel) = s.beliefs.get(bid) {
            if let Some(cl) = s.claims.get(&bel.claim_id) {
                for eid in &cl.evidence_ids {
                    res.push(*eid);
                }
            }
        }
    }
    res
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InvariantViolation {
    Ic6MissingArtifact { evidence_id: EvidenceID, artifact_id: ArtifactID },
    Ic8CycleDetected { evidence_id: EvidenceID, cycle_hash: Hash },
    Ic10EmptyClaim { claim_id: ClaimID },
    Ic16UntraceableDecision { decision_id: DecisionID, missing_evidence: EvidenceID },
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ic6MissingArtifact { evidence_id, artifact_id } =>
                write!(f, "IC6: Evidence {} references missing artifact {}", evidence_id, artifact_id),
            Self::Ic8CycleDetected { evidence_id, cycle_hash } =>
                write!(f, "IC8: Cycle detected at evidence {}, hash {}", evidence_id, cycle_hash),
            Self::Ic10EmptyClaim { claim_id } =>
                write!(f, "IC10: Claim {} has no evidences", claim_id),
            Self::Ic16UntraceableDecision { decision_id, missing_evidence } =>
                write!(f, "IC16: Decision {} references missing evidence {}", decision_id, missing_evidence),
        }
    }
}

pub fn check_invariants(s: &State) -> Result<(), Vec<InvariantViolation>> {
    let mut violations = Vec::new();

    for (id, ev) in &s.evidences {
        if !s.artifacts.contains_key(&ev.artifact_id) {
            violations.push(InvariantViolation::Ic6MissingArtifact {
                evidence_id: *id,
                artifact_id: ev.artifact_id,
            });
        }
    }

    if let Err(e) = ic8_acyclic(s) {
        violations.push(e);
    }

    for (id, cl) in &s.claims {
        if cl.evidence_ids.is_empty() {
            violations.push(InvariantViolation::Ic10EmptyClaim { claim_id: *id });
        }
    }

    for (id, d) in &s.decisions {
        for eid in trace_set(s, d) {
            if !s.evidences.contains_key(&eid) {
                violations.push(InvariantViolation::Ic16UntraceableDecision {
                    decision_id: *id,
                    missing_evidence: eid,
                });
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

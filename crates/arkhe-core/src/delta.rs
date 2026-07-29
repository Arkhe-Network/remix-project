use crate::types::*;
use crate::hash::Hasher;

#[allow(clippy::too_many_arguments)]
pub fn apply(state: State, event: &Event, hasher: &dyn Hasher) -> Result<State, TransitionError> {
    match event {
        Event::ArtifactAdded(id, p, m) => {
            let mut state = state;
            if state.artifacts.contains_key(id) {
                return Err(TransitionError::IdAlreadyExists);
            }
            let hash = format!("{:x?}", hasher.hash(p.as_bytes()));
            state.artifacts.insert(*id, Artifact {
                payload: p.clone(),
                metadata: m.clone(),
                hash,
            });
            Ok(state)
        }
        Event::EvidenceAdded(id, art_id, c, sig, ts, ph) => {
            apply_evidence(state, *id, *art_id, c.clone(), sig.clone(), *ts, ph.clone(), hasher)
        }
        Event::ClaimAdded(id, p, evs) => {
            let mut state = state;
            if state.claims.contains_key(id) {
                return Err(TransitionError::IdAlreadyExists);
            }
            state.claims.insert(*id, Claim {
                proposition: p.clone(),
                evidence_ids: evs.clone(),
            });
            Ok(state)
        }
        Event::BeliefAdded(id, cid, conf, j) => {
            let mut state = state;
            if state.beliefs.contains_key(id) {
                return Err(TransitionError::IdAlreadyExists);
            }
            state.beliefs.insert(*id, Belief {
                claim_id: *cid,
                confidence: *conf,
                justification: j.clone(),
            });
            Ok(state)
        }
        Event::DecisionAdded(id, g, bids, ts) => {
            let mut state = state;
            if state.decisions.contains_key(id) {
                return Err(TransitionError::IdAlreadyExists);
            }
            state.decisions.insert(*id, Decision {
                goal: g.clone(),
                belief_ids: bids.clone(),
                timestamp: *ts,
            });
            Ok(state)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_evidence(mut state: State, id: EvidenceID, art_id: ArtifactID, c: Payload,
                  sig: Hash, ts: u64, ph: Option<Hash>, hasher: &dyn Hasher)
                  -> Result<State, TransitionError> {
    if state.evidences.contains_key(&id) { return Err(TransitionError::IdAlreadyExists); }
    if !state.artifacts.contains_key(&art_id) { return Err(TransitionError::ReferencedIdNotFound); }

    let pre_hash_ok = match &ph {
        None => true,
        Some(parent) => state.evidences.values().any(|e| e.hash == *parent),
    };
    if !pre_hash_ok { return Err(TransitionError::InvalidParentHash); }

    let content_bytes = c.as_bytes();
    let hash = format!("{:x?}", hasher.hash(content_bytes));

    let ev = Evidence {
        artifact_id: art_id, content: c, signature: sig,
        timestamp: ts, parent_hash: ph, hash
    };
    state.evidences.insert(id, ev);
    Ok(state)
}

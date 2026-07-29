use std::collections::HashMap;

pub type ArtifactID = u64;
pub type EvidenceID = u64;
pub type ClaimID = u64;
pub type BeliefID = u64;
pub type DecisionID = u64;
pub type Hash = String;
pub type Payload = String;
pub type Metadata = String;
pub type Propositions = String;
pub type Justifications = String;
pub type Goals = String;

#[derive(Debug, Clone)]
pub struct Artifact {
    pub payload: Payload,
    pub metadata: Metadata,
    pub hash: Hash,
}

#[derive(Debug, Clone)]
pub struct Evidence {
    pub artifact_id: ArtifactID,
    pub content: Payload,
    pub signature: Hash,
    pub timestamp: u64,
    pub parent_hash: Option<Hash>,
    pub hash: Hash,
}

#[derive(Debug, Clone)]
pub struct Claim {
    pub proposition: Propositions,
    pub evidence_ids: Vec<EvidenceID>,
}

#[derive(Debug, Clone)]
pub struct Belief {
    pub claim_id: ClaimID,
    pub confidence: u8,
    pub justification: Justifications,
}

#[derive(Debug, Clone)]
pub struct Decision {
    pub goal: Goals,
    pub belief_ids: Vec<BeliefID>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub artifacts: HashMap<ArtifactID, Artifact>,
    pub evidences: HashMap<EvidenceID, Evidence>,
    pub claims: HashMap<ClaimID, Claim>,
    pub beliefs: HashMap<BeliefID, Belief>,
    pub decisions: HashMap<DecisionID, Decision>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }
}

pub enum Event {
    ArtifactAdded(ArtifactID, Payload, Metadata),
    EvidenceAdded(EvidenceID, ArtifactID, Payload, Hash, u64, Option<Hash>),
    ClaimAdded(ClaimID, Propositions, Vec<EvidenceID>),
    BeliefAdded(BeliefID, ClaimID, u8, Justifications),
    DecisionAdded(DecisionID, Goals, Vec<BeliefID>, u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    IdAlreadyExists,
    ReferencedIdNotFound,
    InvalidParentHash,
}

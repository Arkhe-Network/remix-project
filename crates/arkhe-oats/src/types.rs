use arkhe_core::types::BeliefID;

pub struct EvaluationContext {
    pub belief_ids: Vec<BeliefID>,
}

pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

use arkhe_core::types::State;
use crate::types::{EvaluationContext, PolicyDecision};
use crate::error::OatsError;
use async_trait::async_trait;

#[async_trait]
pub trait Policy: Send + Sync {
    async fn evaluate(&self, ctx: &EvaluationContext, state: &State) -> Result<PolicyDecision, OatsError>;
}

pub struct MinConfidencePolicy {
    pub threshold: u8,
}

#[async_trait]
impl Policy for MinConfidencePolicy {
    async fn evaluate(&self, ctx: &EvaluationContext, state: &State) -> Result<PolicyDecision, OatsError> {
        if ctx.belief_ids.is_empty() {
            return Ok(PolicyDecision::Deny {
                reason: "Decision requires at least one belief".into()
            });
        }

        for bid in &ctx.belief_ids {
            match state.beliefs.get(bid) {
                Some(belief) if belief.confidence >= self.threshold => continue,
                Some(belief) => {
                    return Ok(PolicyDecision::Deny {
                        reason: format!("Belief {} confidence {} < threshold {}",
                            bid, belief.confidence, self.threshold)
                    });
                }
                None => {
                    return Ok(PolicyDecision::Deny {
                        reason: format!("Belief {} not found in state", bid)
                    });
                }
            }
        }

        Ok(PolicyDecision::Allow)
    }
}

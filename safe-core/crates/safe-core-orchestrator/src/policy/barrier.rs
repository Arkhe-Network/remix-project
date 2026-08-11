#[derive(Debug, Clone)]
pub struct BarrierChecker;

impl BarrierChecker {
    pub fn new() -> Self {
        Self
    }

    pub fn classify(&self, claim: &crate::level25::TheoremClaim) -> BarrierVerdict {
        // Simple stub: check if domain is known to be barred
        if claim.domain == "barred-domain" {
            BarrierVerdict::Barred {
                model: "stub-model".to_string(),
                reason: "barred domain".to_string(),
                confidence: 0.9
            }
        } else {
            BarrierVerdict::Pass
        }
    }
}

pub enum BarrierVerdict {
    Pass,
    Barred {
        model: String,
        reason: String,
        confidence: f64,
    },
}

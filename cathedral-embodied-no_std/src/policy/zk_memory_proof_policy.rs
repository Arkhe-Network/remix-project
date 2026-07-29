
use serde::{Serialize, Deserialize};
#[derive(Default, Serialize, Deserialize)]
pub struct ZkMemoryProofPolicy {
    pub require_memory_proof_for_recommendations: bool,
}
impl ZkMemoryProofPolicy {
    pub fn should_require_memory_proof_for_recommendation(&self, _hub: Option<&str>, _value: f64) -> bool {
        self.require_memory_proof_for_recommendations
    }
}

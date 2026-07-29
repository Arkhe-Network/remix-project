//! Policy structures that can be mutated by plausible reasoning.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Condition {
    StepCountModulo(u32),
    ActionRisk(String),        // "high", "medium", "low"
    UserTrustLevel(String),    // "high", "medium", "low"
    LastProofFailed(bool),
    ConfidenceBelow(f32),
    InterferenceAbove(f32),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum PolicyAction {
    Prove,
    #[default]
    Skip,
    ProveAndVerifyOnChain,
    EmergencyHalt,
    ConservativeMode,
    AdjustThreshold { field: String, value: f32 },
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PolicyRule {
    pub id: u32,
    pub condition: Condition,
    pub action: PolicyAction,
    pub priority: u8,
}

/// The complete policy (list of rules, with ordering by priority).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Policy {
    pub rules: Vec<PolicyRule>,
    pub default_action: PolicyAction,
}

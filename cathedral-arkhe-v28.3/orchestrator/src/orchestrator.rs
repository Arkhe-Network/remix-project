use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub agent_id: String,
    pub action_type: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
    pub is_suspicious: bool,
}

#[derive(Debug, Clone)]
pub struct AgentResult {
    pub final_answer: String,
}

pub enum AgentRole {
    Specialist,
    Orchestrator,
}

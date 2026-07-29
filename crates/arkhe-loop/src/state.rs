use serde::{Deserialize, Serialize};

pub type LoopResult = Result<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub max_iterations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopPhase {
    Reasoning,
    Action,
    Reflection,
    Execution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopStep {
    pub phase: LoopPhase,
    pub iteration: u64,
    pub input: String,
    pub output: String,
    pub duration_ms: u64,
    pub timestamp: arkhe_core::Timestamp,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopSnapshot {
    pub config: LoopConfig,
    pub iterations: u64,
}

pub struct LoopState {
    pub config: LoopConfig,
    pub iterations: u64,
}

impl LoopState {
    pub fn new(config: LoopConfig) -> Self {
        Self {
            config,
            iterations: 0,
        }
    }

    pub fn check_limits(&self) -> Result<(), String> {
        if self.iterations >= self.config.max_iterations {
            return Err("Max iterations reached".to_string());
        }
        Ok(())
    }

    pub fn snapshot(&self) -> LoopSnapshot {
        LoopSnapshot {
            config: self.config.clone(),
            iterations: self.iterations,
        }
    }
}

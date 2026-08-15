// crates/safe-core-pea/src/lib.rs
use safe_core_identity::ArkheDid;

pub struct PolicyEngine;

pub enum TaskState {
    Running,
}

pub struct Intent;

impl Intent {
    pub fn new_root(_hash: &blake3::Hash, _did: ArkheDid, _task: &str, _data: &[u8]) -> Self {
        Self
    }

    pub fn advance_state(&mut self, _state: TaskState, _metadata: &()) -> Result<(), ()> {
        Ok(())
    }
}

use crate::{NamespaceId, SessionId};
use chrono::{DateTime, Utc};
use std::ptr;

pub struct ContextState {
    pub namespace: NamespaceId,
    pub session_id: SessionId,
    pub payload: String,
    pub created_at: DateTime<Utc>,
    pub iterations: u32,
}

impl ContextState {
    pub fn new(namespace: NamespaceId, session_id: SessionId, prompt: &str) -> Self {
        Self {
            namespace,
            session_id,
            payload: prompt.to_string(),
            created_at: Utc::now(),
            iterations: 0,
        }
    }

    pub fn increment_iteration(&mut self) {
        self.iterations += 1;
    }

    pub fn age_seconds(&self) -> u64 {
        let now = Utc::now();
        (now - self.created_at).num_seconds().max(0) as u64
    }
}

// Implementação determinística de Burn-After-Use (BAU) via RAII
impl Drop for ContextState {
    fn drop(&mut self) {
        unsafe {
            let bytes = self.payload.as_bytes_mut();
            for byte in bytes.iter_mut() {
                ptr::write_volatile(byte, 0u8);
            }
        }
        tracing::info!(
            "BAU Memory Action: Volatile zeroization executed on context heap buffer for session: {:?}",
            self.session_id
        );
    }
}

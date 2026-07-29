use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditAction {
    Create,
    Read,
    Inject,
    Destroy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub action: AuditAction,
    pub detail: String,
}

pub struct AuditTrail {
    logs: VecDeque<AuditRecord>,
    max_size: usize,
}

impl AuditTrail {
    pub fn new(max_size: usize) -> Self {
        Self {
            logs: VecDeque::new(),
            max_size,
        }
    }

    pub fn log(&mut self, record: AuditRecord) {
        if self.logs.len() >= self.max_size {
            self.logs.pop_front();
        }
        self.logs.push_back(record);
    }

    pub fn records(&self) -> &VecDeque<AuditRecord> {
        &self.logs
    }
}

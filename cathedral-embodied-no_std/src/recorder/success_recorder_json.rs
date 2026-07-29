
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Record {
    pub round: u32,
    pub acceptance_rate: f32,
    pub memory_proof_used: bool,
}

pub struct JsonRecorder {
    pub records: Mutex<Vec<Record>>,
    path: Option<String>,
}

impl JsonRecorder {
    pub fn new(path: Option<&str>) -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            path: path.map(|s| s.to_string()),
        }
    }
    pub fn record_round(&self, round: u32, acceptance_rate: f32, memory_proof_used: bool) {
        self.records.lock().unwrap().push(Record { round, acceptance_rate, memory_proof_used });
    }
    pub fn average_acceptance_rate(&self, _last_n: Option<usize>) -> f32 {
        0.0
    }
    pub fn memory_proof_usage_rate(&self, _last_n: Option<usize>) -> f32 {
        let records = self.records.lock().unwrap();
        if records.is_empty() { return 0.0; }
        let total = records.len() as f32;
        let used = records.iter().filter(|r| r.memory_proof_used).count() as f32;
        used / total
    }
    pub fn flush(&self) {}
}

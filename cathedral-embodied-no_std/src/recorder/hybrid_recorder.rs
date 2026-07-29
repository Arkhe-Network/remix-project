
use crate::recorder::success_recorder_json::JsonRecorder;
use crate::recorder::success_recorder_sqlite::SqliteRecorder;

#[derive(Debug)]
pub enum RecorderError {
    Sqlite(rusqlite::Error),
    Json(String),
    NotInitialized,
}

impl std::fmt::Display for RecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(e) => write!(f, "SQLite error: {}", e),
            Self::Json(s) => write!(f, "JSON error: {}", s),
            Self::NotInitialized => write!(f, "Recorder not initialized"),
        }
    }
}

impl From<rusqlite::Error> for RecorderError {
    fn from(e: rusqlite::Error) -> Self {
        RecorderError::Sqlite(e)
    }
}

pub struct HybridRecorder {
    inner: Box<dyn RecorderOps + Send + Sync>,
}

pub trait RecorderOps: Send + Sync {
    fn record_round(&self, round: u32, acceptance_rate: f32, memory_proof_used: bool) -> Result<(), RecorderError>;
    fn record_hub_performance(&self, hub: &str, round: u32, acceptance_rate: f32, volume: u32, roas: f32) -> Result<(), RecorderError>;
    fn record_recommendation(&self, round: u32, hub: &str, title: &str, url: &str, accepted: bool) -> Result<(), RecorderError>;
    fn average_acceptance_rate(&self, last_n: Option<u32>) -> Result<f32, RecorderError>;
    fn recent_hub_stats(&self, last_rounds: u32) -> Result<Vec<(String, f32, u32, f32)>, RecorderError>;
    fn memory_proof_usage_rate(&self, last_n: Option<u32>) -> Result<f32, RecorderError>;
    fn flush(&self) {}
}

impl HybridRecorder {
    pub fn new(db_path: Option<&str>, json_path: Option<&str>) -> Result<Self, RecorderError> {
        if let Some(db) = db_path {
            match SqliteRecorder::new(db) {
                Ok(rec) => return Ok(Self { inner: Box::new(rec) }),
                Err(e) => eprintln!("[HybridRecorder] SQLite unavailable: {}, trying JSON...", e),
            }
        }

        if let Some(json) = json_path {
            let rec = JsonRecorder::new(Some(json));
            return Ok(Self { inner: Box::new(rec) });
        }

        let rec = JsonRecorder::new(None);
        Ok(Self { inner: Box::new(rec) })
    }

    pub fn record_round(&self, round: u32, acceptance_rate: f32, memory_proof_used: bool) -> Result<(), RecorderError> {
        self.inner.record_round(round, acceptance_rate, memory_proof_used)
    }
    pub fn recent_hub_stats(&self, last_rounds: u32) -> Result<Vec<(String, f32, u32, f32)>, RecorderError> {
        self.inner.recent_hub_stats(last_rounds)
    }
    pub fn flush(&self) {
        self.inner.flush();
    }
}

impl RecorderOps for SqliteRecorder {
    fn record_round(&self, round: u32, acceptance_rate: f32, memory_proof_used: bool) -> Result<(), RecorderError> {
        self.record_round(round, acceptance_rate, memory_proof_used).map_err(Into::into)
    }
    fn record_hub_performance(&self, hub: &str, round: u32, acceptance_rate: f32, volume: u32, roas: f32) -> Result<(), RecorderError> {
        self.record_hub_performance(hub, round, acceptance_rate, volume, roas).map_err(Into::into)
    }
    fn record_recommendation(&self, round: u32, hub: &str, title: &str, url: &str, accepted: bool) -> Result<(), RecorderError> {
        self.record_recommendation(round, hub, title, url, accepted).map_err(Into::into)
    }
    fn average_acceptance_rate(&self, last_n: Option<u32>) -> Result<f32, RecorderError> {
        self.average_acceptance_rate(last_n).map_err(Into::into)
    }
    fn recent_hub_stats(&self, last_rounds: u32) -> Result<Vec<(String, f32, u32, f32)>, RecorderError> {
        self.recent_hub_stats(last_rounds).map_err(Into::into)
    }
    fn memory_proof_usage_rate(&self, last_n: Option<u32>) -> Result<f32, RecorderError> {
        self.memory_proof_usage_rate(last_n).map_err(Into::into)
    }
}

impl RecorderOps for JsonRecorder {
    fn record_round(&self, round: u32, acceptance_rate: f32, memory_proof_used: bool) -> Result<(), RecorderError> {
        self.record_round(round, acceptance_rate, memory_proof_used);
        Ok(())
    }
    fn record_hub_performance(&self, _hub: &str, _round: u32, _acceptance_rate: f32, _volume: u32, _roas: f32) -> Result<(), RecorderError> {
        Ok(())
    }
    fn record_recommendation(&self, _round: u32, _hub: &str, _title: &str, _url: &str, _accepted: bool) -> Result<(), RecorderError> {
        Ok(())
    }
    fn average_acceptance_rate(&self, last_n: Option<u32>) -> Result<f32, RecorderError> {
        Ok(self.average_acceptance_rate(last_n.map(|n| n as usize)))
    }
    fn recent_hub_stats(&self, _last_rounds: u32) -> Result<Vec<(String, f32, u32, f32)>, RecorderError> {
        Ok(Vec::new())
    }
    fn memory_proof_usage_rate(&self, last_n: Option<u32>) -> Result<f32, RecorderError> {
        Ok(self.memory_proof_usage_rate(last_n.map(|n| n as usize)))
    }
    fn flush(&self) {
        self.flush();
    }
}

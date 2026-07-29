
use rusqlite::{params, Connection, Result as SqlResult};
use std::path::Path;
use std::sync::Mutex;

pub struct SqliteRecorder {
    conn: Mutex<Connection>,
}

impl SqliteRecorder {
    pub fn new(db_path: &str) -> SqlResult<Self> {
        let conn = Connection::open(Path::new(db_path))?;
        conn.execute_batch(include_str!("success_recorder_schema.sql"))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn record_round(&self, round: u32, acceptance_rate: f32, memory_proof_used: bool) -> SqlResult<()> {
        self.conn.lock().unwrap().execute(
            "INSERT OR REPLACE INTO rounds (round, acceptance, proof_used) VALUES (?1, ?2, ?3)",
            params![round, acceptance_rate, memory_proof_used as i32],
        )?;
        Ok(())
    }

    pub fn record_hub_performance(&self, hub: &str, round: u32, acceptance_rate: f32, volume: u32, roas: f32) -> SqlResult<()> {
        self.conn.lock().unwrap().execute(
            "INSERT OR REPLACE INTO hub_performance (hub, round, acceptance_rate, volume, roas) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![hub, round, acceptance_rate, volume, roas],
        )?;
        Ok(())
    }

    pub fn record_recommendation(&self, round: u32, hub: &str, title: &str, url: &str, accepted: bool) -> SqlResult<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO recommendations (round, hub, title, url, accepted) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![round, hub, title, url, accepted as i32],
        )?;
        Ok(())
    }

    pub fn average_acceptance_rate(&self, last_n: Option<u32>) -> SqlResult<f32> {
        let sql = match last_n {
            Some(n) => format!("SELECT AVG(acceptance) FROM rounds ORDER BY round DESC LIMIT {}", n),
            None => "SELECT AVG(acceptance) FROM rounds".to_string(),
        };
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let avg: Option<f32> = stmt.query_row([], |row| row.get(0))?;
        Ok(avg.unwrap_or(0.0))
    }

    pub fn recent_hub_stats(&self, last_rounds: u32) -> SqlResult<Vec<(String, f32, u32, f32)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT hub, AVG(acceptance_rate), SUM(volume), AVG(roas) FROM hub_performance WHERE round >= (SELECT MAX(round) - ?1 + 1 FROM rounds) GROUP BY hub"
        )?;
        let rows = stmt.query_map(params![last_rounds], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, f32>(1)?, row.get::<_, u32>(2)?, row.get::<_, f32>(3)?))
        })?;
        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn memory_proof_usage_rate(&self, last_n: Option<u32>) -> SqlResult<f32> {
        let sql = match last_n {
            Some(n) => format!("SELECT AVG(proof_used) FROM rounds ORDER BY round DESC LIMIT {}", n),
            None => "SELECT AVG(proof_used) FROM rounds".to_string(),
        };
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;
        let rate: Option<f32> = stmt.query_row([], |row| row.get(0))?;
        Ok(rate.unwrap_or(0.0))
    }
}

//! Prolog bridge — **EXPERIMENTAL / NON-FUNCTIONAL STUB**.

use serde_json::Value;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PrologError {
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Prolog error: {0}")]
    Prolog(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct PrologClient {
    #[allow(dead_code)]
    stream: UnixStream,
    #[allow(dead_code)]
    reader: BufReader<UnixStream>,
    #[allow(dead_code)]
    id: u64,
}

impl PrologClient {
    pub fn connect(socket_path: &Path) -> Result<Self, PrologError> {
        let stream = UnixStream::connect(socket_path)
            .map_err(|e| PrologError::Connection(e.to_string()))?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Self { stream, reader, id: 0 })
    }

    pub fn query(&mut self, _goal: &str) -> Result<Value, PrologError> {
        unimplemented!("Prolog bridge is a non-functional stub")
    }

    pub fn embed_state(&mut self, _state: &crate::invariants::SystemState) -> Result<Value, PrologError> {
        unimplemented!("Prolog bridge is a non-functional stub")
    }

    pub fn classify_escape(&mut self, _state: &crate::invariants::SystemState) -> Result<String, PrologError> {
        unimplemented!("Prolog bridge is a non-functional stub")
    }
}

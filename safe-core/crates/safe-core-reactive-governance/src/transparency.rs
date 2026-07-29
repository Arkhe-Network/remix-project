//! Transparency Log mock for Reactive Governance

use hash::Hasher;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Append failed")]
    AppendFailed,
}

pub struct TransparencyLog<H: Hasher> {
    _marker: std::marker::PhantomData<H>,
}

impl<H: Hasher> TransparencyLog<H> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    pub fn append(
        &mut self,
        _issued_by: &str,
        _action: &str,
        _timestamp: i64,
        _payload_hash: &str,
        _signature: &[u8],
    ) -> Result<u64, DbError> {
        Ok(1)
    }
}

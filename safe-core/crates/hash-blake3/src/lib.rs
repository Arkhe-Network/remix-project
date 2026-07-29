//! Safe-Core Hash
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HashError {
    #[error("Hashing failed: {0}")]
    Internal(String),
}

pub trait Hasher: Send + Sync {
    fn update(&mut self, data: &[u8]);
    fn finalize(self) -> [u8; 32];
    fn hash(data: &[u8]) -> [u8; 32] where Self: Sized;
}

#[cfg(feature = "blake3")]
pub struct Blake3Hasher {
    state: blake3::Hasher,
}

#[cfg(feature = "blake3")]
impl Blake3Hasher {
    pub fn new() -> Self {
        Self { state: blake3::Hasher::new() }
    }
}

#[cfg(feature = "blake3")]
impl Hasher for Blake3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }
    fn finalize(self) -> [u8; 32] {
        self.state.finalize().into()
    }
    fn hash(data: &[u8]) -> [u8; 32] {
        blake3::hash(data).into()
    }
}

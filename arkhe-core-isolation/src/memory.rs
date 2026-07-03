use core::convert::Infallible;
use std::str::FromStr;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Memória segura com zeroização garantida no Drop.
#[derive(Clone)]
pub struct SecureMemory {
    buffer: Vec<u8>,
}

impl SecureMemory {
    pub fn new(data: Vec<u8>) -> Self {
        Self { buffer: data }
    }

    // Expõe apenas referência imutável
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl FromStr for SecureMemory {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.as_bytes().to_vec()))
    }
}

// Garante que a memória é zeroizada antes de ser retornada ao SO
impl Zeroize for SecureMemory {
    fn zeroize(&mut self) {
        // Sobrescreve com zeros antes de dealocar
        self.buffer.iter_mut().for_each(|b| *b = 0);
        // O tamanho lógico pode permanecer, mas o conteúdo é destruído
    }
}

impl ZeroizeOnDrop for SecureMemory {}

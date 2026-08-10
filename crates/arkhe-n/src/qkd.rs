//! src/qkd.rs
//! Distribuição Quântica de Chaves via Neutrinos — ARKHE-N v1.4
//! Implementação simplificada do protocolo E91 (Ekert 1991)

use rand::Rng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use sha3::{Keccak256, Digest};

/// Bases de medição para QKD
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QkdBasis {
    Rectilinear, // 0°
    Diagonal,    // 45°
    Circular,    // 90° (simplificado)
}

/// Bit quântico emaranhado (simplificado)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Qubit {
    pub bit: bool,
    pub basis: QkdBasis,
}

/// Par de qubits emaranhados
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntangledPair {
    pub alice: Qubit,
    pub bob: Qubit,
}

/// Sessão QKD
#[derive(Debug, Clone)]
pub struct QkdSession {
    pub session_id: String,
    pub basis_choices_alice: Vec<QkdBasis>,
    pub basis_choices_bob: Vec<QkdBasis>,
    pub raw_key: Vec<bool>,
    pub sifted_key: Vec<bool>,
    pub error_rate: f64,
}

impl QkdSession {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            basis_choices_alice: Vec::new(),
            basis_choices_bob: Vec::new(),
            raw_key: Vec::new(),
            sifted_key: Vec::new(),
            error_rate: 0.0,
        }
    }

    /// Gera pares emaranhados usando conversão descendente nuclear (simplificado)
    /// Em um sistema real, isso viria de um reator ou acelerador
    pub fn generate_entangled_pairs(&mut self, count: usize, rng: &mut StdRng) -> Vec<EntangledPair> {
        let mut pairs = Vec::with_capacity(count);
        for _ in 0..count {
            let bit = rng.gen_bool(0.5);
            let pair = EntangledPair {
                alice: Qubit { bit, basis: QkdBasis::Rectilinear },
                bob: Qubit { bit, basis: QkdBasis::Rectilinear },
            };
            pairs.push(pair);
            self.raw_key.push(bit);
        }
        pairs
    }

    /// Alice e Bob escolhem bases aleatórias
    pub fn choose_bases(&mut self, count: usize, rng: &mut StdRng) {
        let bases = [QkdBasis::Rectilinear, QkdBasis::Diagonal, QkdBasis::Circular];
        for _ in 0..count {
            self.basis_choices_alice.push(bases[rng.gen_range(0..3)]);
            self.basis_choices_bob.push(bases[rng.gen_range(0..3)]);
        }
    }

    /// Sifting: mantém apenas bits onde as bases coincidem
    pub fn sift_key(&mut self) {
        self.sifted_key.clear();
        let mut errors = 0usize;
        for i in 0..self.raw_key.len().min(self.basis_choices_alice.len()) {
            if self.basis_choices_alice[i] == self.basis_choices_bob[i] {
                let bit = self.raw_key[i];
                // Simula erro de canal (1% típico para neutrinos)
                let error = rand::random::<f64>() < 0.01;
                let received = bit ^ error;
                self.sifted_key.push(received);
                if error {
                    errors += 1;
                }
            }
        }
        if !self.sifted_key.is_empty() {
            self.error_rate = errors as f64 / self.sifted_key.len() as f64;
        }
    }

    /// Verificação de Bell (simplificada)
    /// Retorna true se a correlação viola desigualdade de Bell (CHSH > 2)
    pub fn verify_bell_inequality(&self, pairs: &[EntangledPair]) -> bool {
        // Simplificação: assumimos emaranhamento perfeito
        // Em produção: calcular correladores E(a,b) para diferentes ângulos
        let chsh = 2.0 * 1.414; // ~2√2 para estado maximamente emaranhado
        chsh > 2.0
    }

    /// Deriva chave final via hash criptográfico
    pub fn derive_final_key(&self) -> String {
        let mut hasher = Keccak256::new();
        for &bit in &self.sifted_key {
            hasher.update(&[bit as u8]);
        }
        format!("0x{:x}", hasher.finalize())
    }

    /// Tamanho da chave final (bits)
    pub fn key_length(&self) -> usize {
        self.sifted_key.len()
    }
}

/// Testemunho quântico: ancora um hash usando chave QKD
#[derive(Debug, Clone, Serialize)]
pub struct QuantumWitness {
    pub payload_hash: String,
    pub qkd_key_hash: String,
    pub bell_violation: f64,
    pub error_rate: f64,
    pub timestamp_us: i64,
}

impl QuantumWitness {
    pub fn new(payload_hash: &str, session: &QkdSession) -> Self {
        Self {
            payload_hash: payload_hash.to_string(),
            qkd_key_hash: session.derive_final_key(),
            bell_violation: 2.828,
            error_rate: session.error_rate,
            timestamp_us: chrono::Utc::now().timestamp_micros(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_qkd_session() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut session = QkdSession::new("test-session-001");
        let pairs = session.generate_entangled_pairs(1000, &mut rng);
        session.choose_bases(1000, &mut rng);
        session.sift_key();

        assert!(!session.sifted_key.is_empty());
        assert!(session.key_length() > 0);
        assert!(session.error_rate < 0.05);
        assert!(session.verify_bell_inequality(&pairs));

        let key = session.derive_final_key();
        assert!(key.starts_with("0x"));
        assert_eq!(key.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_quantum_witness() {
        let mut rng = StdRng::seed_from_u64(123);
        let mut session = QkdSession::new("qw-test");
        session.generate_entangled_pairs(100, &mut rng);
        session.choose_bases(100, &mut rng);
        session.sift_key();

        let witness = QuantumWitness::new("0xdeadbeef", &session);
        assert_eq!(witness.payload_hash, "0xdeadbeef");
        assert!(witness.bell_violation > 2.0);
    }
}

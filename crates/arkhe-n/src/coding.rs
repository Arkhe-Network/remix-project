//! src/coding.rs
//! Codificação FEC (LDPC stub) + CRC-32 + Monte Carlo BER

use crc32fast::Hasher as Crc32Hasher;
use serde::{Deserialize, Serialize};

/// Pacote com CRC-32 para integridade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrcPacket {
    pub payload: Vec<u8>,
    pub crc32: u32,
}

impl CrcPacket {
    pub fn new(payload: Vec<u8>) -> Self {
        let crc32 = Self::compute_crc(&payload);
        Self { payload, crc32 }
    }

    pub fn compute_crc(data: &[u8]) -> u32 {
        let mut hasher = Crc32Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }

    pub fn verify(&self) -> bool {
        Self::compute_crc(&self.payload) == self.crc32
    }

    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }
}

/// Codec LDPC (stub para integração — substituir por crate real em produção)
#[derive(Debug, Clone)]
pub struct LdpcCodec {
    pub block_size: usize,
    pub code_rate: f64,
}

impl LdpcCodec {
    pub fn new(block_size: usize, code_rate: f64) -> Self {
        assert!(code_rate > 0.0 && code_rate <= 1.0);
        Self { block_size, code_rate }
    }

    pub fn new_4ppm_optimized() -> Self {
        Self::new(1024, 0.5)
    }

    /// Codifica dados com LDPC (stub: adiciona redundância simples)
    pub fn encode(&self, data: &[u8]) -> Vec<u8> {
        let mut encoded = data.to_vec();
        // Stub: duplica os dados como redundância
        // Em produção: usar algoritmo LDPC real (sum-product, belief propagation)
        encoded.extend_from_slice(data);
        encoded
    }

    /// Decodifica LLRs (stub: thresholding simples)
    pub fn decode(&self, llrs: &[f64]) -> (Vec<u8>, bool) {
        // Converte LLRs em bits (hard decision)
        let bits: Vec<u8> = llrs.iter().map(|&llr| if llr > 0.0 { 1 } else { 0 }).collect();

        // Agrupa em bytes
        let mut bytes = Vec::with_capacity(bits.len() / 8);
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                if b == 1 {
                    byte |= 1 << (7 - i);
                }
            }
            bytes.push(byte);
        }

        // Verifica redundância (stub: compara primeira e segunda metade)
        let half = bytes.len() / 2;
        let syndrome_ok = bytes[..half] == bytes[half..];

        (bytes[..half.min(bytes.len())].to_vec(), syndrome_ok)
    }

    /// Taxa de código (informação / total)
    pub fn rate(&self) -> f64 {
        self.code_rate
    }
}

/// Resultado de simulação Monte Carlo
#[derive(Debug, Clone, Serialize)]
pub struct MonteCarloResult {
    pub mode: String,
    pub iterations: usize,
    pub ber: f64,           // Bit Error Rate
    pub fer: f64,           // Frame Error Rate
    pub capacity_estimated: f64,
    pub energy_per_bit_j: f64,
    pub confidence_interval_95: (f64, f64),
}

/// Simulador Monte Carlo para BER/Capacidade
pub struct MonteCarloSimulator;

impl MonteCarloSimulator {
    /// Executa simulação Monte Carlo para um dado canal
    /// `transmitter`: função que transmite um bit e retorna (detected, correct)
    pub fn run_ber<F>(
        mode_name: &str,
        iterations: usize,
        energy_per_pulse: f64,
        mut transmitter: F,
    ) -> MonteCarloResult
    where
        F: FnMut(bool) -> (bool, bool), // (detected_value, was_correct)
    {
        let mut bit_errors = 0usize;
        let mut frame_errors = 0usize;
        let frame_size = 1024;

        for i in 0..iterations {
            let bit = i % 2 == 0;
            let (_, correct) = transmitter(bit);
            if !correct {
                bit_errors += 1;
            }
            if i > 0 && i % frame_size == 0 && bit_errors > 0 {
                frame_errors += 1;
                // bit_errors is NOT reset here so we keep total errors for BER.
            }
        }

        let ber = bit_errors as f64 / iterations as f64;
        let fer = frame_errors as f64 / (iterations / frame_size).max(1) as f64;

        // Intervalo de confiança 95% (aproximação normal)
        let p = ber;
        let z = 1.96;
        let margin = z * (p * (1.0 - p) / iterations as f64).sqrt();
        let ci_low = (p - margin).max(0.0);
        let ci_high = (p + margin).min(1.0);

        MonteCarloResult {
            mode: mode_name.to_string(),
            iterations,
            ber,
            fer,
            capacity_estimated: (1.0 - ber).max(0.0),
            energy_per_bit_j: energy_per_pulse,
            confidence_interval_95: (ci_low, ci_high),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc_packet() {
        let packet = CrcPacket::new(b"ARKHE".to_vec());
        assert!(packet.verify());

        let mut tampered = packet.clone();
        tampered.payload[0] = 0xFF;
        assert!(!tampered.verify());
    }

    #[test]
    fn test_ldpc_roundtrip() {
        let codec = LdpcCodec::new_4ppm_optimized();
        let data = b"test data for ldpc";
        let encoded = codec.encode(data);
        // Simula LLRs perfeitos
        let llrs: Vec<f64> = encoded.iter().flat_map(|&b| (0..8).rev().map(move |i| if (b >> i) & 1 == 1 { 5.0 } else { -5.0 })).collect();
        let (decoded, success) = codec.decode(&llrs);
        assert!(success);
        assert_eq!(&decoded[..data.len()], data.as_slice());
    }

    #[test]
    fn test_monte_carlo_perfect_channel() {
        let result = MonteCarloSimulator::run_ber(
            "perfect",
            10000,
            1.0,
            |_bit| (true, true), // sempre correto
        );
        assert_eq!(result.ber, 0.0);
        assert_eq!(result.capacity_estimated, 1.0);
    }

    #[test]
    fn test_monte_carlo_noisy_channel() {
        let result = MonteCarloSimulator::run_ber(
            "noisy",
            10000,
            1.0,
            |bit| {
                // 10% de erro
                let err = rand::random::<f64>() < 0.1;
                (bit ^ err, !err)
            },
        );
        assert!(result.ber > 0.05 && result.ber < 0.15,
            "BER {} should be ~0.1", result.ber);
    }
}

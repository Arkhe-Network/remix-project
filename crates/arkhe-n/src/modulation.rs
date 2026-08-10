//! src/modulation.rs
//! Modulação M-PPM generalizada — ARKHE-N v1.4
//! Respeita a semântica de slots posicionais do PPM.

use serde::{Deserialize, Serialize};

/// Símbolo M-PPM: representa o índice do slot ativo (0..M-1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PpmSymbol {
    pub slot: usize,
    pub m: u8, // número total de slots (M)
}

impl PpmSymbol {
    /// Cria um símbolo PPM válido
    pub fn new(slot: usize, m: u8) -> Result<Self, &'static str> {
        if slot >= m as usize {
            return Err("Slot index out of bounds");
        }
        Ok(Self { slot, m })
    }

    /// Bits por símbolo = log2(M)
    pub fn bits_per_symbol(&self) -> usize {
        (self.m as f64).log2() as usize
    }

    /// Converte símbolo para bits (little-endian)
    pub fn to_bits(&self) -> Vec<bool> {
        let bits_count = self.bits_per_symbol();
        let mut bits = Vec::with_capacity(bits_count);
        let mut value = self.slot;
        for _ in 0..bits_count {
            bits.push(value & 1 == 1);
            value >>= 1;
        }
        bits
    }

    /// Converte bits (little-endian) para símbolo
    pub fn from_bits(bits: &[bool], m: u8) -> Result<Self, &'static str> {
        let expected = (m as f64).log2() as usize;
        if bits.len() != expected {
            return Err("Bit count mismatch for M-PPM");
        }
        let mut slot = 0usize;
        for (i, &b) in bits.iter().enumerate() {
            if b {
                slot |= 1 << i;
            }
        }
        if slot >= m as usize {
            return Err("Decoded slot exceeds M");
        }
        Ok(Self { slot, m })
    }
}

pub struct PpmModem {
    pub m: u8,
}

impl PpmModem {
    pub fn new(m: u8) -> Result<Self, &'static str> {
        if !m.is_power_of_two() || m < 2 {
            return Err("M must be a power of 2 and >= 2");
        }
        Ok(Self { m })
    }

    /// Codifica bytes em símbolos M-PPM
    pub fn encode(&self, data: &[u8]) -> Vec<PpmSymbol> {
        let bits_per_sym = (self.m as f64).log2() as usize;
        let mut symbols = Vec::with_capacity(data.len() * 8 / bits_per_sym);
        let mut bit_buffer: Vec<bool> = Vec::with_capacity(bits_per_sym);

        for &byte in data {
            for i in 0..8 {
                bit_buffer.push((byte >> i) & 1 == 1);
                if bit_buffer.len() == bits_per_sym {
                    let sym = PpmSymbol::from_bits(&bit_buffer, self.m).unwrap();
                    symbols.push(sym);
                    bit_buffer.clear();
                }
            }
        }

        // Padding com zeros se necessário
        if !bit_buffer.is_empty() {
            while bit_buffer.len() < bits_per_sym {
                bit_buffer.push(false);
            }
            let sym = PpmSymbol::from_bits(&bit_buffer, self.m).unwrap();
            symbols.push(sym);
        }

        symbols
    }

    /// Decodifica símbolos M-PPM em bytes
    pub fn decode(&self, symbols: &[PpmSymbol]) -> Vec<u8> {
        let bits_per_sym = (self.m as f64).log2() as usize;
        let mut bits: Vec<bool> = Vec::with_capacity(symbols.len() * bits_per_sym);
        for sym in symbols {
            bits.extend(sym.to_bits());
        }

        let mut bytes = Vec::with_capacity(bits.len() / 8);
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &b) in chunk.iter().enumerate() {
                if b {
                    byte |= 1 << i;
                }
            }
            bytes.push(byte);
        }
        bytes
    }

    /// Simula transmissão de um símbolo pelo canal Poisson
    /// Retorna: (símbolo_detectado, llrs_por_slot)
    pub fn simulate_transmission<F>(
        &self,
        symbol: &PpmSymbol,
        mut slot_transmitter: F,
    ) -> (PpmSymbol, Vec<f64>)
    where
        F: FnMut(usize) -> (bool, f64),
    {
        let mut llrs = vec![0.0; self.m as usize];
        let mut detected_slot = 0usize;
        let mut max_llr = f64::NEG_INFINITY;

        for slot in 0..self.m as usize {
            let (detected, conf) = slot_transmitter(slot);
            // LLR aproximado para slot: positivo se detectado, negativo se não
            let llr = if detected {
                10.0 * conf
            } else {
                -10.0 * (1.0 - conf)
            };
            llrs[slot] = llr;
            if llr > max_llr {
                max_llr = llr;
                detected_slot = slot;
            }
        }

        (PpmSymbol::new(detected_slot, self.m).unwrap(), llrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppm4_roundtrip() {
        let modem = PpmModem::new(4).unwrap();
        let original = b"Hello, ARKHE-N! \xF0\x9F\x9A\x80";
        let symbols = modem.encode(original);
        let decoded = modem.decode(&symbols);
        assert_eq!(original.to_vec(), decoded);
    }

    #[test]
    fn test_ppm8_roundtrip() {
        let modem = PpmModem::new(8).unwrap();
        let original = b"KM3NeT submarine test";
        let symbols = modem.encode(original);
        let decoded = modem.decode(&symbols);
        assert_eq!(original.to_vec(), decoded);
    }

    #[test]
    fn test_ppm16_roundtrip() {
        let modem = PpmModem::new(16).unwrap();
        let original = b"Cooled muon beam 2026";
        let symbols = modem.encode(original);
        let decoded = modem.decode(&symbols);
        assert_eq!(original.to_vec(), decoded);
    }

    #[test]
    fn test_invalid_m() {
        assert!(PpmModem::new(3).is_err()); // não potência de 2
        assert!(PpmModem::new(1).is_err()); // < 2
        assert!(PpmModem::new(0).is_err());
    }

    #[test]
    fn test_slot_bounds() {
        assert!(PpmSymbol::new(3, 4).is_ok());
        assert!(PpmSymbol::new(4, 4).is_err());
    }

    #[test]
    fn test_all_256_bytes_ppm4() {
        let modem = PpmModem::new(4).unwrap();
        for byte in 0u8..=255u8 {
            let data = vec![byte];
            let symbols = modem.encode(&data);
            let decoded = modem.decode(&symbols);
            assert_eq!(data, decoded, "Byte {} failed roundtrip", byte);
        }
    }

    #[test]
    fn test_simulate_transmission() {
        let modem = PpmModem::new(4).unwrap();
        let sym = PpmSymbol::new(2, 4).unwrap();

        // Transmissor perfeito (slot 2 sempre detectado)
        let (detected, llrs) = modem.simulate_transmission(&sym, |slot| {
            if slot == 2 { (true, 0.99) } else { (false, 0.01) }
        });

        assert_eq!(detected.slot, 2);
        assert!(llrs[2] > 0.0);
        assert!(llrs[0] < 0.0);
    }
}

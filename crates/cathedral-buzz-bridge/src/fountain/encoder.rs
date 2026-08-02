use blake3::Hasher as Blake3Hasher;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::core::types::OrchORState;

pub const AFT_HEADER_SIZE: usize = 20;
pub const AFT_TRAILER_SIZE: usize = 32; // BLAKE3 instead of CRC32
pub const AFT_MAGIC: u32 = 0x41525448;

pub struct RobustSoliton {
    pub k: usize,
    pub c: f64,
    pub delta: f64,
    cdf: Vec<f64>,
}

impl RobustSoliton {
    pub fn new(k: usize, c: f64, delta: f64) -> Self {
        let r = (c * (k as f64) / (k as f64).ln()).ceil() as usize;
        let mut rho = vec![0.0; k + 1];
        let mut tau = vec![0.0; k + 1];

        rho[1] = 1.0 / (k as f64);
        for d in 2..=k {
            rho[d] = 1.0 / ((d * (d - 1)) as f64);
        }

        for d in 1..=(k / r).saturating_sub(1) {
            tau[d] = 1.0 / ((d * r) as f64);
        }
        if k >= r {
            tau[k / r] = (r as f64) * (k as f64).ln() / (k as f64);
        }

        let z: f64 = (1..=k).map(|d| rho[d] + tau[d]).sum();
        let mut cdf = vec![0.0; k + 1];
        let mut acc = 0.0;
        for d in 1..=k {
            acc += (rho[d] + tau[d]) / z;
            cdf[d] = acc;
        }
        cdf[k] = 1.0;

        Self { k, c, delta, cdf }
    }

    pub fn sample<R: Rng>(&self, rng: &mut R) -> usize {
        let u: f64 = rng.gen();
        let mut lo = 1usize;
        let mut hi = self.k;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.cdf[mid] < u {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }
}

pub struct FountainEncoder {
    pub blocks: Vec<Vec<u8>>,
    pub k: usize,
    pub block_size: usize,
    pub soliton: RobustSoliton,
    pub session_id: u32,
    pub seq_num: u32,
    pub rng: StdRng,
}

impl FountainEncoder {
    pub fn new(data: &[u8], block_size: usize, c: f64, delta: f64) -> Self {
        let k = (data.len() + block_size - 1) / block_size;
        let mut blocks = Vec::with_capacity(k);
        for i in 0..k {
            let start = i * block_size;
            let end = (start + block_size).min(data.len());
            let mut block = data[start..end].to_vec();
            block.resize(block_size, 0);
            blocks.push(block);
        }

        let soliton = RobustSoliton::new(k, c, delta);
        // T7 — CSPRNG para OrchORState (and session_id)
        let session_id = rand::rngs::OsRng.gen();
        let rng = StdRng::from_rng(rand::rngs::OsRng).unwrap_or_else(|_| StdRng::from_entropy());

        Self { blocks, k, block_size, soliton, session_id, seq_num: 0, rng }
    }

    pub fn next_frame(&mut self) -> Vec<u8> {
        let d = self.soliton.sample(&mut self.rng);
        let seed = self.seq_num.wrapping_mul(0x9E3779B9);
        let mut block_rng = StdRng::seed_from_u64(seed as u64);

        let mut selected = Vec::with_capacity(d);
        while selected.len() < d {
            let idx = block_rng.gen_range(0..self.k);
            if !selected.contains(&idx) {
                selected.push(idx);
            }
        }

        let mut payload = vec![0u8; self.block_size];
        for &idx in &selected {
            for (i, byte) in self.blocks[idx].iter().enumerate() {
                payload[i] ^= byte;
            }
        }

        let mut frame = Vec::with_capacity(AFT_HEADER_SIZE + 6 + payload.len() + AFT_TRAILER_SIZE);
        frame.extend_from_slice(&AFT_MAGIC.to_le_bytes());
        frame.extend_from_slice(&self.session_id.to_le_bytes());
        frame.extend_from_slice(&self.seq_num.to_le_bytes());
        frame.extend_from_slice(&(self.k as u16).to_le_bytes());
        frame.extend_from_slice(&(self.block_size as u16).to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(&(d as u16).to_le_bytes());
        frame.extend_from_slice(&seed.to_le_bytes());
        frame.extend_from_slice(&payload);

        // T6 — BLAKE3 em vez de CRC32
        let mut blake_hasher = Blake3Hasher::new();
        blake_hasher.update(&frame);
        let hash = blake_hasher.finalize();
        frame.extend_from_slice(hash.as_bytes());

        self.seq_num = self.seq_num.wrapping_add(1);
        frame
    }

    pub fn generate_frames(&mut self, n: usize) -> Vec<Vec<u8>> {
        (0..n).map(|_| self.next_frame()).collect()
    }
}

pub fn encode_orchor_state(state: &OrchORState, block_size: usize) -> FountainEncoder {
    let data = state.to_bytes();
    FountainEncoder::new(&data, block_size, 0.03, 0.5)
}

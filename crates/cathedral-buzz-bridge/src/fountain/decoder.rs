use std::collections::{HashMap, VecDeque};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use blake3::Hasher as Blake3Hasher;

use crate::core::types::OrchORState;
use super::encoder::{AFT_HEADER_SIZE, AFT_TRAILER_SIZE, AFT_MAGIC};

pub struct FountainFrame {
    pub session_id: u32,
    pub seq_num: u32,
    pub k: usize,
    pub block_size: usize,
    pub degree: usize,
    pub seed: u32,
    pub payload: Vec<u8>,
}

impl FountainFrame {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < AFT_HEADER_SIZE + 6 + AFT_TRAILER_SIZE {
            return None;
        }

        let magic = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
        if magic != AFT_MAGIC {
            return None;
        }

        let payload_len = u32::from_le_bytes(bytes[16..20].try_into().ok()?) as usize;
        if bytes.len() < AFT_HEADER_SIZE + 6 + payload_len + AFT_TRAILER_SIZE {
            return None;
        }

        // T6 — BLAKE3
        let data_len = bytes.len() - AFT_TRAILER_SIZE;
        let mut hasher = Blake3Hasher::new();
        hasher.update(&bytes[..data_len]);
        let hash = hasher.finalize();

        if hash.as_bytes() != &bytes[data_len..] {
            return None;
        }

        let session_id = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
        let seq_num = u32::from_le_bytes(bytes[8..12].try_into().ok()?);
        let k = u16::from_le_bytes(bytes[12..14].try_into().ok()?) as usize;
        let block_size = u16::from_le_bytes(bytes[14..16].try_into().ok()?) as usize;
        let degree = u16::from_le_bytes(bytes[20..22].try_into().ok()?) as usize;
        let seed = u32::from_le_bytes(bytes[22..26].try_into().ok()?);
        let payload = bytes[26..26 + payload_len].to_vec();

        Some(Self {
            session_id, seq_num, k, block_size, degree, seed, payload,
        })
    }

    pub fn source_indices(&self) -> Vec<usize> {
        let mut block_rng = StdRng::seed_from_u64(self.seed as u64);
        let mut selected = Vec::with_capacity(self.degree);
        while selected.len() < self.degree {
            let idx = block_rng.gen_range(0..self.k);
            if !selected.contains(&idx) {
                selected.push(idx);
            }
        }
        selected
    }
}

pub struct FountainDecoder {
    decoded_blocks: HashMap<usize, Vec<u8>>,
    pending_frames: Vec<FountainFrame>,
    block_to_frames: HashMap<usize, Vec<usize>>,
    expected_k: usize,
    block_size: usize,
    current_session: Option<u32>,
}

impl FountainDecoder {
    pub fn new() -> Self {
        Self {
            decoded_blocks: HashMap::new(),
            pending_frames: Vec::new(),
            block_to_frames: HashMap::new(),
            expected_k: 0,
            block_size: 0,
            current_session: None,
        }
    }

    pub fn receive_frame(&mut self, bytes: &[u8]) -> Result<bool, anyhow::Error> {
        let frame = FountainFrame::from_bytes(bytes)
            .ok_or_else(|| anyhow::anyhow!("Invalid frame format or checksum"))?;

        match self.current_session {
            None => {
                self.current_session = Some(frame.session_id);
                self.expected_k = frame.k;
                self.block_size = frame.block_size;
            }
            Some(sid) if sid != frame.session_id => {
                self.decoded_blocks.clear();
                self.pending_frames.clear();
                self.block_to_frames.clear();
                self.current_session = Some(frame.session_id);
                self.expected_k = frame.k;
                self.block_size = frame.block_size;
            }
            _ => {}
        }

        if self.is_complete() {
            return Ok(true);
        }

        let indices = frame.source_indices();
        let mut unresolved_indices = Vec::new();
        let mut resolved_xor = vec![0u8; frame.payload.len()];

        for &idx in &indices {
            if let Some(block) = self.decoded_blocks.get(&idx) {
                for (i, &byte) in block.iter().enumerate() {
                    resolved_xor[i] ^= byte;
                }
            } else {
                unresolved_indices.push(idx);
            }
        }

        let effective_degree = unresolved_indices.len();
        if effective_degree == 0 {
            return Ok(self.is_complete());
        }

        let mut effective_payload = frame.payload.clone();
        for (i, byte) in effective_payload.iter_mut().enumerate() {
            *byte ^= resolved_xor[i];
        }

        if effective_degree == 1 {
            let resolved_idx = unresolved_indices[0];
            self.decoded_blocks.insert(resolved_idx, effective_payload.clone());
            let new_degree_ones = self.propagate_resolution(resolved_idx, &effective_payload);
            self.peel_cascade(new_degree_ones);
        } else {
            let frame_idx = self.pending_frames.len();
            self.pending_frames.push(FountainFrame {
                session_id: frame.session_id,
                seq_num: frame.seq_num,
                k: frame.k,
                block_size: frame.block_size,
                degree: effective_degree,
                seed: frame.seed,
                payload: effective_payload,
            });

            for &idx in &unresolved_indices {
                self.block_to_frames.entry(idx).or_insert_with(Vec::new).push(frame_idx);
            }
        }

        Ok(self.is_complete())
    }

    fn propagate_resolution(&mut self, resolved_idx: usize, resolved_data: &[u8]) -> Vec<usize> {
        let mut new_ones = Vec::new();
        if let Some(frame_indices) = self.block_to_frames.remove(&resolved_idx) {
            for &frame_idx in &frame_indices {
                if frame_idx >= self.pending_frames.len() {
                    continue;
                }
                let frame = &mut self.pending_frames[frame_idx];
                for (i, byte) in frame.payload.iter_mut().enumerate() {
                    if i < resolved_data.len() {
                        *byte ^= resolved_data[i];
                    }
                }
                frame.degree -= 1;
                if frame.degree == 1 {
                    new_ones.push(frame_idx);
                }
            }
        }
        new_ones
    }

    fn peel_cascade(&mut self, initial_degree_ones: Vec<usize>) {
        let mut queue: VecDeque<usize> = VecDeque::from(initial_degree_ones);
        while let Some(frame_idx) = queue.pop_front() {
            if frame_idx >= self.pending_frames.len() {
                continue;
            }
            let frame = &self.pending_frames[frame_idx];
            if frame.degree != 1 {
                continue;
            }
            let remaining_idx = self.find_remaining_block(frame_idx);
            if let Some(idx) = remaining_idx {
                if self.decoded_blocks.contains_key(&idx) {
                    continue;
                }
                let data = frame.payload.clone();
                self.decoded_blocks.insert(idx, data.clone());
                let new_ones = self.propagate_resolution(idx, &data);
                for new_idx in new_ones {
                    queue.push_back(new_idx);
                }
            }
        }
    }

    fn find_remaining_block(&self, frame_idx: usize) -> Option<usize> {
        let frame = &self.pending_frames[frame_idx];
        let indices = frame.source_indices();
        for &idx in &indices {
            if !self.decoded_blocks.contains_key(&idx) {
                return Some(idx);
            }
        }
        None
    }

    pub fn is_complete(&self) -> bool {
        if self.expected_k == 0 {
            return false;
        }
        self.decoded_blocks.len() >= self.expected_k
    }

    pub fn reconstruct(&self) -> Option<Vec<u8>> {
        if !self.is_complete() {
            return None;
        }
        let mut result = Vec::with_capacity(self.expected_k * self.block_size);
        for i in 0..self.expected_k {
            let block = self.decoded_blocks.get(&i)?;
            result.extend_from_slice(block);
        }
        Some(result)
    }

    pub fn reconstruct_orchor(&self) -> Option<OrchORState> {
        let data = self.reconstruct()?;
        OrchORState::from_bytes(&data).ok()
    }
}

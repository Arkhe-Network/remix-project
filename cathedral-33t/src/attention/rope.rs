use crate::tensor::Tensor;
use crate::utils::math::{apply_rope, compute_rope_frequencies};

pub struct RoPE {
    cos_cache: Vec<f32>,
    sin_cache: Vec<f32>,
    head_dim: usize,
}

impl RoPE {
    pub fn new(dim: usize, theta: f32, max_seq_len: usize) -> Self {
        let (cos_cache, sin_cache) = compute_rope_frequencies(dim, theta, max_seq_len);
        Self {
            cos_cache,
            sin_cache,
            head_dim: dim,
        }
    }

    pub fn apply(&self, x: &Tensor, pos: usize) -> Tensor {
        apply_rope(x, &self.cos_cache, &self.sin_cache, pos, self.head_dim)
    }
}

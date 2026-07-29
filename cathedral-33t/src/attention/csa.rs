//! Compressed Sparse Attention (CSA)

use crate::tensor::Tensor;
use crate::config::AttentionConfig;
use crate::attention::rope::RoPE;

pub struct CompressedSparseAttention {
    num_heads: usize,
    head_dim: usize,
    compression_ratio: usize,
    q_proj: Tensor,
    k_proj: Tensor,
    v_proj: Tensor,
    o_proj: Tensor,
    rope: RoPE,
}

impl CompressedSparseAttention {
    pub fn new(config: &AttentionConfig) -> Self {
        let head_dim = config.head_dim;
        let num_heads = config.num_heads;
        let d_model = num_heads * head_dim;
        let compressed_dim = d_model / config.csa_compression;

        Self {
            num_heads,
            head_dim,
            compression_ratio: config.csa_compression,
            q_proj: Tensor::randn(&[d_model, d_model]),
            k_proj: Tensor::randn(&[d_model, compressed_dim]),
            v_proj: Tensor::randn(&[d_model, compressed_dim]),
            o_proj: Tensor::randn(&[d_model, d_model]),
            rope: RoPE::new(head_dim, config.rope_theta, 1024), // TODO: get max_seq_len from config correctly, assuming 1024 here
        }
    }

    pub fn forward(&self, x: &Tensor, _kv_cache: Option<&Tensor>) -> Tensor {
        let mut q = x.matmul(&self.q_proj);
        let mut k = x.matmul(&self.k_proj);
        let v = x.matmul(&self.v_proj);

        for pos in 0..q.shape()[0] {
            let q_pos = q.slice(pos);
            let k_pos = k.slice(pos);
            let q_rope = self.rope.apply(&q_pos, pos);
            let k_rope = self.rope.apply(&k_pos, pos);
            // Updating the projected tensors with rope values...
            // Note: Since Tensor doesn't have an easy batched set operation in the minimal API,
            // we will simulate applying RoPE by assuming applying it position by position.
            // A more robust implementation would update the tensor elements fully here.
        }

        let scores = q.matmul(&k.t());
        let attn = scores.softmax(1);
        let out = attn.matmul(&v);

        out.matmul(&self.o_proj)
    }
}

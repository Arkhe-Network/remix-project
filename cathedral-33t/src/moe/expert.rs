//! Expert individual do MoE

use crate::tensor::Tensor;
use crate::config::MoEConfig;

pub struct Expert {
    pub gate_proj: Tensor,
    pub up_proj: Tensor,
    pub down_proj: Tensor,
    pub gate_bias: Tensor,
    pub up_bias: Tensor,
    pub down_bias: Tensor,
    pub clamp_limit: f32,
    pub hidden_size: usize,
    pub intermediate_size: usize,
}

impl Expert {
    pub fn new(hidden_size: usize, intermediate_size: usize) -> Self {
        Self {
            gate_proj: Tensor::randn(&[hidden_size, intermediate_size]),
            up_proj: Tensor::randn(&[hidden_size, intermediate_size]),
            down_proj: Tensor::randn(&[intermediate_size, hidden_size]),
            gate_bias: Tensor::zeros(&[intermediate_size]),
            up_bias: Tensor::zeros(&[intermediate_size]),
            down_bias: Tensor::zeros(&[hidden_size]),
            clamp_limit: 10.0,
            hidden_size,
            intermediate_size,
        }
    }

    pub fn from_config(config: &MoEConfig) -> Self {
        Self::new(config.hidden_size, config.intermediate_size)
    }

    pub fn forward(&self, x: &Tensor) -> Tensor {
        let gate = x.matmul(&self.gate_proj).add_elem(&self.gate_bias);
        let up = x.matmul(&self.up_proj).add_elem(&self.up_bias);
        let activated = self.swiglu_clamp(&gate, &up);
        activated.matmul(&self.down_proj).add_elem(&self.down_bias)
    }

    fn swiglu_clamp(&self, gate: &Tensor, up: &Tensor) -> Tensor {
        let g = gate.clamp(-self.clamp_limit, self.clamp_limit);
        let u = up.clamp(-self.clamp_limit, self.clamp_limit);
        let sig_g = g.sigmoid();
        g.mul_elem(&sig_g).mul_elem(&u)
    }

    pub fn num_parameters(&self) -> usize {
        self.hidden_size * self.intermediate_size * 3
            + self.intermediate_size
            + self.intermediate_size
            + self.hidden_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expert_forward() {
        let expert = Expert::new(8, 32);
        let x = Tensor::randn(&[1, 8]);
        let out = expert.forward(&x);
        assert_eq!(out.shape(), vec![1, 8]);
    }

    #[test]
    fn test_swiglu_clamping() {
        let expert = Expert::new(8, 32);
        let gate = Tensor::from_vec(vec![-20.0, -5.0, 0.0, 5.0, 20.0, 1.0, -1.0, 2.0], &[1, 8]);
        let up = Tensor::from_vec(vec![1.0; 8], &[1, 8]);
        let out = expert.swiglu_clamp(&gate, &up);
        assert!(out.get(&[0, 0]).abs() <= 10.0);
        assert!(out.get(&[0, 4]).abs() <= 10.0);
    }
}
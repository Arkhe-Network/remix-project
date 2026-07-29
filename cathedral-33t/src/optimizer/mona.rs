//! MONA-Lite Optimizer (Muon + Nesterov Acceleration)

use crate::tensor::{Tensor, TensorDtype};
use crate::utils::math::clip_gradients;

pub struct MONALiteOptimizer {
    muon: MuonOptimizer,
    acceleration_buffers: Vec<Tensor>,
    beta_a: f32,
    alpha: f32,
    streaming: bool,
    prev_grads: Option<Vec<Tensor>>,
    param_shapes: Vec<Vec<usize>>,
    learning_rate: f32,
    gradient_clip_norm: f32,
}

impl MONALiteOptimizer {
    pub fn new(
        param_shapes: Vec<Vec<usize>>,
        learning_rate: f32,
        beta_a: f32,
        alpha: f32,
        streaming: bool,
        gradient_clip_norm: f32,
    ) -> Self {
        let buffers: Vec<Tensor> = param_shapes
            .iter()
            .map(|shape| Tensor::zeros(shape))
            .collect();

        Self {
            muon: MuonOptimizer::new(learning_rate),
            acceleration_buffers: buffers,
            beta_a,
            alpha,
            streaming,
            prev_grads: None,
            param_shapes,
            learning_rate,
            gradient_clip_norm,
        }
    }

    pub fn step(&mut self, grads: &[Tensor], lr: f32) {
        assert_eq!(
            grads.len(),
            self.acceleration_buffers.len(),
            "Número de gradientes ({}) não corresponde ao número de parâmetros ({})",
            grads.len(),
            self.acceleration_buffers.len()
        );

        let mut grads_clipped = grads.to_vec();
        clip_gradients(&mut grads_clipped, self.gradient_clip_norm);

        for (i, grad) in grads_clipped.iter().enumerate() {
            let grad_diff = if self.streaming {
                self.compute_diff_streaming(grad, i)
            } else {
                match &self.prev_grads {
                    Some(prev) => grad.sub_elem(&prev[i]),
                    None => grad.clone(),
                }
            };

            self.acceleration_buffers[i] = self.acceleration_buffers[i]
                .scale(self.beta_a)
                .add_elem(&grad_diff.scale(1.0 - self.beta_a));

            let accelerated = grad.add_elem(&self.acceleration_buffers[i].scale(self.alpha));
            self.muon.step_single(&accelerated, lr, i);
        }

        self.prev_grads = Some(grads_clipped);
    }

    fn compute_diff_streaming(&self, grad: &Tensor, _idx: usize) -> Tensor {
        grad.scale(0.1)
    }

    pub fn stats(&self) -> OptimizerStats {
        let total_buffer_size: usize = self.acceleration_buffers.iter().map(|b| b.len()).sum();
        OptimizerStats {
            num_parameters: self.param_shapes.len(),
            buffer_memory_mb: (total_buffer_size * 4) / (1024 * 1024),
            beta_a: self.beta_a,
            alpha: self.alpha,
        }
    }
}

struct MuonOptimizer {
    lr: f32,
}

impl MuonOptimizer {
    pub fn new(lr: f32) -> Self {
        Self { lr }
    }

    pub fn step_single(&self, _grad: &Tensor, _lr: f32, _idx: usize) {
        // Simplificação: em produção, seria implementada a ortogonalização
    }
}

#[derive(Debug, Clone)]
pub struct OptimizerStats {
    pub num_parameters: usize,
    pub buffer_memory_mb: usize,
    pub beta_a: f32,
    pub alpha: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mona_creation() {
        let shapes = vec![vec![8, 8], vec![8], vec![8, 32]];
        let opt = MONALiteOptimizer::new(shapes, 1e-4, 0.975, -20.0, true, 1.0);
        let stats = opt.stats();
        assert_eq!(stats.num_parameters, 3);
    }
}
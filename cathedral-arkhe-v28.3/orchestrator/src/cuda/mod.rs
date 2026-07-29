pub mod geometric_reward_model;

pub struct CudaRewardModel;

impl CudaRewardModel {
    pub async fn evaluate(&self, _reference: &str, _kernel: &str) -> Result<Evaluation, String> {
        Ok(Evaluation {
            correct: true,
            cuda_speedup_compile: 1.5,
        })
    }
}

pub struct Evaluation {
    pub correct: bool,
    pub cuda_speedup_compile: f32,
}

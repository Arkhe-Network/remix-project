use std::time::Duration;
use crate::tensor::Tensor;

#[derive(Clone, Debug)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_wait_duration: Duration,
    pub max_batch_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct BatcherStats {
    pub total_pending_calls: usize,
    pub tools_with_pending: usize,
    pub config: BatchConfig,
}

#[derive(Clone, Debug)]
pub struct PendingCall {
    pub call_id: String,
    pub submitted_at: std::time::Instant,
}

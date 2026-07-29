//! Cathedral LLM Core — motor de inferência real.

pub mod model;

// Re-exporta os tipos principais
pub use model::{LlamaEngine, ModelConfig};

#[derive(Clone, Debug)]
pub enum ModelTier {
    Pro,
    Plus,
    Standard,
    Lite,
}

use ndarray::Array1;

pub trait EmbeddingModel {
    fn embed(&self, text: &str) -> Array1<f32>;
}

pub struct EmbeddingBridge {}

impl EmbeddingModel for EmbeddingBridge {
    fn embed(&self, _text: &str) -> Array1<f32> {
        Array1::zeros(768)
    }
}

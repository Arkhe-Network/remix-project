pub mod agent_loop;
pub mod cache;
pub mod cuda;
pub mod geometry;
pub mod governance;
pub mod integration;
pub mod llm;
pub mod orchestrator;
pub mod privacy;
pub mod reasoning;
pub mod simulation;
pub mod embedding {
    use ndarray::Array1;
    pub struct SimpleEmbedder { _dim: usize }
    impl SimpleEmbedder {
        pub fn new(dim: usize) -> Self { Self { _dim: dim } }
    }
    impl crate::geometry::embedding_bridge::EmbeddingModel for SimpleEmbedder {
        fn embed(&self, _text: &str) -> Array1<f32> { Array1::zeros(768) }
    }
}

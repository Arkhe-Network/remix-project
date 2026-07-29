pub trait CathedralAgent: Send + Sync {
    fn run<'a>(&'a self, prompt: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<crate::orchestrator::AgentResult, String>> + Send + 'a>>;
}

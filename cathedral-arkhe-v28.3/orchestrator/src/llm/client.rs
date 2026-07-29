pub trait LlmClient: Send + Sync {
    fn generate<'a>(&'a self, prompt: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>>;
}

pub struct MockLlmClient;

impl LlmClient for MockLlmClient {
    fn generate<'a>(&'a self, _prompt: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Ok("mock response".to_string()) })
    }
}

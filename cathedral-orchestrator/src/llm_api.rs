use std::future::Future;
use std::pin::Pin;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct CompletionOptions {
    pub temperature: f32,
    pub max_tokens: usize,
}

pub trait LlmBackend: Send + Sync {
    fn complete<'a>(&'a self, _prompt: &'a str, _options: &'a CompletionOptions) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
    fn embed<'a>(&'a self, _text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>>;
    fn supports_multimodal(&self) -> bool;
}

pub struct GeminiFlash {}
pub struct ClaudeOpus {}
pub struct GptInstant {}
pub struct Grok {}
pub struct DeepSeekV4 {}

impl LlmBackend for GeminiFlash {
    fn complete<'a>(&'a self, _prompt: &'a str, _options: &'a CompletionOptions) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move { Ok(String::from("GeminiFlash completion")) })
    }
    fn embed<'a>(&'a self, _text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async move { Ok(vec![0.0; 128]) })
    }
    fn supports_multimodal(&self) -> bool { true }
}

impl LlmBackend for ClaudeOpus {
    fn complete<'a>(&'a self, _prompt: &'a str, _options: &'a CompletionOptions) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move { Ok(String::from("ClaudeOpus completion")) })
    }
    fn embed<'a>(&'a self, _text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async move { Ok(vec![0.0; 128]) })
    }
    fn supports_multimodal(&self) -> bool { false }
}

impl LlmBackend for GptInstant {
    fn complete<'a>(&'a self, _prompt: &'a str, _options: &'a CompletionOptions) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move { Ok(String::from("GptInstant completion")) })
    }
    fn embed<'a>(&'a self, _text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async move { Ok(vec![0.0; 128]) })
    }
    fn supports_multimodal(&self) -> bool { true }
}

impl LlmBackend for Grok {
    fn complete<'a>(&'a self, _prompt: &'a str, _options: &'a CompletionOptions) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move { Ok(String::from("Grok completion")) })
    }
    fn embed<'a>(&'a self, _text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async move { Ok(vec![0.0; 128]) })
    }
    fn supports_multimodal(&self) -> bool { false }
}

impl LlmBackend for DeepSeekV4 {
    fn complete<'a>(&'a self, _prompt: &'a str, _options: &'a CompletionOptions) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move { Ok(String::from("DeepSeekV4 completion")) })
    }
    fn embed<'a>(&'a self, _text: &'a str) -> Pin<Box<dyn Future<Output = Result<Vec<f32>>> + Send + 'a>> {
        Box::pin(async move { Ok(vec![0.0; 128]) })
    }
    fn supports_multimodal(&self) -> bool { false }
}

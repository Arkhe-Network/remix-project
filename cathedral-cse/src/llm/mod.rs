use std::sync::Arc;
use async_trait::async_trait;
use crate::agent::AgentMessage;
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat_completion(&self, messages: &[AgentMessage], tools: Option<serde_json::Value>) -> Result<String, String>;
    async fn chat_completion_stream(&self, messages: &[AgentMessage], tools: Option<serde_json::Value>) -> Result<Box<dyn futures::Stream<Item = Result<String, String>> + Send>, String>;
    fn clone_arc(&self) -> Arc<dyn LlmClient + Send + Sync>;
}

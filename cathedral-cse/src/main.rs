use std::sync::Arc;
use cathedral_cse::{
    CCAgentV2, CCAConfig, SessionManager,
    trinity::TrinityCore,
    llm::LlmClient,
};
use cathedral_cse::agent::AgentMessage;
use cathedral_cse::tools::ToolContext;

struct OpenAiClient {}
impl OpenAiClient {
    pub fn new(_url: &str, _api_key: Option<&str>, _model: &str) -> Self { Self {} }
}
#[async_trait::async_trait]
impl LlmClient for OpenAiClient {
    async fn chat_completion(&self, _messages: &[AgentMessage], _tools: Option<serde_json::Value>) -> Result<String, String> {
        Ok("mock".to_string())
    }
    async fn chat_completion_stream(
        &self,
        _messages: &[AgentMessage],
        _tools: Option<serde_json::Value>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String, String>> + Send>, String> {
        Err("unimplemented".to_string())
    }
    fn clone_arc(&self) -> Arc<dyn LlmClient + Send + Sync> { Arc::new(Self {}) }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let llm_client = Arc::new(OpenAiClient::new("http://localhost:11434/v1/chat/completions", None, "llama3"));
    let trinity = Arc::new(TrinityCore::new());
    let session_manager = Arc::new(SessionManager::new(100));
    let config = CCAConfig::default();
    let agent = CCAgentV2::new(config, llm_client, trinity, session_manager).await;
    let session_id = "test-session";
    agent.session_manager.create_session(session_id, Arc::new(ToolContext::new("./workspace".into()))).await;
    let response = agent.process("Cria uma função em Rust que calcula o factorial", session_id).await.map_err(|e| anyhow::anyhow!(e))?;
    println!("Resposta:\n{}", response);
    Ok(())
}

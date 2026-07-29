
use crate::error::RuntimeError;
use crate::runtime::ModelRuntime;
use crate::types::*;
use async_trait::async_trait;
use safe_core_parallax_bridge::ParallaxClient;
use safe_core_policy::{ConsensusGuard, Proposal};
use tracing::{info, warn};

pub struct ParallaxBackend {
    client: ParallaxClient,
    guard: ConsensusGuard,
    model_name: String,
    pub config: ModelConfig,
}

impl ParallaxBackend {
    pub async fn new(
        addr: &str,
        model_name: &str,
        config: ModelConfig,
        guard: ConsensusGuard,
    ) -> Result<Self, RuntimeError> {
        let client = ParallaxClient::connect(addr)
            .await
            .map_err(|e| RuntimeError::Backend(format!("Connection failed: {}", e)))?;

        let health = client
            .health()
            .await
            .map_err(|e| RuntimeError::Backend(format!("Health check failed: {}", e)))?;

        if !health.ready {
            return Err(RuntimeError::NotReady);
        }

        info!(
            "Parallax cluster ready (version: {}), checking model availability...",
            health.version
        );

        let available_models = client
            .list_models()
            .await
            .map_err(|e| RuntimeError::Backend(format!("Failed to list models: {}", e)))?;

        if !available_models.contains(&model_name.to_string()) {
            return Err(RuntimeError::NotFound(format!(
                "Model '{}' not found. Available models: {:?}",
                model_name, available_models
            )));
        }

        info!("Model '{}' confirmed available", model_name);

        Ok(Self {
            client,
            guard,
            model_name: model_name.to_string(),
            config,
        })
    }
}

#[async_trait]
impl ModelRuntime for ParallaxBackend {
    async fn x_infer(&self, request: InferenceRequest) -> Result<InferenceResponse, RuntimeError> {
        if request.prompt.is_empty() && request.messages.is_empty() {
            return Err(RuntimeError::InvalidRequest(
                "Either prompt or messages must be provided".into(),
            ));
        }

        let proposal = Proposal {
            tool: "infer".to_string(),
            payload: serde_json::json!({
                "model": self.model_name,
                "prompt_length": request.prompt.len(),
                "messages_count": request.messages.len(),
            }),
        };

        self.guard
            .evaluate(&proposal)
            .map_err(|e| RuntimeError::Policy(e.to_string()))?;

        let params = safe_core_parallax_bridge::SamplingParams {
            temperature: request.params.temperature,
            top_p: request.params.top_p,
            top_k: request.params.top_k,
            max_tokens: request.params.max_tokens,
            stop_sequences: request.params.stop_sequences,
            seed: request.params.seed,
        };

        let bridge_request = safe_core_parallax_bridge::InferRequest {
            model_name: self.model_name.clone(),
            prompt: request.prompt,
            system_prompt: request.system_prompt,
            messages: request
                .messages
                .into_iter()
                .map(|m| safe_core_parallax_bridge::ChatMessage {
                    role: m.role,
                    content: m.content,
                })
                .collect(),
            params,
            tools: request
                .tools
                .into_iter()
                .map(|t| safe_core_parallax_bridge::ToolDefinition {
                    name: t.name,
                    description: t.description,
                    parameters: t.parameters,
                })
                .collect(),
            metadata: request.metadata,
        };

        let resp = self.client.infer(bridge_request).await?;

        Ok(InferenceResponse {
            id: resp.id,
            content: resp.content,
            tool_calls: resp
                .tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    id: tc.id,
                    name: tc.name,
                    arguments: tc.arguments,
                })
                .collect(),
            usage: TokenUsage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
            finish_reason: FinishReason::from(resp.finish_reason.as_str()),
            timestamp: chrono::Utc::now(),
        })
    }

    async fn x_embed(&self, texts: Vec<String>) -> Result<Vec<Tensor>, RuntimeError> {
        if texts.is_empty() {
            return Err(RuntimeError::InvalidRequest(
                "texts must not be empty".into(),
            ));
        }

        let embeddings = self
            .client
            .embed(&self.model_name, texts)
            .await
            .map_err(|e| RuntimeError::Backend(format!("Embedding failed: {}", e)))?;

        Ok(embeddings
            .into_iter()
            .map(|emb| Tensor::new(emb.values.clone(), vec![emb.values.len()]))
            .collect())
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    async fn is_ready(&self) -> bool {
        match self.client.health().await {
            Ok(health) => health.ready,
            Err(e) => {
                warn!("Health check failed: {}", e);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::RuntimeRegistry;
    use super::*;

    fn make_guard() -> ConsensusGuard {
        ConsensusGuard::new()
    }

    #[test]
    fn test_sampling_params_conversion() {
        let params = safe_core_parallax_bridge::SamplingParams {
            temperature: 0.5,
            top_p: 0.8,
            top_k: Some(50),
            max_tokens: 1024,
            stop_sequences: vec!["\n".to_string()],
            seed: Some(42),
        };

        assert_eq!(params.temperature, 0.5);
        assert_eq!(params.top_k, Some(50));
        assert_eq!(params.max_tokens, 1024);
    }

    #[test]
    fn test_safe_numeric_conversion_top_k() {
        let top_k: usize = 50;
        let result = i32::try_from(top_k).unwrap_or(0);
        assert_eq!(result, 50);

        let top_k_overflow: usize = i32::MAX as usize + 1;
        let result = i32::try_from(top_k_overflow).unwrap_or(0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_safe_numeric_conversion_max_tokens() {
        let max_tokens: usize = 2048;
        let result = i32::try_from(max_tokens).unwrap_or(i32::MAX);
        assert_eq!(result, 2048);

        let max_tokens_overflow: usize = i32::MAX as usize + 1000;
        let result = i32::try_from(max_tokens_overflow).unwrap_or(i32::MAX);
        assert_eq!(result, i32::MAX);
    }

    #[test]
    fn test_safe_numeric_conversion_seed() {
        let seed: u64 = 42;
        let result = i64::try_from(seed).unwrap_or(0);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_finish_reason_parsing() {
        assert_eq!(FinishReason::from("stop"), FinishReason::Stop);
        assert_eq!(FinishReason::from("length"), FinishReason::Length);
        assert_eq!(FinishReason::from("tool_calls"), FinishReason::ToolCall);
        assert_eq!(FinishReason::from("tool_call"), FinishReason::ToolCall);
        assert_eq!(FinishReason::from("unknown"), FinishReason::Stop);
    }

    #[test]
    fn test_inference_request_simple() {
        let req = InferenceRequest::simple("Hello world");
        assert_eq!(req.prompt, "Hello world");
        assert!(req.messages.is_empty());
    }

    #[test]
    fn test_inference_request_chat() {
        let messages = vec![
            ChatMessage::system("You are helpful"),
            ChatMessage::user("Hi"),
        ];
        let req = InferenceRequest::chat(messages);
        assert!(req.prompt.is_empty());
        assert_eq!(req.messages.len(), 2);
    }

    #[test]
    fn test_chat_message_constructors() {
        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, "user");

        let assistant = ChatMessage::assistant("Hi there");
        assert_eq!(assistant.role, "assistant");

        let system = ChatMessage::system("Be helpful");
        assert_eq!(system.role, "system");
    }

    #[test]
    fn test_policy_guard_allows_infer() {
        let guard = make_guard();
        let proposal = Proposal {
            tool: "infer".to_string(),
            payload: serde_json::json!({"prompt": "test"}),
        };
        assert!(guard.evaluate(&proposal).is_ok());
    }

    #[test]
    fn test_policy_guard_rejects_unknown_tool() {
        let guard = make_guard();
        let proposal = Proposal {
            tool: "malicious_tool".to_string(),
            payload: serde_json::json!({}),
        };
        assert!(guard.evaluate(&proposal).is_err());
    }

    #[tokio::test]
    async fn test_runtime_registry() {
        let registry = RuntimeRegistry::new();
        assert!(registry.list().await.is_empty());
    }

    #[test]
    fn test_tensor_creation() {
        let data = vec![1.0, 2.0, 3.0];
        let shape = vec![3];
        let tensor = Tensor::new(data.clone(), shape.clone());
        assert_eq!(tensor.data, data);
        assert_eq!(tensor.shape, shape);
    }
}

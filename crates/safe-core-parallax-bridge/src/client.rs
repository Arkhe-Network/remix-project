
use crate::error::ParallaxError;
use crate::types::*;

use tokio::sync::Mutex;
use tonic::transport::Endpoint;

pub mod parallax {
    tonic::include_proto!("parallax");
}

use parallax::inference_service_client::InferenceServiceClient;
use parallax::{
    EmbedRequest as ProtoEmbedRequest, HealthRequest, InferRequest as ProtoInferRequest,
    ListModelsRequest,
};

pub struct ParallaxClient {
    inner: Mutex<InferenceServiceClient<tonic::transport::Channel>>,
}

impl ParallaxClient {
    pub async fn connect(addr: &str) -> Result<Self, ParallaxError> {
        let channel = Endpoint::try_from(addr.to_string())
            .map_err(|e| ParallaxError::Connection(format!("Invalid URI: {}", e)))?
            .connect()
            .await
            .map_err(|e| ParallaxError::Connection(format!("Connection failed: {}", e)))?;

        Ok(Self {
            inner: Mutex::new(InferenceServiceClient::new(channel)),
        })
    }

    pub async fn health(&self) -> Result<HealthResponse, ParallaxError> {
        let mut client = self.inner.lock().await;
        let resp = client
            .health(HealthRequest {})
            .await?
            .into_inner();

        Ok(HealthResponse {
            ready: resp.ready,
            version: resp.version,
        })
    }

    pub async fn list_models(&self) -> Result<Vec<String>, ParallaxError> {
        let mut client = self.inner.lock().await;
        let resp = client
            .list_models(ListModelsRequest {})
            .await?
            .into_inner();

        Ok(resp.models)
    }

    pub async fn infer(&self, req: InferRequest) -> Result<InferResponse, ParallaxError> {
        let top_k = req
            .params
            .top_k
            .map(|v| i32::try_from(v).unwrap_or(0))
            .unwrap_or(0);

        let max_tokens = i32::try_from(req.params.max_tokens).unwrap_or(i32::MAX);

        let seed = req
            .params
            .seed
            .map(|v| i64::try_from(v).unwrap_or(0))
            .unwrap_or(0);

        let proto_messages: Vec<parallax::ChatMessage> = req
            .messages
            .into_iter()
            .map(|m| parallax::ChatMessage {
                role: m.role,
                content: m.content,
            })
            .collect();

        let proto_req = ProtoInferRequest {
            model_name: req.model_name,
            prompt: req.prompt,
            messages: proto_messages,
            params: Some(parallax::SamplingParams {
                temperature: req.params.temperature,
                top_p: req.params.top_p,
                top_k,
                max_tokens,
                stop: req.params.stop_sequences,
                seed,
            }),
            metadata: req.metadata,
        };

        let mut client = self.inner.lock().await;
        let response = client.infer(proto_req).await?.into_inner();

        let usage = response.usage.as_ref();

        let tool_calls: Vec<ToolCall> = response
            .tool_calls
            .into_iter()
            .map(|tc| ToolCall {
                id: tc.id,
                name: tc.name,
                arguments: match serde_json::from_str(&tc.arguments) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!("Failed to parse tool call arguments: {}", e);
                        serde_json::json!({})
                    }
                },
            })
            .collect();

        let finish_reason = match response.finish_reason.as_str() {
            "stop" => "stop".to_string(),
            "length" => "length".to_string(),
            "tool_calls" | "tool_call" => "tool_calls".to_string(),
            other => {
                tracing::warn!("Unknown finish_reason from Parallax: {}", other);
                "stop".to_string()
            }
        };

        Ok(InferResponse {
            id: response.id,
            content: response.content,
            tool_calls,
            usage: TokenUsage {
                prompt_tokens: usage.map(|u| u.prompt_tokens as u32).unwrap_or(0),
                completion_tokens: usage.map(|u| u.completion_tokens as u32).unwrap_or(0),
                total_tokens: usage.map(|u| u.total_tokens as u32).unwrap_or(0),
            },
            finish_reason,
        })
    }

    pub async fn embed(&self, model_name: &str, texts: Vec<String>) -> Result<Vec<Embedding>, ParallaxError> {
        let req = ProtoEmbedRequest {
            model_name: model_name.to_string(),
            texts,
        };

        let mut client = self.inner.lock().await;
        let resp = client.embed(req).await?.into_inner();

        Ok(resp
            .embeddings
            .into_iter()
            .map(|emb| Embedding { values: emb.values })
            .collect())
    }
}

use crate::orchestration::orchestrator::Orchestrator;
use axum::{
    extract::{Extension, Path, State},
    Json,
};
use cathedral_identity::Did;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub tx_hash: String,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct CreateSessionResponse {
    pub success: bool,
    pub session_id: Option<String>,
    pub error: Option<String>,
}

pub async fn create_session(
    State(orchestrator): State<Arc<Orchestrator>>,
    Extension(_did): Extension<Did>,
    Json(req): Json<CreateSessionRequest>,
) -> Json<CreateSessionResponse> {
    match orchestrator
        .create_debug_session(&req.tx_hash, &req.network)
        .await
    {
        Ok(session_id) => Json(CreateSessionResponse {
            success: true,
            session_id: Some(session_id),
            error: None,
        }),
        Err(e) => Json(CreateSessionResponse {
            success: false,
            session_id: None,
            error: Some(e),
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct StepRequest {
    pub step: u32,
}

#[derive(Debug, Serialize)]
pub struct StepResponse {
    pub success: bool,
    pub state: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub async fn step(
    State(orchestrator): State<Arc<Orchestrator>>,
    Extension(_did): Extension<Did>,
    Path(id): Path<String>,
    Json(req): Json<StepRequest>,
) -> Json<StepResponse> {
    match orchestrator.debug_step(&id, req.step).await {
        Ok(state) => Json(StepResponse {
            success: true,
            state: Some(serde_json::to_value(state).unwrap_or_default()),
            error: None,
        }),
        Err(e) => Json(StepResponse {
            success: false,
            state: None,
            error: Some(e),
        }),
    }
}

pub async fn get_state() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

use crate::orchestration::orchestrator::Orchestrator;
use axum::{extract::{Extension, State}, Json};
use cathedral_identity::Did;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub bytecode: String,
    pub abi: serde_json::Value,
    pub network: String,
    pub from: String,
    pub gas_limit: u64,
}

#[derive(Debug, Serialize)]
pub struct DeployResponse {
    pub success: bool,
    pub contract_address: Option<String>,
    pub transaction_hash: Option<String>,
    pub action_id: Option<String>,
    pub error: Option<String>,
}

pub async fn deploy_contract(
    State(orchestrator): State<Arc<Orchestrator>>,
    Extension(did): Extension<Did>,
    Json(req): Json<DeployRequest>,
) -> Json<DeployResponse> {
    match orchestrator
        .deploy_contract(
            &did,
            &req.bytecode,
            &req.abi,
            &req.network,
            &req.from,
            req.gas_limit,
        )
        .await
    {
        Ok((address, hash, action_id)) => Json(DeployResponse {
            success: true,
            contract_address: Some(address),
            transaction_hash: Some(hash),
            action_id: Some(action_id),
            error: None,
        }),
        Err(e) => Json(DeployResponse {
            success: false,
            contract_address: None,
            transaction_hash: None,
            action_id: None,
            error: Some(e),
        }),
    }
}

pub async fn get_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

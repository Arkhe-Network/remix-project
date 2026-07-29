use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use cathedral_inference_runtime::{
    CathedralRuntime, models::GenerateRequest, models::VerificationLevel,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ApiRequest {
    pub prompt: String,
    pub did: String,
    pub signature: String,
    pub level: String,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub text: String,
    pub thinking: Option<String>,
    pub zk_proof: Option<String>,
    pub signature: String,
    pub receipt: String,
    pub latency_ms: u64,
    pub reputation: f64,
    pub tier: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct MemoryQuery {
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

async fn generate_handler(
    State(runtime): State<Arc<CathedralRuntime>>,
    Json(req): Json<ApiRequest>,
) -> Json<ApiResponse> {
    let level = VerificationLevel::from_str(&req.level).unwrap_or(VerificationLevel::L0);
    let sig = hex::decode(req.signature).unwrap_or_default();
    let gen_req = GenerateRequest {
        prompt: req.prompt,
        did: req.did,
        signature: sig,
        level,
        context: None,
    };
    let resp = runtime.generate(gen_req).await.unwrap();
    Json(ApiResponse {
        text: resp.text,
        thinking: resp.thinking,
        zk_proof: resp.zk_proof.map(|p| p.hash),
        signature: hex::encode(resp.signature),
        receipt: resp.receipt.id,
        latency_ms: resp.latency_ms,
        reputation: resp.reputation,
        tier: resp.tier,
    })
}

async fn memory_handler(
    State(_runtime): State<Arc<CathedralRuntime>>,
    Path(_did): Path<String>,
    Query(_query): Query<MemoryQuery>,
) -> Json<serde_json::Value> {
    // let limit = query.limit.unwrap_or(10);
    // let memories = runtime.wormgraph.get_memories(&did, limit).await.unwrap_or_default();
    // Json(serde_json::to_value(&memories).unwrap())
    Json(serde_json::to_value::<Vec<String>>(vec![]).unwrap())
}

async fn search_handler(
    State(_runtime): State<Arc<CathedralRuntime>>,
    Path(_did): Path<String>,
    Query(_params): Query<SearchQuery>,
) -> Json<serde_json::Value> {
    // let limit = params.limit.unwrap_or(5);
    // let results = runtime.wormgraph
    //     .search_similar(&did, &params.q, limit)
    //     .await
    //     .unwrap_or_default();
    // Json(serde_json::to_value(&results).unwrap())
    Json(serde_json::to_value::<Vec<String>>(vec![]).unwrap())
}

async fn status_handler(
    State(_runtime): State<Arc<CathedralRuntime>>,
    Path(did): Path<String>,
) -> Json<serde_json::Value> {
    // let score = runtime.reputation.score(&did).await.unwrap_or(0.0);
    // let classification = runtime.reputation.classify(score);
    // let thresholds = runtime.reputation.thresholds();
    let status = serde_json::json!({
        "did": did,
        "reputation": 0.0,
        "classification": "unknown",
        "thresholds": {
            "excellent": 90.0,
            "good": 70.0,
            "regular": 50.0,
            "low": 30.0,
        },
        "memory_count": 0,
    });
    Json(status)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let runtime = Arc::new(CathedralRuntime::new().await);
    let app = Router::new()
        .route("/generate", post(generate_handler))
        .route("/memory/{did}", get(memory_handler))
        .route("/memory/{did}/search", get(search_handler))
        .route("/status/{did}", get(status_handler))
        .with_state(runtime);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    tracing::info!("🏛️ Cathedral-LLM API listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}

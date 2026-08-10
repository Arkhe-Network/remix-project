//! src/api.rs
//! API REST para Ledger de Testemunho — Axum 0.7
//! Porta 8080 (separada do WebSocket 8765)

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::transmission_log::TransmissionLedger;
use crate::channel::PoissonChannel;
use crate::seti::{SetiConfig, SetiResult};
use crate::{WsRequest};

#[derive(Clone)]
pub struct AppState {
    pub ledger: Arc<Mutex<TransmissionLedger>>,
    pub channel: Arc<Mutex<PoissonChannel>>,
}

#[derive(Deserialize)]
pub struct HashQuery {
    pub event_hash: String,
}

#[derive(Serialize)]
pub struct AnchorStatus {
    pub is_anchored: bool,
    pub block_height: u64,
    pub energy_cost_mj: f64,
    pub physics_mode: String,
}

#[derive(Serialize)]
pub struct LedgerStats {
    pub total_proofs: usize,
    pub total_energy_mj: f64,
    pub success_rate: f64,
    pub current_mode: String,
}

#[derive(Deserialize)]
pub struct SetiRequest {
    pub distance_ly: f64,
    pub power_w: f64,
    pub energy_pev: f64,
}

#[derive(Deserialize)]
pub struct TransmitRequest {
    pub payload: String,
    pub physics_mode: Option<String>,
}

/// GET /api/v1/ledger/verify?event_hash=0x...
async fn verify_hash(
    State(state): State<AppState>,
    Query(query): Query<HashQuery>,
) -> impl IntoResponse {
    let ledger = state.ledger.lock().await;
    let is_anchored = ledger.verify_anchored(&query.event_hash).unwrap_or(false);
    let last = ledger.get_last();

    let response = AnchorStatus {
        is_anchored,
        block_height: ledger.proofs_len() as u64,
        energy_cost_mj: last.as_ref().map(|p| p.energy_used_j / 1e6).unwrap_or(0.0),
        physics_mode: last.as_ref().map(|p| p.physics_mode.clone()).unwrap_or_default(),
    };
    (StatusCode::OK, Json(response))
}

/// GET /api/v1/ledger/stats
async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    let ledger = state.ledger.lock().await;
    let channel: tokio::sync::MutexGuard<'_, crate::channel::PoissonChannel> = state.channel.lock().await;

    let stats = LedgerStats {
        total_proofs: ledger.proofs_len(),
        total_energy_mj: ledger.total_energy_consumed_megajoules(),
        success_rate: ledger.stats().map(|s| s["success_rate"].as_f64().unwrap_or(0.0)).unwrap_or(0.0),
        current_mode: channel.historical_context(),
    };
    (StatusCode::OK, Json(stats))
}

/// POST /api/v1/transmit
async fn transmit_proof(
    State(_state): State<AppState>,
    Json(req): Json<TransmitRequest>,
) -> impl IntoResponse {
    // Nota: para uso real, precisaria do ServerState completo com RNG e LDPC
    // Este endpoint retorna uma confirmação de recebimento
    let response = serde_json::json!({
        "status": "RECEIVED",
        "payload_hash": format!("0x{:064x}", req.payload.len()),
        "physics_mode": req.physics_mode.unwrap_or_else(|| "minerva".to_string()),
    });
    (StatusCode::ACCEPTED, Json(response))
}

/// POST /api/v1/seti/analyze
async fn analyze_seti(
    State(_state): State<AppState>,
    Json(req): Json<SetiRequest>,
) -> impl IntoResponse {
    let config = SetiConfig {
        mode: crate::seti::GalacticMode::GlashowResonance,
        target_distance_ly: req.distance_ly,
        transmitter_power_w: req.power_w,
        detector_area_m2: 1e12,
        energy_per_neutrino_ev: req.energy_pev * 1e15,
    };
    let result = SetiResult::from_config(&config);
    (StatusCode::OK, Json(result))
}

/// GET /health
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "status": "operational",
        "version": "1.4.0",
        "protocol": "ARKHE-N",
        "modes": ["minerva", "cevns", "saenz", "coh_ar750", "km3net", "cooled_muon"]
    })))
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/ledger/verify", get(verify_hash))
        .route("/api/v1/ledger/stats", get(get_stats))
        .route("/api/v1/transmit", post(transmit_proof))
        .route("/api/v1/seti/analyze", post(analyze_seti))
        .route("/health", get(health_check))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    ////use tower::ServiceExt as _;
    //use tower::Service;

    use tower::ServiceExt;

    fn mock_state() -> AppState {
        AppState {
            ledger: Arc::new(Mutex::new(TransmissionLedger::new())),
            channel: Arc::new(Mutex::new(PoissonChannel::minerva_default())),
        }
    }

    #[tokio::test]
    async fn test_health() {
        let state = mock_state();
        let app = create_router(state);
        let response = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_seti_analyze() {
        let state = mock_state();
        let app = create_router(state);
        let body = serde_json::json!({
            "distance_ly": 1000.0,
            "power_w": 1e15,
            "energy_pev": 6.3
        });
        let response = app
            .oneshot(
                Request::post("/api/v1/seti/analyze")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap()
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

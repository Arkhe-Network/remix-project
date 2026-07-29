pub mod auth;
pub mod compile;
pub mod debug;
pub mod deploy;
pub mod plugins;
pub mod ledger;

use axum::{Router, routing::{get, post}, middleware};
use std::sync::Arc;
use crate::orchestration::orchestrator::Orchestrator;
use crate::api::compile::{compile_contract, list_versions};
use crate::api::debug::{create_session, step, get_state};
use crate::api::deploy::{deploy_contract, get_status};
use crate::api::plugins::{list_plugins, activate_plugin, deactivate_plugin, call_plugin};
use crate::api::ledger::{list_actions, get_action, verify_action};
use crate::api::auth::{register, login, did_auth_middleware};

pub fn create_routes(orchestrator: Arc<Orchestrator>) -> Router {
    Router::new()
        // Auth (no middleware)
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        // Protected routes
        .nest(
            "/api",
            Router::new()
                // Compilação
                .route("/compile", post(compile_contract))
                .route("/compile/versions", get(list_versions))
                // Debugging
                .route("/debug/session", post(create_session))
                .route("/debug/session/:id/step", post(step))
                .route("/debug/session/:id/state", get(get_state))
                // Deployment
                .route("/deploy", post(deploy_contract))
                .route("/deploy/:tx_hash/status", get(get_status))
                // Plugins
                .route("/plugins", get(list_plugins))
                .route("/plugins/:id/activate", post(activate_plugin))
                .route("/plugins/:id/deactivate", post(deactivate_plugin))
                .route("/plugins/:id/call", post(call_plugin))
                // Ledger
                .route("/ledger/actions", get(list_actions))
                .route("/ledger/actions/:id", get(get_action))
                .route("/ledger/verify/:id", get(verify_action))
                .with_state(orchestrator)
                .layer(middleware::from_fn(did_auth_middleware))
        )
}

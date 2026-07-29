use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::{Response, IntoResponse, Json},
};
use serde_json::json;
use cathedral_identity::{Did, verify_signature};

pub async fn did_auth_middleware(
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| (axum::http::StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Missing Authorization header"
        }))))?;

    let parts: Vec<&str> = auth_header.split_whitespace().collect();
    if parts.len() != 2 || parts[0] != "Bearer" {
        return Err((axum::http::StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid Authorization format. Use: Bearer <did>:<signature>"
        }))));
    }

    let token = parts[1];
    let (did_str, sig_hex) = token.split_once(':')
        .ok_or_else(|| (axum::http::StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid token format. Use: <did>:<signature>"
        }))))?;

    let did = Did::parse(did_str)
        .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid DID format"
        }))))?;

    let sig = hex::decode(sig_hex)
        .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid signature format (expected hex)"
        }))))?;

    let message = req.uri().to_string();
    if !verify_signature(&did, &sig, message.as_bytes()) {
        return Err((axum::http::StatusCode::UNAUTHORIZED, Json(json!({
            "error": "Invalid signature"
        }))));
    }

    req.extensions_mut().insert(did);
    Ok(next.run(req).await)
}

pub async fn register() -> Json<serde_json::Value> {
    Json(json!({}))
}

pub async fn login() -> Json<serde_json::Value> {
    Json(json!({}))
}

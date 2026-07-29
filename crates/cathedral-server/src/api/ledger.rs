use axum::Json;

pub async fn list_actions() -> Json<serde_json::Value> {
    Json(serde_json::json!([]))
}

pub async fn get_action() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

pub async fn verify_action() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

use axum::Json;

pub async fn list_plugins() -> Json<serde_json::Value> {
    Json(serde_json::json!([]))
}

pub async fn activate_plugin() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

pub async fn deactivate_plugin() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

pub async fn call_plugin() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

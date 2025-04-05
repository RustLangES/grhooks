use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::Value;

use crate::GlobalConfig;

pub async fn webhook_handler(
    header: HeaderMap,
    State(config): State<GlobalConfig>,
    Path(path): Path<String>,
    Json(value): Json<Value>,
) -> impl IntoResponse {
    tracing::debug!("Path: {path:?}");
    tracing::trace!("Value: {value:?}");
    let config = config.read().await;

    let Some(webhook) = config
        .webhooks
        .iter()
        .find(|w| w.path.as_ref().is_some_and(|p| *p == path))
    else {
        return (
            StatusCode::NOT_FOUND,
            format!("Path {path:?} not registered"),
        );
    };

    let Some(event_type) = header.get("X-GitHub-Event").and_then(|v| v.to_str().ok()) else {
        return (
            StatusCode::BAD_REQUEST,
            format!("Missing X-GitHub-Event header"),
        );
    };

    if !webhook.events.is_empty() && !webhook.events.contains(&"*".to_string()) {
        if !webhook.events.contains(&event_type.to_string()) {
            return (
                StatusCode::BAD_REQUEST,
                format!("Event '{event_type}' not allowed"),
            );
        }
    }

    match grhooks_core::execute_command(&webhook, event_type, &value).await {
        Ok(output) => (StatusCode::OK, output),
        Err(e) => {
            tracing::error!("Error executing command: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

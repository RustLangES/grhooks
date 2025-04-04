use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::Value;
use srtemplate::SrTemplate;

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

    let ctx = Arc::new(SrTemplate::default());
    ctx.add_variable("event.type", event_type);
    grhooks_core::process_value(ctx.clone(), "event", &value);

    match ctx.render(&webhook.command.trim()) {
        Ok(rendered_cmd) => match execute_command(&rendered_cmd).await {
            Ok(output) => (StatusCode::OK, output),
            Err(e) => {
                tracing::error!("Error executing command: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        },
        Err(e) => {
            tracing::error!("Error executing command: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

async fn execute_command(cmd: &str) -> std::io::Result<String> {
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .await?;
    tracing::debug!("Command: {cmd}");

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Command failed ({}):\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    let output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    tracing::debug!("Command Output: {output}");

    Ok(output)
}

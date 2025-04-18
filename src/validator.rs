use axum::{
    extract::{Path, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use grhooks_config::grhooks_origin::{Error as OriginError, Origin, WebhookOrigin};
use grhooks_core::render_secret;

use crate::{GlobalConfig, errors::HeaderValidationError};

pub async fn validate_headers(
    request: Request,
    next: Next,
) -> Result<Response, HeaderValidationError> {
    let headers = request.headers();
    let origin = determine_origin(headers)?;
    origin.validate_headers(headers)?;
    Ok(next.run(request).await)
}

pub async fn validate_signature_middleware(
    Path(path): Path<String>,
    State(config): State<GlobalConfig>,
    request: Request,
    next: Next,
) -> Result<Response, HeaderValidationError> {
    let config = config.read().await;
    let headers = request.headers().clone();

    let webhook_config = config
        .webhooks
        .iter()
        .find(|w| w.path == path)
        .ok_or(HeaderValidationError::WebhookNotFound)?;

    let validator = webhook_config.origin;
    let event_type = validator.extract_event_type(&headers)?;

    if let Some(secret) = &webhook_config.secret {
        let secret = render_secret(secret, &event_type);
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(HeaderValidationError::AxumError)?;

        validator.validate_signature(&headers, &secret, &bytes)?;

        let request = Request::from_parts(parts, axum::body::Body::from(bytes));
        Ok(next.run(request).await)
    } else {
        Ok(next.run(request).await)
    }
}

fn determine_origin(headers: &HeaderMap) -> Result<Origin, OriginError> {
    if headers.contains_key("X-GitHub-Event") {
        Ok(Origin::GitHub)
    } else if headers.contains_key("X-Gitlab-Event") {
        Ok(Origin::GitLab)
    } else {
        Err(OriginError::MissingHeader("X-*-Event"))
    }
}

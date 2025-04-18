use axum::{
    extract::{Path, Request, State},
    middleware::Next,
    response::Response,
};
use grhooks_core::render_secret;
use grhooks_origin::{Origin, WebhookOrigin};

use crate::{GlobalConfig, errors::HeaderValidationError};

pub async fn validate_headers(
    request: Request,
    next: Next,
) -> Result<Response, HeaderValidationError> {
    let headers = request.headers();
    let origin = Origin::try_from(headers)?;
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
    let origin = Origin::try_from(&headers)?;

    let webhook_config = config
        .webhooks
        .iter()
        .find(|w| w.path == path)
        .ok_or(HeaderValidationError::WebhookNotFound)?;

    let event_type = origin.extract_event_type(&headers)?;

    if let Some(secret) = &webhook_config.secret {
        let secret = render_secret(secret, &event_type);
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(HeaderValidationError::AxumError)?;

        origin.validate_signature(&headers, &secret, &bytes)?;

        let request = Request::from_parts(parts, axum::body::Body::from(bytes));
        Ok(next.run(request).await)
    } else {
        Ok(next.run(request).await)
    }
}

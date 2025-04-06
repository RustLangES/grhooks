use std::usize;

use crate::GlobalConfig;
use axum::{
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};

#[derive(Debug)]
pub enum HeaderValidationError {
    MissingHeader(&'static str),
    InvalidSignature,
    InvalidUserAgent,
    WebhookNotFound,
    AxumError(axum::Error),
}

impl IntoResponse for HeaderValidationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            HeaderValidationError::MissingHeader(header) => (
                StatusCode::BAD_REQUEST,
                format!("Missing required header: {header}"),
            ),
            HeaderValidationError::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, "Invalid signature".to_string())
            }
            HeaderValidationError::InvalidUserAgent => {
                (StatusCode::BAD_REQUEST, "Invalid User-Agent".to_string())
            }
            HeaderValidationError::WebhookNotFound => {
                (StatusCode::NOT_FOUND, "Webhook not configured".to_string())
            }
            HeaderValidationError::AxumError(error) => (StatusCode::BAD_REQUEST, error.to_string()),
        };
        (status, message).into_response()
    }
}

pub async fn validate_headers(
    request: Request,
    next: Next,
) -> Result<Response, HeaderValidationError> {
    let headers = request.headers();

    validate_required_headers(headers)?;
    validate_user_agent(headers)?;

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

    let Some(webhook_config) = config.webhooks.iter().find(|w| w.path == path) else {
        return Err(HeaderValidationError::WebhookNotFound);
    };

    let Some(event_type) = headers.get("X-GitHub-Event").and_then(|v| v.to_str().ok()) else {
        return Err(HeaderValidationError::MissingHeader("X-GitHub-Event"));
    };

    if let Some(secret) = &webhook_config.secret {
        let secret = grhooks_core::render_secret(secret, event_type);
        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(HeaderValidationError::AxumError)?;

        validate_signature(&headers, &secret, &bytes)?;

        let request = Request::from_parts(parts, axum::body::Body::from(bytes));
        Ok(next.run(request).await)
    } else {
        Ok(next.run(request).await)
    }
}

fn validate_required_headers(headers: &HeaderMap) -> Result<(), HeaderValidationError> {
    const REQUIRED_HEADERS: [&str; 4] = [
        "X-GitHub-Hook-ID",
        "X-GitHub-Event",
        "X-GitHub-Delivery",
        "User-Agent",
    ];

    for header in REQUIRED_HEADERS {
        if !headers.contains_key(header) {
            return Err(HeaderValidationError::MissingHeader(header));
        }
    }

    Ok(())
}

fn validate_user_agent(headers: &HeaderMap) -> Result<(), HeaderValidationError> {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .ok_or(HeaderValidationError::MissingHeader("User-Agent"))?;

    if !user_agent.starts_with("GitHub-Hookshot/") {
        return Err(HeaderValidationError::InvalidUserAgent);
    }

    Ok(())
}

fn validate_signature(
    headers: &HeaderMap,
    secret: &str,
    body: impl AsRef<[u8]>,
) -> Result<(), HeaderValidationError> {
    let (expected_signature, signature) = match headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
    {
        Some(signature) => {
            let signature = signature.trim_start_matches("sha256=");
            let mut mac = Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(body.as_ref());
            let expected_signature = hex::encode(mac.finalize().into_bytes());
            (expected_signature, signature)
        }
        None => {
            let signature = headers
                .get("X-Hub-Signature")
                .and_then(|v| v.to_str().ok())
                .ok_or(HeaderValidationError::MissingHeader("X-Hub-Signature"))?;
            let signature = signature.trim_start_matches("sha1=");
            let mut mac = Hmac::<sha1::Sha1>::new_from_slice(secret.as_bytes())
                .expect("HMAC can take key of any size");
            mac.update(body.as_ref());
            let expected_signature = hex::encode(mac.finalize().into_bytes());
            (expected_signature, signature)
        }
    };

    if !constant_time_eq::constant_time_eq(signature.as_bytes(), expected_signature.as_bytes()) {
        return Err(HeaderValidationError::InvalidSignature);
    }

    Ok(())
}

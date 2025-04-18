use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

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

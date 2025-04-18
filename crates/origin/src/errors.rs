use std::fmt::Debug;

use axum::http::StatusCode;
use axum::response::IntoResponse;

pub enum Error {
    MissingHeader(&'static str),
    InvalidSignature,
    InvalidUserAgent,
    UnsupportedEvent,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingHeader(header) => write!(f, "Missing required header: {}", header),
            Error::InvalidSignature => write!(f, "Invalid signature"),
            Error::InvalidUserAgent => write!(f, "Invalid user agent"),
            Error::UnsupportedEvent => write!(f, "Unsupported event type"),
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::MissingHeader(header) => (
                StatusCode::BAD_REQUEST,
                format!("Missing required header: {}", header),
            )
                .into_response(),
            Error::InvalidSignature => {
                (StatusCode::BAD_REQUEST, "Invalid signature").into_response()
            }
            Error::InvalidUserAgent => {
                (StatusCode::BAD_REQUEST, "Invalid user agent").into_response()
            }
            Error::UnsupportedEvent => {
                (StatusCode::BAD_REQUEST, "Unsupported event type").into_response()
            }
        }
    }
}

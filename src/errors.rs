use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum HeaderValidationError {
    WebhookNotFound,
    OriginValidation(grhooks_origin::Error),
    AxumError(axum::Error),
}

impl IntoResponse for HeaderValidationError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            HeaderValidationError::WebhookNotFound => {
                (StatusCode::NOT_FOUND, "Webhook not configured".to_string())
            }
            HeaderValidationError::AxumError(error) => (StatusCode::BAD_REQUEST, error.to_string()),
            HeaderValidationError::OriginValidation(error) => return error.into_response(),
        };
        (status, message).into_response()
    }
}

impl From<grhooks_origin::Error> for HeaderValidationError {
    fn from(error: grhooks_origin::Error) -> Self {
        HeaderValidationError::OriginValidation(error)
    }
}

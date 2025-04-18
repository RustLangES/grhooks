use super::{Error, WebhookOrigin};
use axum::http::HeaderMap;

pub struct GitLabValidator;

impl WebhookOrigin for GitLabValidator {
    fn validate_headers(&self, headers: &HeaderMap) -> Result<(), Error> {
        const REQUIRED_HEADERS: [&str; 3] =
            ["X-Gitlab-Event", "X-Gitlab-Webhook-UUID", "X-Gitlab-UUID"];

        for header in REQUIRED_HEADERS {
            if !headers.contains_key(header) {
                return Err(Error::MissingHeader(header));
            }
        }

        let user_agent = headers
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .ok_or(Error::MissingHeader("User-Agent"))?;

        if !user_agent.starts_with("Gitlab/") {
            return Err(Error::InvalidUserAgent);
        }

        Ok(())
    }

    fn extract_event_type(&self, headers: &HeaderMap) -> Result<String, Error> {
        headers
            .get("X-Gitlab-Event")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string)
            .ok_or(Error::MissingHeader("X-Gitlab-Event"))
    }

    fn validate_signature(
        &self,
        headers: &HeaderMap,
        _secret: &str,
        _body: &[u8],
    ) -> Result<(), Error> {
        _ = headers
            .get("X-Gitlab-Instance")
            .and_then(|v| v.to_str().ok())
            .ok_or(Error::MissingHeader("X-Gitlab-Event-UUID"))?;

        Ok(())
    }
}

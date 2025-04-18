use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::Sha256;

use crate::{Error, WebhookOrigin};

pub struct GitHubValidator;

impl WebhookOrigin for GitHubValidator {
    fn validate_headers(&self, headers: &HeaderMap) -> Result<(), Error> {
        const REQUIRED_HEADERS: [&str; 4] = [
            "X-GitHub-Hook-ID",
            "X-GitHub-Event",
            "X-GitHub-Delivery",
            "User-Agent",
        ];

        for header in REQUIRED_HEADERS {
            if !headers.contains_key(header) {
                return Err(Error::MissingHeader(header));
            }
        }

        let user_agent = headers
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .ok_or(Error::MissingHeader("User-Agent"))?;

        if !user_agent.starts_with("GitHub-Hookshot/") {
            return Err(Error::InvalidUserAgent);
        }

        Ok(())
    }

    fn extract_event_type(&self, headers: &HeaderMap) -> Result<String, Error> {
        headers
            .get("X-GitHub-Event")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or(Error::MissingHeader("X-GitHub-Event"))
    }

    fn validate_signature(
        &self,
        headers: &HeaderMap,
        secret: &str,
        body: &[u8],
    ) -> Result<(), Error> {
        let (expected_signature, signature) = match headers
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
        {
            Some(signature) => {
                let signature = signature.trim_start_matches("sha256=");
                let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
                    .map_err(|_| Error::InvalidSignature)?;
                mac.update(body);
                let expected_signature = hex::encode(mac.finalize().into_bytes());
                (expected_signature, signature)
            }
            None => {
                let signature = headers
                    .get("X-Hub-Signature")
                    .and_then(|v| v.to_str().ok())
                    .ok_or(Error::MissingHeader("X-Hub-Signature"))?;
                let signature = signature.trim_start_matches("sha1=");
                let mut mac = Hmac::<Sha1>::new_from_slice(secret.as_bytes())
                    .map_err(|_| Error::InvalidSignature)?;
                mac.update(body);
                let expected_signature = hex::encode(mac.finalize().into_bytes());
                (expected_signature, signature)
            }
        };

        if !constant_time_eq::constant_time_eq(signature.as_bytes(), expected_signature.as_bytes())
        {
            return Err(Error::InvalidSignature);
        }

        Ok(())
    }
}

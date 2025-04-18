#![allow(clippy::missing_errors_doc)]

use axum::http::HeaderMap;
use serde::Deserialize;

pub use crate::errors::Error;

mod errors;
mod github;
mod gitlab;

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    #[default]
    GitHub,
    GitLab,
}

pub trait WebhookOrigin {
    fn validate_headers(&self, headers: &HeaderMap) -> Result<(), Error>;
    fn extract_event_type(&self, headers: &HeaderMap) -> Result<String, Error>;
    fn validate_signature(
        &self,
        headers: &HeaderMap,
        secret: &str,
        body: &[u8],
    ) -> Result<(), Error>;
}

impl WebhookOrigin for Origin {
    fn validate_headers(&self, headers: &HeaderMap) -> Result<(), Error> {
        match self {
            Origin::GitHub => github::GitHubValidator.validate_headers(headers),
            Origin::GitLab => gitlab::GitLabValidator.validate_headers(headers),
        }
    }

    fn extract_event_type(&self, headers: &HeaderMap) -> Result<String, Error> {
        match self {
            Origin::GitHub => github::GitHubValidator.extract_event_type(headers),
            Origin::GitLab => gitlab::GitLabValidator.extract_event_type(headers),
        }
    }

    fn validate_signature(
        &self,
        headers: &HeaderMap,
        secret: &str,
        body: &[u8],
    ) -> Result<(), Error> {
        match self {
            Origin::GitHub => github::GitHubValidator.validate_signature(headers, secret, body),
            Origin::GitLab => gitlab::GitLabValidator.validate_signature(headers, secret, body),
        }
    }
}

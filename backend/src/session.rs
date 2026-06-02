//! HTTP extractor for authenticated user sessions.
//!
//! This module provides a small Axum extractor that reads the `Authorization`
//! header (expecting a `Bearer <token>` value) and exposes a typed
//! `UserSession` for route handlers. The extractor intentionally performs only
//! minimal parsing and does not validate tokens — callers may extend it to
//! verify signatures or look up session state in Redis/DB as needed.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};
use http::header::AUTHORIZATION;
use serde::Deserialize;

/// Represents an authenticated user session extracted from the request.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct UserSession {
    /// The principal identifier extracted from the Authorization token.
    /// For PrediFi this is commonly the user's wallet address (e.g. `G...`).
    pub user_address: String,
}

/// Simple extractor that expects `Authorization: Bearer <token>`.
///
/// The extractor treats the bearer token as the user's address string and
/// returns `401 Unauthorized` when the header is missing or malformed. Handlers
/// that want optional authentication may accept `Option<UserSession>` instead.
#[async_trait]
impl<S> FromRequestParts<S> for UserSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Read the Authorization header
        let header_value = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing authorization header"))?;

        // Expect "Bearer <token>"
        let mut parts = header_value.splitn(2, ' ');
        let scheme = parts.next().unwrap_or("");
        let token = parts.next().unwrap_or("");

        if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() {
            return Err((StatusCode::UNAUTHORIZED, "invalid authorization header"));
        }

        // Minimal validation: token length and characters can be checked here.
        // For now we treat the token as the canonical user address.
        Ok(UserSession {
            user_address: token.to_string(),
        })
    }
}

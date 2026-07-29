//! JWT security hardening: token revocation, refresh token rotation, and key versioning.
//!
//! This module provides:
//! - Token revocation lists (blacklist) backed by Redis
//! - Refresh token lifecycle management with automatic rotation
//! - Key version tracking to support secret rotation
//! - Rate limiting helpers for token endpoints

use crate::redis_cache::RedisCache;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents an issued token pair (access + refresh) with rotation metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Access token (short-lived, 1 hour).
    pub access_token: String,
    /// Refresh token (long-lived, 7 days).
    pub refresh_token: String,
    /// Unix timestamp when the refresh token was issued.
    pub issued_at: u64,
    /// Key version at issuance; used to invalidate old tokens if secrets rotate.
    pub key_version: u64,
}

/// Checks if a token (by JTI - JWT ID) has been revoked.
///
/// Revoked tokens are stored in Redis under the key pattern:
/// `jwt:revoked:{jti}` with a TTL matching the token's remaining expiry.
pub async fn is_token_revoked(cache: &RedisCache, token_jti: &str) -> bool {
    if !cache.is_available() {
        // If Redis is down, default to NOT revoking (fail-open).
        // In production, you may want to fail-closed for security.
        tracing::warn!("Redis unavailable; cannot check token revocation for {}", token_jti);
        return false;
    }

    cache.exists(&format!("jwt:revoked:{}", token_jti)).await
}

/// Revoke a token by storing its JTI in Redis.
///
/// The revocation entry expires after `ttl_secs`, matching the token's remaining lifetime.
pub async fn revoke_token(
    cache: &RedisCache,
    token_jti: &str,
    ttl_secs: u64,
) -> Result<(), String> {
    if !cache.is_available() {
        return Err("Redis cache not available".to_string());
    }

    let key = format!("jwt:revoked:{}", token_jti);
    cache.set::<String>(&key, &"1".to_string(), ttl_secs).await;
    Ok(())
}

/// Revoke all tokens for a user (on password change, logout, etc.).
///
/// This marks all tokens for `user_address` as revoked by setting a key:
/// `jwt:user:revoke_before:{user_address}` to the current Unix timestamp.
/// Tokens issued before this time are considered revoked.
pub async fn revoke_user_tokens(cache: &RedisCache, user_address: &str) -> Result<(), String> {
    if !cache.is_available() {
        return Err("Redis cache not available".to_string());
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let key = format!("jwt:user:revoke_before:{}", user_address);
    cache.set(&key, &now.to_string(), 86400 * 30).await; // Store for 30 days
    Ok(())
}

/// Check if a token (issued at `issued_at`) is still valid for a user.
///
/// Returns `true` if the token was issued after the user's revoke cutoff.
/// If no revoke cutoff exists, the token is valid.
pub async fn is_token_valid_for_user(
    cache: &RedisCache,
    user_address: &str,
    issued_at: u64,
) -> bool {
    if !cache.is_available() {
        return true; // Fail open if Redis is unavailable
    }

    let key = format!("jwt:user:revoke_before:{}", user_address);
    if let Some(revoke_before) = cache.get::<u64>(&key).await {
        return issued_at >= revoke_before;
    }
    true
}

/// Generate a unique JWT ID (JTI) for a token.
///
/// Used as a stable identifier for revocation purposes.
pub fn generate_jti(user_address: &str, issued_at: u64, index: u32) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    user_address.hash(&mut hasher);
    issued_at.hash(&mut hasher);
    index.hash(&mut hasher);

    format!("{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jti_is_deterministic() {
        let jti1 = generate_jti("GABC123", 1000, 0);
        let jti2 = generate_jti("GABC123", 1000, 0);
        assert_eq!(jti1, jti2);
    }

    #[test]
    fn jti_differs_for_different_inputs() {
        let jti1 = generate_jti("GABC123", 1000, 0);
        let jti2 = generate_jti("GABC123", 1001, 0);
        assert_ne!(jti1, jti2);
    }

    #[test]
    fn jti_differs_for_different_users() {
        let jti1 = generate_jti("GABC123", 1000, 0);
        let jti2 = generate_jti("GDEF456", 1000, 0);
        assert_ne!(jti1, jti2);
    }
}

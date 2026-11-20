//! Wallet challenge/response login (#2071).
//!
//! `POST /auth/challenge` stores a short-lived random nonce for a Stellar
//! address; `POST /auth/verify` checks an ed25519 signature over a fixed,
//! domain-separated message containing that nonce and issues the same
//! access/refresh pair used by `/auth/refresh`.

use axum::{extract::State, Json};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;

use crate::{
    config::Config,
    redis_cache::RedisCache,
    response::{error_codes, ApiResponse},
    routes::v1::AppState,
};

/// Nonce lifetime for a login challenge.
pub const CHALLENGE_TTL_SECS: u64 = 300;
/// Domain separator preventing cross-service signature replay.
pub const LOGIN_DOMAIN: &str = "predifi.login.v1";

fn redis_key(address: &str) -> String {
    format!("auth:challenge:{address}")
}

/// Message the wallet must sign. Kept identical on challenge creation and
/// verification so clients can display exactly what they are signing.
pub fn challenge_message(address: &str, nonce: &str) -> String {
    format!("PrediFi wallet login\nDomain: {LOGIN_DOMAIN}\nAddress: {address}\nNonce: {nonce}")
}

/// In-memory nonce store used only when Redis is unavailable (local dev and
/// unit tests). Production deployments always use Redis through [`RedisCache`].
fn fallback_challenges() -> &'static Mutex<HashMap<String, (String, Instant)>> {
    static STORE: OnceLock<Mutex<HashMap<String, (String, Instant)>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn store_challenge(cache: &RedisCache, address: &str, nonce: &str, ttl_secs: u64) {
    if cache.is_available() {
        cache.set(&redis_key(address), &nonce.to_string(), ttl_secs).await;
    } else if let Ok(mut store) = fallback_challenges().lock() {
        store.insert(
            address.to_string(),
            (nonce.to_string(), Instant::now() + Duration::from_secs(ttl_secs)),
        );
    }
}

async fn take_challenge(cache: &RedisCache, address: &str) -> Option<String> {
    if cache.is_available() {
        let nonce = cache.get::<String>(&redis_key(address)).await;
        // Single use: consume the nonce even when the signature later fails.
        cache.delete(&redis_key(address)).await;
        nonce
    } else if let Ok(mut store) = fallback_challenges().lock() {
        let entry = store.remove(address)?;
        (entry.1 > Instant::now()).then_some(entry.0)
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub address: String,
    pub nonce: String,
    /// Base64-encoded 64-byte ed25519 signature.
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub address: String,
    pub nonce: String,
    pub message: String,
    pub expires_in: u64,
}

/// `POST /api/v1/auth/challenge` — issue a nonce for a wallet login.
pub async fn challenge(
    State(state): State<AppState>,
    Json(body): Json<ChallengeRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    if crate::validated_types::StellarAddress::new(body.address.clone()).is_err() {
        return ApiResponse::<()>::error(
            StatusCode::BAD_REQUEST,
            error_codes::INVALID_INPUT,
            "address must be a valid Stellar account address",
        )
        .into_response();
    }

    let nonce_bytes = rand::random::<[u8; 32]>();
    let nonce: String = nonce_bytes.iter().map(|b| format!("{b:02x}")).collect();
    store_challenge(&state.redis, &body.address, &nonce, CHALLENGE_TTL_SECS).await;

    ApiResponse::success(ChallengeResponse {
        message: challenge_message(&body.address, &nonce),
        address: body.address,
        nonce,
        expires_in: CHALLENGE_TTL_SECS,
    })
    .into_response()
}

/// Core verification logic, shared by the handler and unit tests.
///
/// All failures collapse to `Err(())` so callers return one generic 401 and
/// never disclose whether the address, nonce, or signature was at fault.
pub async fn verify_login(
    cache: &RedisCache,
    config: &Config,
    body: &VerifyRequest,
) -> Result<crate::jwt_security::TokenPair, ()> {
    let public_bytes = stellar_strkey::ed25519::PublicKey::from_string(&body.address)
        .map_err(|_| ())?
        .0;
    let verifying_key = VerifyingKey::from_bytes(&public_bytes).map_err(|_| ())?;
    let signature_bytes = BASE64.decode(&body.signature).map_err(|_| ())?;
    let signature = Signature::from_slice(&signature_bytes).map_err(|_| ())?;

    let expected_nonce = take_challenge(cache, &body.address).await.ok_or(())?;
    if bool::from(
        expected_nonce
            .as_bytes()
            .ct_eq(body.nonce.as_bytes()),
    ) == false
    {
        return Err(());
    }

    verifying_key
        .verify(challenge_message(&body.address, &body.nonce).as_bytes(), &signature)
        .map_err(|_| ())?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ())?
        .as_secs();
    crate::jwt_security::issue_token_pair(
        &body.address,
        &config.secret_key,
        now,
        config.jwt_key_version,
    )
    .map_err(|_| ())
}

/// `POST /api/v1/auth/verify` — verify a wallet signature and issue tokens.
pub async fn verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyRequest>,
) -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    match verify_login(&state.redis, &state.config, &body).await {
        Ok(pair) => ApiResponse::success(json!({
            "access_token": pair.access_token,
            "refresh_token": pair.refresh_token,
        }))
        .into_response(),
        Err(()) => ApiResponse::<()>::error(
            StatusCode::UNAUTHORIZED,
            error_codes::UNAUTHORIZED,
            "invalid login credentials",
        )
        .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn fixture() -> (ed25519_dalek::SigningKey, String, RedisCache, Config) {
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
        let address = stellar_strkey::ed25519::PublicKey(signing_key.verifying_key().to_bytes())
            .to_string();
        (signing_key, address, RedisCache::disabled(), Config::default_for_test())
    }

    fn signed_body(
        signing_key: &ed25519_dalek::SigningKey,
        address: &str,
        nonce: &str,
    ) -> VerifyRequest {
        let message = challenge_message(address, nonce);
        VerifyRequest {
            address: address.to_string(),
            nonce: nonce.to_string(),
            signature: BASE64.encode(signing_key.sign(message.as_bytes()).to_bytes()),
        }
    }

    #[tokio::test]
    async fn valid_signature_issues_token_pair() {
        let (key, address, cache, config) = fixture();
        let nonce = "valid-nonce";
        store_challenge(&cache, &address, nonce, CHALLENGE_TTL_SECS).await;
        let body = signed_body(&key, &address, nonce);
        let pair = verify_login(&cache, &config, &body).await.expect("login succeeds");
        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn wrong_address_is_rejected() {
        let (key, _address, cache, config) = fixture();
        let other = stellar_strkey::ed25519::PublicKey([7u8; 32]).to_string();
        let nonce = "wrong-address-nonce";
        store_challenge(&cache, &other, nonce, CHALLENGE_TTL_SECS).await;
        // Signature made by a different key than the address being claimed.
        let body = signed_body(&key, &other, nonce);
        assert!(verify_login(&cache, &config, &body).await.is_err());
    }

    #[tokio::test]
    async fn nonce_is_single_use() {
        let (key, address, cache, config) = fixture();
        let nonce = "single-use-nonce";
        store_challenge(&cache, &address, nonce, CHALLENGE_TTL_SECS).await;
        let body = signed_body(&key, &address, nonce);
        assert!(verify_login(&cache, &config, &body).await.is_ok());
        assert!(verify_login(&cache, &config, &body).await.is_err());
    }

    #[tokio::test]
    async fn expired_nonce_is_rejected() {
        let (key, address, cache, config) = fixture();
        let nonce = "expired-nonce";
        // Simulate a nonce whose TTL has already elapsed in the local store.
        store_challenge(&cache, &address, nonce, 0).await;
        let body = signed_body(&key, &address, nonce);
        assert!(verify_login(&cache, &config, &body).await.is_err());
    }
}

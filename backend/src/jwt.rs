//! JWT format validation and secret-backed token verification.
//!
//! [`validate_jwt_format`] performs a lightweight structural check before any
//! cryptographic work. [`verify_jwt_secret`] validates that the configured
//! signing secret meets minimum security requirements. [`verify_jwt_token`]
//! combines both checks and verifies the token signature and claims.
//!
//! Algorithm pinning: verification always uses HMAC-SHA256. The token `alg`
//! header is never trusted, which blocks `alg:none` and RS256/HS256 confusion.

use crate::constants::{
    JWT_ACCESS_TOKEN_EXPIRY_SECS, JWT_MIN_LENGTH, JWT_PARTS_COUNT, JWT_REFRESH_TOKEN_EXPIRY_SECS,
    JWT_SECRET_MIN_LENGTH,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// Clock-skew leeway applied to `exp` (seconds).
const JWT_EXP_LEEWAY_SECS: u64 = 30;

/// Errors returned when a token string fails the structural JWT check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtFormatError {
    /// The token is too short to be a plausible JWT.
    TooShort,
    /// The token does not have exactly three dot-separated parts.
    WrongPartCount {
        /// How many parts were actually found.
        found: usize,
    },
    /// One of the three parts is empty.
    EmptyPart {
        /// Zero-based index of the empty part (0 = header, 1 = payload, 2 = signature).
        index: usize,
    },
    /// A part contains characters that are not valid base64url.
    InvalidBase64Url {
        /// Zero-based index of the offending part.
        index: usize,
    },
}

impl std::fmt::Display for JwtFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "token is too short to be a valid JWT"),
            Self::WrongPartCount { found } => write!(
                f,
                "JWT must have exactly {JWT_PARTS_COUNT} parts separated by '.', found {found}"
            ),
            Self::EmptyPart { index } => {
                write!(f, "JWT part {index} (0-indexed) must not be empty")
            }
            Self::InvalidBase64Url { index } => {
                write!(f, "JWT part {index} contains invalid base64url characters")
            }
        }
    }
}

impl std::error::Error for JwtFormatError {}

/// Errors returned when the configured JWT signing secret is unusable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtSecretError {
    /// The secret is empty.
    Empty,
    /// The secret is shorter than [`JWT_SECRET_MIN_LENGTH`].
    TooShort {
        /// Minimum required length in bytes.
        min_length: usize,
    },
}

impl std::fmt::Display for JwtSecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "JWT signing secret must not be empty"),
            Self::TooShort { min_length } => {
                write!(f, "JWT signing secret must be at least {min_length} bytes")
            }
        }
    }
}

impl std::error::Error for JwtSecretError {}

/// Errors returned when JWT verification fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtVerifyError {
    /// The configured signing secret is invalid.
    InvalidSecret(JwtSecretError),
    /// The token failed the structural format check.
    InvalidFormat(JwtFormatError),
    /// The token signature does not match the configured secret.
    InvalidSignature,
    /// The token has expired.
    Expired,
    /// The `token_type` claim does not match the expected type.
    WrongTokenType,
    /// The token was issued under a different signing-key version.
    KeyVersionMismatch,
    /// The token has been revoked (logout, rotation, or user-wide revoke).
    Revoked,
    /// Refresh-token rotation requires a reachable token store (Redis).
    TokenStoreUnavailable,
    /// The token could not be decoded.
    Decode(String),
}

impl std::fmt::Display for JwtVerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSecret(error) => write!(f, "invalid JWT secret: {error}"),
            Self::InvalidFormat(error) => write!(f, "invalid JWT format: {error}"),
            Self::InvalidSignature => write!(f, "JWT signature verification failed"),
            Self::Expired => write!(f, "JWT has expired"),
            Self::WrongTokenType => write!(f, "JWT token type is not valid for this endpoint"),
            Self::KeyVersionMismatch => write!(f, "JWT was issued under a different key version"),
            Self::Revoked => write!(f, "JWT has been revoked"),
            Self::TokenStoreUnavailable => {
                write!(f, "token store unavailable; cannot rotate refresh tokens")
            }
            Self::Decode(message) => write!(f, "failed to decode JWT: {message}"),
        }
    }
}

impl std::error::Error for JwtVerifyError {}

/// Claims extracted from a verified PrediFi JWT.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PredifiClaims {
    /// Subject — the authenticated user's wallet address.
    pub sub: String,
    /// Expiration time as a Unix timestamp (seconds since epoch).
    /// Required for all issued tokens; validated on every call to
    /// `verify_jwt_token`.
    pub exp: u64,
    /// Issued-at time as a Unix timestamp. Used for user-wide revocation.
    #[serde(default)]
    pub iat: u64,
    /// Unique token identifier used for revocation and refresh rotation.
    #[serde(default)]
    pub jti: String,
    /// Token type: "access" or "refresh".
    /// Prevents accidental use of a refresh token in place of an access token.
    #[serde(default = "default_token_type")]
    pub token_type: String,
    /// Key version at token issuance. Bumping key_version on the server
    /// invalidates all tokens issued under previous versions (rotation support).
    #[serde(default = "default_key_version")]
    pub key_version: u64,
}

fn default_token_type() -> String {
    "access".to_string()
}

fn default_key_version() -> u64 {
    0
}

/// Validate that the configured JWT signing secret meets minimum requirements.
pub fn verify_jwt_secret(secret: &str) -> Result<(), JwtSecretError> {
    if secret.is_empty() {
        return Err(JwtSecretError::Empty);
    }
    if secret.len() < JWT_SECRET_MIN_LENGTH {
        return Err(JwtSecretError::TooShort {
            min_length: JWT_SECRET_MIN_LENGTH,
        });
    }
    Ok(())
}

/// Validate that `token` matches the structural JWT format.
pub fn validate_jwt_format(token: &str) -> Result<(), JwtFormatError> {
    if token.len() < JWT_MIN_LENGTH {
        return Err(JwtFormatError::TooShort);
    }

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != JWT_PARTS_COUNT {
        return Err(JwtFormatError::WrongPartCount { found: parts.len() });
    }

    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() {
            return Err(JwtFormatError::EmptyPart { index });
        }

        if !part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '=')
        {
            return Err(JwtFormatError::InvalidBase64Url { index });
        }
    }

    Ok(())
}

/// Build a validation config that pins HS256 and always enforces `exp`.
fn hs256_validation() -> Validation {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.leeway = JWT_EXP_LEEWAY_SECS;
    validation.required_spec_claims.insert("exp".to_string());
    validation
}

/// Verify a JWT against the configured HMAC secret and return decoded claims.
///
/// This function:
/// - Validates the JWT secret
/// - Checks format (3 dot-separated base64url parts)
/// - Rejects any `alg` other than HS256 (`alg:none`, RS256 confusion, etc.)
/// - Verifies the HMAC-SHA256 signature
/// - Checks expiration (`exp` is required)
/// - Validates non-empty subject claim
///
/// **Algorithm Confusion Prevention:**
/// The `Algorithm::HS256` is hard-coded and NEVER negotiated based on the token's
/// `alg` header. The header is inspected first and any mismatch is rejected
/// before signature verification.
pub fn verify_jwt_token(token: &str, secret: &str) -> Result<PredifiClaims, JwtVerifyError> {
    verify_jwt_secret(secret).map_err(JwtVerifyError::InvalidSecret)?;
    validate_jwt_format(token).map_err(JwtVerifyError::InvalidFormat)?;

    let header = decode_header(token).map_err(|_| JwtVerifyError::InvalidSignature)?;
    if header.alg != Algorithm::HS256 {
        return Err(JwtVerifyError::InvalidSignature);
    }

    let token_data = decode::<PredifiClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &hs256_validation(),
    )
    .map_err(map_decode_error)?;

    if token_data.claims.sub.is_empty() {
        return Err(JwtVerifyError::Decode(
            "JWT subject claim must not be empty".to_string(),
        ));
    }

    Ok(token_data.claims)
}

/// Verify a JWT and validate it matches the expected token type and key version.
///
/// This is the stricter validation function for production use.
/// It ensures:
/// - Token is valid (signature, expiration, format)
/// - Token type matches expected (prevents refresh token misuse as access token)
/// - Key version matches current server version (allows key rotation)
pub fn verify_jwt_token_strict(
    token: &str,
    secret: &str,
    expected_type: &str,
    current_key_version: u64,
) -> Result<PredifiClaims, JwtVerifyError> {
    let claims = verify_jwt_token(token, secret)?;

    if claims.token_type != expected_type {
        return Err(JwtVerifyError::WrongTokenType);
    }

    if claims.key_version != current_key_version {
        return Err(JwtVerifyError::KeyVersionMismatch);
    }

    Ok(claims)
}

fn map_decode_error(error: jsonwebtoken::errors::Error) -> JwtVerifyError {
    match error.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtVerifyError::Expired,
        jsonwebtoken::errors::ErrorKind::InvalidSignature
        | jsonwebtoken::errors::ErrorKind::InvalidAlgorithm
        | jsonwebtoken::errors::ErrorKind::InvalidAlgorithmName => JwtVerifyError::InvalidSignature,
        _ => JwtVerifyError::Decode(error.to_string()),
    }
}

/// Extract a bearer token from an `Authorization` header value.
pub fn extract_bearer_token(header_value: &str) -> Option<&str> {
    let (scheme, token) = header_value.split_once(' ')?;
    if scheme.eq_ignore_ascii_case("bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

/// Issue a signed JWT for `sub` with a standard expiry window.
///
/// The token expires in [`JWT_ACCESS_TOKEN_EXPIRY_SECS`] seconds from the
/// current system time.  Returns an error string if signing fails.
pub fn sign_jwt(sub: &str, secret: &str, now_unix: u64) -> Result<String, String> {
    sign_jwt_with_type(sub, secret, now_unix, "access", 0)
}

/// Issue a signed JWT with explicit token type and key version.
///
/// `token_type` should be "access" or "refresh".
/// `key_version` allows invalidating old tokens if the secret is rotated.
pub fn sign_jwt_with_type(
    sub: &str,
    secret: &str,
    now_unix: u64,
    token_type: &str,
    key_version: u64,
) -> Result<String, String> {
    use jsonwebtoken::{encode, EncodingKey, Header};

    let expiry = if token_type == "refresh" {
        now_unix + JWT_REFRESH_TOKEN_EXPIRY_SECS
    } else {
        now_unix + JWT_ACCESS_TOKEN_EXPIRY_SECS
    };

    let claims = PredifiClaims {
        sub: sub.to_string(),
        exp: expiry,
        iat: now_unix,
        jti: uuid::Uuid::new_v4().to_string(),
        token_type: token_type.to_string(),
        key_version,
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
pub fn sign_jwt_for_test(sub: &str, secret: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs();

    encode(
        &Header::new(Algorithm::HS256),
        &PredifiClaims {
            sub: sub.to_string(),
            exp: now + JWT_ACCESS_TOKEN_EXPIRY_SECS,
            iat: now,
            jti: "test-jti".to_string(),
            token_type: "access".to_string(),
            key_version: 0,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("valid test JWT")
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};

    const TEST_SECRET: &str = "predifi-dev-secret-do-not-use-in-production-32";

    // A minimal but structurally valid JWT-shaped token (not a real signed token).
    const VALID_TOKEN: &str =
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    fn claims(sub: &str, exp: u64, token_type: &str, key_version: u64) -> PredifiClaims {
        PredifiClaims {
            sub: sub.to_string(),
            exp,
            iat: 1_700_000_000,
            jti: "test-jti".to_string(),
            token_type: token_type.to_string(),
            key_version,
        }
    }

    #[test]
    fn accepts_well_formed_jwt() {
        assert!(validate_jwt_format(VALID_TOKEN).is_ok());
    }

    #[test]
    fn rejects_empty_string() {
        assert_eq!(validate_jwt_format(""), Err(JwtFormatError::TooShort));
    }

    #[test]
    fn rejects_too_short_token() {
        assert_eq!(validate_jwt_format("short"), Err(JwtFormatError::TooShort));
    }

    #[test]
    fn rejects_token_with_only_two_parts() {
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::WrongPartCount { found: 2 })
        );
    }

    #[test]
    fn rejects_token_with_four_parts() {
        let token = "aaaaaaaaa.bbbbbbbbb.ccccccccc.ddddddddd";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::WrongPartCount { found: 4 })
        );
    }

    #[test]
    fn rejects_token_with_empty_header() {
        let token = ".eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::EmptyPart { index: 0 })
        );
    }

    #[test]
    fn rejects_token_with_empty_payload() {
        let token = "eyJhbGciOiJIUzI1NiJ9..SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::EmptyPart { index: 1 })
        );
    }

    #[test]
    fn rejects_token_with_empty_signature() {
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::EmptyPart { index: 2 })
        );
    }

    #[test]
    fn rejects_token_with_invalid_characters_in_header() {
        let token = "eyJhbGci OiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fw";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::InvalidBase64Url { index: 0 })
        );
    }

    #[test]
    fn rejects_token_with_plus_sign() {
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fw+MeJf";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::InvalidBase64Url { index: 2 })
        );
    }

    #[test]
    fn accepts_token_with_padding_equals() {
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c==";
        assert!(validate_jwt_format(token).is_ok());
    }

    #[test]
    fn accepts_token_with_url_safe_chars() {
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fw-MeJf36POk6yJV_adQssw5c";
        assert!(validate_jwt_format(token).is_ok());
    }

    #[test]
    fn verify_jwt_secret_rejects_empty_and_short_values() {
        assert_eq!(verify_jwt_secret(""), Err(JwtSecretError::Empty));
        assert_eq!(
            verify_jwt_secret("too-short"),
            Err(JwtSecretError::TooShort {
                min_length: JWT_SECRET_MIN_LENGTH
            })
        );
    }

    #[test]
    fn verify_jwt_secret_accepts_minimum_length_secret() {
        assert!(verify_jwt_secret(TEST_SECRET).is_ok());
    }

    #[test]
    fn verify_jwt_token_accepts_valid_signed_token() {
        let token = sign_jwt_for_test("GABC123", TEST_SECRET);
        let claims = verify_jwt_token(&token, TEST_SECRET).expect("valid token");
        assert_eq!(claims.sub, "GABC123");
        assert_eq!(claims.token_type, "access");
        assert!(!claims.jti.is_empty());
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn verify_jwt_token_rejects_wrong_secret() {
        let token = sign_jwt_for_test("GABC123", TEST_SECRET);
        let error = verify_jwt_token(&token, "another-secret-that-is-long-enough-123456")
            .expect_err("wrong secret");
        assert_eq!(error, JwtVerifyError::InvalidSignature);
    }

    #[test]
    fn verify_jwt_token_rejects_malformed_token_before_crypto() {
        let error = verify_jwt_token("not-a-jwt", TEST_SECRET).expect_err("malformed token");
        assert!(matches!(error, JwtVerifyError::InvalidFormat(_)));
    }

    #[test]
    fn verify_jwt_token_rejects_expired_token() {
        let expired_claims = claims("GABC123", 1, "access", 0);
        let token = encode(
            &Header::new(Algorithm::HS256),
            &expired_claims,
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .expect("signed expired token");
        let error = verify_jwt_token(&token, TEST_SECRET).expect_err("expired token");
        assert_eq!(error, JwtVerifyError::Expired);
    }

    #[test]
    fn verify_jwt_token_rejects_alg_none() {
        // Header alg:none, dummy non-empty signature so format checks pass.
        // Payload: {"sub":"GABC123","exp":9999999999,"token_type":"access","key_version":0}
        let token = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJHQUJDMTIzIiwiZXhwIjo5OTk5OTk5OTk5LCJ0b2tlbl90eXBlIjoiYWNjZXNzIiwia2V5X3ZlcnNpb24iOjB9.e30";
        let error = verify_jwt_token(token, TEST_SECRET).expect_err("alg none");
        assert_eq!(error, JwtVerifyError::InvalidSignature);
    }

    #[test]
    fn verify_jwt_token_strict_rejects_refresh_used_as_access() {
        let token = sign_jwt_with_type("GABC123", TEST_SECRET, 1_800_000_000, "refresh", 0)
            .expect("signed refresh");
        let error = verify_jwt_token_strict(&token, TEST_SECRET, "access", 0)
            .expect_err("refresh must not authenticate as access");
        assert_eq!(error, JwtVerifyError::WrongTokenType);
    }

    #[test]
    fn verify_jwt_token_strict_rejects_old_key_version() {
        let token = sign_jwt_with_type("GABC123", TEST_SECRET, 1_800_000_000, "access", 0)
            .expect("signed access");
        let error = verify_jwt_token_strict(&token, TEST_SECRET, "access", 1)
            .expect_err("old key version");
        assert_eq!(error, JwtVerifyError::KeyVersionMismatch);
    }

    #[test]
    fn verify_jwt_token_strict_accepts_matching_type_and_version() {
        let token = sign_jwt_with_type("GABC123", TEST_SECRET, 1_800_000_000, "refresh", 2)
            .expect("signed refresh");
        let claims = verify_jwt_token_strict(&token, TEST_SECRET, "refresh", 2).expect("ok");
        assert_eq!(claims.token_type, "refresh");
        assert_eq!(claims.key_version, 2);
    }

    #[test]
    fn refresh_tokens_live_longer_than_access_tokens() {
        let now = 1_800_000_000u64;
        let access = sign_jwt_with_type("GABC123", TEST_SECRET, now, "access", 0).unwrap();
        let refresh = sign_jwt_with_type("GABC123", TEST_SECRET, now, "refresh", 0).unwrap();
        let access_claims = verify_jwt_token(&access, TEST_SECRET).unwrap();
        let refresh_claims = verify_jwt_token(&refresh, TEST_SECRET).unwrap();
        assert_eq!(
            access_claims.exp,
            now + JWT_ACCESS_TOKEN_EXPIRY_SECS
        );
        assert_eq!(
            refresh_claims.exp,
            now + JWT_REFRESH_TOKEN_EXPIRY_SECS
        );
        assert_ne!(access_claims.jti, refresh_claims.jti);
    }

    #[test]
    fn extract_bearer_token_parses_authorization_header() {
        assert_eq!(
            extract_bearer_token("Bearer abc.def.ghi"),
            Some("abc.def.ghi")
        );
        assert_eq!(extract_bearer_token("Basic abc"), None);
        assert_eq!(extract_bearer_token("Bearer "), None);
    }

    #[test]
    fn error_display_too_short() {
        let msg = JwtFormatError::TooShort.to_string();
        assert!(msg.contains("too short"));
    }

    #[test]
    fn error_display_wrong_part_count() {
        let msg = JwtFormatError::WrongPartCount { found: 2 }.to_string();
        assert!(msg.contains('2'));
    }

    #[test]
    fn error_display_empty_part() {
        let msg = JwtFormatError::EmptyPart { index: 1 }.to_string();
        assert!(msg.contains('1'));
    }

    #[test]
    fn error_display_invalid_base64url() {
        let msg = JwtFormatError::InvalidBase64Url { index: 0 }.to_string();
        assert!(msg.contains("base64url"));
    }
}

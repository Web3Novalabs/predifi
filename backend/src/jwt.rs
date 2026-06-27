//! JWT format validation and secret-backed token verification.
//!
//! [`validate_jwt_format`] performs a lightweight structural check before any
//! cryptographic work. [`verify_jwt_secret`] validates that the configured
//! signing secret meets minimum security requirements. [`verify_jwt_token`]
//! combines both checks and verifies the token signature and claims.

use crate::constants::{JWT_MIN_LENGTH, JWT_PARTS_COUNT, JWT_SECRET_MIN_LENGTH};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

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
            Self::TooShort { min_length } => write!(
                f,
                "JWT signing secret must be at least {min_length} bytes"
            ),
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

/// Verify a JWT against the configured HMAC secret and return decoded claims.
pub fn verify_jwt_token(token: &str, secret: &str) -> Result<PredifiClaims, JwtVerifyError> {
    verify_jwt_secret(secret).map_err(JwtVerifyError::InvalidSecret)?;
    validate_jwt_format(token).map_err(JwtVerifyError::InvalidFormat)?;

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;
    validation.required_spec_claims.clear();

    let token_data = decode::<PredifiClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(map_decode_error)?;

    if token_data.claims.sub.is_empty() {
        return Err(JwtVerifyError::Decode(
            "JWT subject claim must not be empty".to_string(),
        ));
    }

    Ok(token_data.claims)
}

fn map_decode_error(error: jsonwebtoken::errors::Error) -> JwtVerifyError {
    match error.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtVerifyError::Expired,
        jsonwebtoken::errors::ErrorKind::InvalidSignature => JwtVerifyError::InvalidSignature,
        _ => JwtVerifyError::Decode(error.to_string()),
    }
}

/// Extract a bearer token from an `Authorization` header value.
pub fn extract_bearer_token(header_value: &str) -> Option<&str> {
    let mut parts = header_value.splitn(2, ' ');
    let scheme = parts.next()?;
    let token = parts.next()?;
    if scheme.eq_ignore_ascii_case("bearer") && !token.is_empty() {
        Some(token)
    } else {
        None
    }
}

#[cfg(test)]
pub fn sign_jwt_for_test(sub: &str, secret: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};

    encode(
        &Header::new(Algorithm::HS256),
        &PredifiClaims {
            sub: sub.to_string(),
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("valid test JWT")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "predifi-dev-secret-do-not-use-in-production-32";

    // A minimal but structurally valid JWT-shaped token (not a real signed token).
    const VALID_TOKEN: &str =
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

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

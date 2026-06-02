//! JWT format validation.
//!
//! Provides a lightweight structural check that a token string matches the
//! standard JWT format (`header.payload.signature`) **before** any
//! cryptographic verification is attempted.  Rejecting malformed tokens early
//! avoids unnecessary work and produces clearer error messages.
//!
//! This module intentionally does **not** verify signatures or decode claims —
//! that is the responsibility of a dedicated JWT library.

use crate::constants::{JWT_MIN_LENGTH, JWT_PARTS_COUNT};

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

/// Validate that `token` matches the structural JWT format.
///
/// Checks performed (in order):
/// 1. Minimum length (`>= JWT_MIN_LENGTH`).
/// 2. Exactly three dot-separated parts.
/// 3. No part is empty.
/// 4. Every character in each part is a valid base64url character
///    (`A-Z`, `a-z`, `0-9`, `-`, `_`).  Padding (`=`) is also accepted
///    because some implementations include it.
///
/// This function does **not** decode or verify the token — it only checks
/// the surface structure so that obviously invalid tokens are rejected
/// before any cryptographic work begins.
///
/// # Errors
///
/// Returns a [`JwtFormatError`] describing the first structural problem found.
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            validate_jwt_format("short"),
            Err(JwtFormatError::TooShort)
        );
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
        // Space is not a valid base64url character.
        let token = "eyJhbGci OiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fw";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::InvalidBase64Url { index: 0 })
        );
    }

    #[test]
    fn rejects_token_with_plus_sign() {
        // `+` is standard base64 but NOT base64url.
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fw+MeJf";
        assert_eq!(
            validate_jwt_format(token),
            Err(JwtFormatError::InvalidBase64Url { index: 2 })
        );
    }

    #[test]
    fn accepts_token_with_padding_equals() {
        // Some JWT implementations include base64 padding.
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c==";
        assert!(validate_jwt_format(token).is_ok());
    }

    #[test]
    fn accepts_token_with_url_safe_chars() {
        // `-` and `_` are the base64url replacements for `+` and `/`.
        let token = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fw-MeJf36POk6yJV_adQssw5c";
        assert!(validate_jwt_format(token).is_ok());
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

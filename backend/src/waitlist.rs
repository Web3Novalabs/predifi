//! Pre-launch waitlist sign-ups (`POST /api/v1/waitlist`).

use serde::Deserialize;
use sqlx::PgPool;

/// Longest email address allowed by RFC 5321.
const MAX_EMAIL_LEN: usize = 254;
const MAX_LOCAL_PART_LEN: usize = 64;
const MAX_NAME_LEN: usize = 100;

/// Request body for `POST /api/v1/waitlist`.
#[derive(Debug, Deserialize)]
pub struct WaitlistRequest {
    pub name: String,
    pub email: String,
}

/// Trim and lower-case an email so case variants collapse to one row.
pub fn normalize_email(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Pragmatic email check: one `@`, a non-empty local part, and a dotted domain
/// whose labels are non-empty. Deliberately not a full RFC 5322 parser.
pub fn is_valid_email(email: &str) -> bool {
    if email.is_empty() || email.len() > MAX_EMAIL_LEN {
        return false;
    }
    if email.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    if local.is_empty() || local.len() > MAX_LOCAL_PART_LEN || domain.contains('@') {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    domain.split('.').all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    })
}

/// A name is valid when it is non-blank and at most [`MAX_NAME_LEN`] characters.
pub fn is_valid_name(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty() && n.chars().count() <= MAX_NAME_LEN && !n.chars().any(|c| c.is_control())
}

/// Insert a sign-up. A repeat of an existing email is a no-op, so the caller
/// cannot tell a new address from one already on the list.
pub async fn add_to_waitlist(pool: &PgPool, name: &str, email: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO waitlist (name, email)
        VALUES ($1, $2)
        ON CONFLICT (email) DO NOTHING
        "#,
    )
    .bind(name)
    .bind(email)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_case_and_whitespace() {
        assert_eq!(normalize_email("  Ada@Example.COM "), "ada@example.com");
    }

    #[test]
    fn accepts_ordinary_emails() {
        for e in [
            "a@b.co",
            "first.last+tag@sub.example.org",
            "x_y@ex-ample.io",
        ] {
            assert!(is_valid_email(e), "{e} should be valid");
        }
    }

    #[test]
    fn rejects_malformed_emails() {
        let too_long = format!("{}@example.com", "a".repeat(250));
        for e in [
            "",
            "plain",
            "@example.com",
            "a@",
            "a@b",
            "a@@b.com",
            "a b@c.com",
            "a@b..com",
            "a@.com",
            "a@b.com.",
            "a@-b.com",
            "a@b_c.com",
            too_long.as_str(),
        ] {
            assert!(!is_valid_email(e), "{e:?} should be invalid");
        }
    }

    #[test]
    fn validates_names() {
        assert!(is_valid_name("Ada Lovelace"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("   "));
        assert!(!is_valid_name(&"x".repeat(101)));
        assert!(!is_valid_name("bad\u{0}name"));
    }
}

use std::{collections::HashMap, env, num::ParseIntError};
use predifi_errors::ConfigError;

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 3000;
const DEFAULT_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/predifi";
const DEFAULT_DB_MAX_CONNECTIONS: u32 = 10;
const DEFAULT_DB_MIN_CONNECTIONS: u32 = 1;
const DEFAULT_DB_ACQUIRE_TIMEOUT_SECS: u64 = 30;
const DEFAULT_RPC_HEALTH_TIMEOUT_SECS: u64 = 2;
const DEFAULT_RPC_HEALTH_RETRY_COUNT: u8 = 3;
const DEFAULT_LOG_LEVEL: &str = "info";
const DEFAULT_STELLAR_RPC_URL: &str = "https://soroban-testnet.stellar.org";
const DEFAULT_TREASURY_FEE_BPS: u32 = 300;
const DEFAULT_REFERRAL_FEE_BPS: u32 = 5000;
const DEFAULT_REDIS_URL: &str = "redis://localhost:6379";

/// Origins allowed by default when `CORS_ALLOWED_ORIGINS` is not set.
pub const DEFAULT_CORS_ORIGINS: &[&str] = &[
    "http://localhost:3000",
    "http://localhost:5173",
    "https://predifi.app",
];

/// Runtime configuration loaded from environment variables.
///
/// All fields have compiled-in defaults so the server can start without any
/// environment variables set (useful for local development and unit tests).
/// Call [`Config::from_env`] at startup to populate the struct from the
/// process environment, or [`Config::from_map`] in tests to supply values
/// directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// IP address the HTTP server binds to (default `0.0.0.0`).
    pub host: String,
    /// TCP port the HTTP server listens on (default `3000`).
    pub port: u16,
    /// PostgreSQL connection string (default points to a local `predifi` DB).
    pub database_url: String,
    /// Maximum number of connections in the SQLx pool (default `10`).
    pub db_max_connections: u32,
    /// Minimum number of idle connections kept alive in the pool (default `1`).
    pub db_min_connections: u32,
    /// Seconds to wait when acquiring a connection from the pool (default `30`).
    pub db_acquire_timeout_secs: u64,
    /// Per-attempt timeout in seconds for the Stellar RPC health check (default `2`).
    pub rpc_health_timeout_secs: u64,
    /// Number of times to retry the Stellar RPC health check before reporting failure (default `3`).
    pub rpc_health_retry_count: u8,
    /// Tracing log level passed to `RUST_LOG` / `EnvFilter` (default `"info"`).
    pub log_level: String,
    /// Protocol treasury fee in basis points (default `300` = 3 %).
    pub treasury_fee_bps: u32,
    /// Referral share of the protocol fee in basis points (default `5000` = 50 % of the fee).
    pub referral_fee_bps: u32,
    /// Stellar Soroban RPC endpoint URL (default: testnet).
    pub stellar_rpc_url: String,
    /// Optional Sentry DSN for error reporting. `None` disables Sentry.
    pub sentry_dsn: Option<String>,
    /// Redis connection URL (default `redis://localhost:6379`).
    pub redis_url: String,
    /// Validated list of origins permitted by the CORS policy.
    ///
    /// Loaded from the `PREDIFI_CORS_ALLOWED_ORIGINS` environment variable as a
    /// comma-separated list of origins (e.g. `https://app.example.com,https://admin.example.com`).
    /// Each entry must be a valid `http://` or `https://` origin — scheme + host (+ optional port)
    /// with no path, query string, or fragment.  Invalid entries are rejected at startup.
    ///
    /// Falls back to [`DEFAULT_CORS_ORIGINS`] when the variable is absent.
    pub cors_allowed_origins: Vec<String>,
}

impl Config {
    /// Load all runtime settings from environment variables, with defaults.
    pub fn from_env() -> Result<Self, ConfigError> {
        let vars: HashMap<String, String> = env::vars().collect();
        Self::from_map(&vars)
    }

    fn from_map(vars: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let host = get_string(vars, "PREDIFI_APP_HOST", DEFAULT_HOST);
        let port = get_u16(vars, "PREDIFI_APP_PORT", DEFAULT_PORT)?;
        let database_url = get_string(vars, "PREDIFI_DATABASE_URL", DEFAULT_DATABASE_URL);
        let db_max_connections = get_u32(vars, "PREDIFI_DB_MAX_CONNECTIONS", DEFAULT_DB_MAX_CONNECTIONS)?;
        let db_min_connections = get_u32(vars, "PREDIFI_DB_MIN_CONNECTIONS", DEFAULT_DB_MIN_CONNECTIONS)?;
        let db_acquire_timeout_secs = get_u64(
            vars,
            "PREDIFI_DB_ACQUIRE_TIMEOUT_SECS",
            DEFAULT_DB_ACQUIRE_TIMEOUT_SECS,
        )?;
        let rpc_health_timeout_secs = get_u64(
            vars,
            "PREDIFI_RPC_HEALTH_TIMEOUT_SECS",
            DEFAULT_RPC_HEALTH_TIMEOUT_SECS,
        )?;
        let rpc_health_retry_count = get_u8(
            vars,
            "PREDIFI_RPC_HEALTH_RETRY_COUNT",
            DEFAULT_RPC_HEALTH_RETRY_COUNT,
        )?;
        let log_level = get_string(vars, "RUST_LOG", DEFAULT_LOG_LEVEL);
        let treasury_fee_bps = get_u32(vars, "PREDIFI_TREASURY_FEE_BPS", DEFAULT_TREASURY_FEE_BPS)?;
        let referral_fee_bps = get_u32(vars, "PREDIFI_REFERRAL_FEE_BPS", DEFAULT_REFERRAL_FEE_BPS)?;
        let stellar_rpc_url = get_string(vars, "PREDIFI_STELLAR_RPC_URL", DEFAULT_STELLAR_RPC_URL);
        let sentry_dsn = vars.get("PREDIFI_SENTRY_DSN").cloned();
        let redis_url = get_string(vars, "PREDIFI_REDIS_URL", DEFAULT_REDIS_URL);

        // Parse and strictly validate CORS origins.
        let cors_allowed_origins = parse_cors_origins(vars)?;

        if db_min_connections > db_max_connections {
            return Err(ConfigError::InvalidValue {
                key: "PREDIFI_DB_MIN_CONNECTIONS",
                reason: format!(
                    "must be <= PREDIFI_DB_MAX_CONNECTIONS ({}), got {}",
                    db_max_connections, db_min_connections
                ),
            });
        }

        Ok(Self {
            host,
            port,
            database_url,
            db_max_connections,
            db_min_connections,
            db_acquire_timeout_secs,
            rpc_health_timeout_secs,
            rpc_health_retry_count,
            log_level,
            treasury_fee_bps,
            referral_fee_bps,
            stellar_rpc_url,
            sentry_dsn,
            redis_url,
            cors_allowed_origins,
        })
    }

    /// Return the `host:port` string used to bind the TCP listener.
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Build a minimal [`Config`] suitable for unit tests.
    ///
    /// Uses `127.0.0.1:0` (OS-assigned port), an in-memory-style Postgres URL,
    /// and the compiled-in fee defaults so tests do not depend on environment
    /// variables.
    #[cfg(test)]
    pub fn default_for_test() -> Self {
        Self {
            host: String::from("127.0.0.1"),
            port: 0,
            database_url: String::from("postgres://localhost/test"),
            db_max_connections: 1,
            db_min_connections: 1,
            db_acquire_timeout_secs: 1,
            rpc_health_timeout_secs: 2,
            rpc_health_retry_count: 3,
            log_level: String::from("debug"),
            treasury_fee_bps: DEFAULT_TREASURY_FEE_BPS,
            referral_fee_bps: DEFAULT_REFERRAL_FEE_BPS,
            stellar_rpc_url: String::from(DEFAULT_STELLAR_RPC_URL),
            sentry_dsn: None,
            redis_url: String::from(DEFAULT_REDIS_URL),
            cors_allowed_origins: DEFAULT_CORS_ORIGINS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

// ConfigError moved to the shared `predifi-errors` crate and re-exported when
// that crate is used with the `std` feature. See `contracts/predifi-errors`.
/// Error returned when a configuration value cannot be parsed or is logically invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// An environment variable was set but its value could not be parsed as the
    /// expected numeric type.
    InvalidNumber {
        /// Name of the environment variable.
        key: &'static str,
        /// The raw string value that failed to parse.
        value: String,
        /// Human-readable parse error from the standard library.
        reason: String,
    },
    /// An environment variable was set to a syntactically valid string but the
    /// value violates a semantic constraint (e.g. min > max, invalid URL).
    InvalidValue {
        /// Name of the environment variable.
        key: &'static str,
        /// Human-readable description of the constraint that was violated.
        reason: String,
    },
    /// A required environment variable is absent and has no compiled-in default.
    MissingRequired {
        /// Name of the missing environment variable.
        key: &'static str,
        /// Human-readable description of what the variable is used for.
        description: &'static str,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber { key, value, reason } => {
                write!(f, "invalid value for {}='{}': {}", key, value, reason)
            }
            Self::InvalidValue { key, reason } => {
                write!(f, "invalid value for {}: {}", key, reason)
            }
            Self::MissingRequired { key, description } => {
                write!(
                    f,
                    "required environment variable '{}' is not set ({})",
                    key, description
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Parse and strictly validate the `PREDIFI_CORS_ALLOWED_ORIGINS` environment variable.
///
/// The value must be a comma-separated list of origins.  Each origin must:
/// - Use the `http` or `https` scheme.
/// - Contain a non-empty host.
/// - Have **no** path component other than an optional trailing slash on the root.
/// - Have **no** query string or fragment.
///
/// Returns the default origins when the variable is absent.
fn parse_cors_origins(vars: &HashMap<String, String>) -> Result<Vec<String>, ConfigError> {
    let raw = match vars.get("PREDIFI_CORS_ALLOWED_ORIGINS") {
        Some(v) => v.clone(),
        None => {
            return Ok(DEFAULT_CORS_ORIGINS
                .iter()
                .map(|s| s.to_string())
                .collect());
        }
    };

    let origins: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if origins.is_empty() {
        return Err(ConfigError::InvalidValue {
            key: "PREDIFI_CORS_ALLOWED_ORIGINS",
            reason: String::from("must contain at least one origin"),
        });
    }

    for origin in &origins {
        validate_cors_origin(origin)?;
    }

    Ok(origins)
}

/// Validate a single CORS origin string.
///
/// A valid origin is `scheme://host` or `scheme://host:port` where:
/// - `scheme` is `http` or `https`
/// - `host` is non-empty
/// - There is no path (other than an empty path or bare `/`), no query, and no fragment.
fn validate_cors_origin(origin: &str) -> Result<(), ConfigError> {
    // Reject wildcards and the special `null` origin outright.
    if origin == "*" || origin.eq_ignore_ascii_case("null") {
        return Err(ConfigError::InvalidValue {
            key: "PREDIFI_CORS_ALLOWED_ORIGINS",
            reason: format!("'{}' is not a valid origin — wildcards and 'null' are not permitted", origin),
        });
    }

    // Split off the scheme.
    let rest = if let Some(r) = origin.strip_prefix("https://") {
        r
    } else if let Some(r) = origin.strip_prefix("http://") {
        r
    } else {
        return Err(ConfigError::InvalidValue {
            key: "PREDIFI_CORS_ALLOWED_ORIGINS",
            reason: format!(
                "'{}' is not a valid origin — must start with 'http://' or 'https://'",
                origin
            ),
        });
    };

    // Reject fragments and query strings.
    if rest.contains('#') || rest.contains('?') {
        return Err(ConfigError::InvalidValue {
            key: "PREDIFI_CORS_ALLOWED_ORIGINS",
            reason: format!(
                "'{}' is not a valid origin — must not contain a query string or fragment",
                origin
            ),
        });
    }

    // Split host[:port] from any path component.
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, ""),
    };

    // A path other than "" or "/" is not allowed in an origin.
    if !path.is_empty() && path != "/" {
        return Err(ConfigError::InvalidValue {
            key: "PREDIFI_CORS_ALLOWED_ORIGINS",
            reason: format!(
                "'{}' is not a valid origin — must not contain a path component",
                origin
            ),
        });
    }

    // The host part must be non-empty.
    let host = match authority.rfind(':') {
        // Possible port — strip it and validate.
        Some(colon_idx) => {
            let port_str = &authority[colon_idx + 1..];
            // Only treat it as a port if it's all digits; otherwise it's part of an IPv6 address.
            if port_str.chars().all(|c| c.is_ascii_digit()) {
                let port: u32 = port_str.parse().unwrap_or(u32::MAX);
                if port > 65535 {
                    return Err(ConfigError::InvalidValue {
                        key: "PREDIFI_CORS_ALLOWED_ORIGINS",
                        reason: format!(
                            "'{}' is not a valid origin — port {} is out of range (0-65535)",
                            origin, port_str
                        ),
                    });
                }
                &authority[..colon_idx]
            } else {
                authority
            }
        }
        None => authority,
    };

    if host.is_empty() {
        return Err(ConfigError::InvalidValue {
            key: "PREDIFI_CORS_ALLOWED_ORIGINS",
            reason: format!("'{}' is not a valid origin — host must not be empty", origin),
        });
    }

    Ok(())
}

fn get_string(vars: &HashMap<String, String>, key: &'static str, default: &str) -> String {
    vars.get(key)
        .map_or_else(|| default.to_string(), |value| value.clone())
}

fn get_u16(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u16,
) -> Result<u16, ConfigError> {
    match vars.get(key) {
        Some(value) => value
            .parse::<u16>()
            .map_err(|err| to_number_error(key, value, err)),
        None => Ok(default),
    }
}

fn get_u32(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    match vars.get(key) {
        Some(value) => value
            .parse::<u32>()
            .map_err(|err| to_number_error(key, value, err)),
        None => Ok(default),
    }
}

fn get_u8(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u8,
) -> Result<u8, ConfigError> {
    match vars.get(key) {
        Some(value) => value
            .parse::<u8>()
            .map_err(|err| to_number_error(key, value, err)),
        None => Ok(default),
    }
}

fn get_u64(
    vars: &HashMap<String, String>,
    key: &'static str,
    default: u64,
) -> Result<u64, ConfigError> {
    match vars.get(key) {
        Some(value) => value
            .parse::<u64>()
            .map_err(|err| to_number_error(key, value, err)),
        None => Ok(default),
    }
}

fn to_number_error(key: &'static str, value: &str, err: ParseIntError) -> ConfigError {
    ConfigError::InvalidNumber {
        key,
        value: value.to_string(),
        reason: err.to_string(),
    }
}

/// Return the value of `key` from `vars`, or `Err(ConfigError::MissingRequired)` if absent.
///
/// Use this for environment variables that **must** be explicitly set in
/// production deployments (i.e. variables with no safe compiled-in default).
#[allow(dead_code)]
fn get_required_string(
    vars: &HashMap<String, String>,
    key: &'static str,
    description: &'static str,
) -> Result<String, ConfigError> {
    vars.get(key)
        .cloned()
        .ok_or(ConfigError::MissingRequired { key, description })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_uses_defaults_when_env_is_missing() {
        let vars = HashMap::new();
        let config = Config::from_map(&vars).expect("defaults should build a valid config");

        assert_eq!(config.host, DEFAULT_HOST);
        assert_eq!(config.port, DEFAULT_PORT);
        assert_eq!(config.database_url, DEFAULT_DATABASE_URL);
        assert_eq!(config.db_max_connections, DEFAULT_DB_MAX_CONNECTIONS);
        assert_eq!(config.db_min_connections, DEFAULT_DB_MIN_CONNECTIONS);
        assert_eq!(
            config.db_acquire_timeout_secs,
            DEFAULT_DB_ACQUIRE_TIMEOUT_SECS
        );
        assert_eq!(config.log_level, DEFAULT_LOG_LEVEL);
        assert_eq!(config.treasury_fee_bps, DEFAULT_TREASURY_FEE_BPS);
        assert_eq!(config.referral_fee_bps, DEFAULT_REFERRAL_FEE_BPS);
        assert_eq!(config.redis_url, DEFAULT_REDIS_URL);
    }

    #[test]
    fn config_rejects_non_numeric_port() {
        let vars = HashMap::from([(String::from("PREDIFI_APP_PORT"), String::from("not-a-number"))]);
        let error = Config::from_map(&vars).expect_err("port must be numeric");

        assert!(
            matches!(
                error,
                ConfigError::InvalidNumber {
                    key: "PREDIFI_APP_PORT",
                    ..
                }
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn config_rejects_min_connections_larger_than_max() {
        let vars = HashMap::from([
            (String::from("PREDIFI_DB_MIN_CONNECTIONS"), String::from("20")),
            (String::from("PREDIFI_DB_MAX_CONNECTIONS"), String::from("10")),
        ]);
        let error = Config::from_map(&vars).expect_err("min > max must be rejected");

        assert!(
            matches!(
                error,
                ConfigError::InvalidValue {
                    key: "PREDIFI_DB_MIN_CONNECTIONS",
                    ..
                }
            ),
            "unexpected error: {error}"
        );
    }

    // ── CORS origin validation ────────────────────────────────────────────────

    #[test]
    fn cors_defaults_when_env_absent() {
        let vars = HashMap::new();
        let config = Config::from_map(&vars).expect("defaults should be valid");
        let expected: Vec<String> = DEFAULT_CORS_ORIGINS.iter().map(|s| s.to_string()).collect();
        assert_eq!(config.cors_allowed_origins, expected);
    }

    #[test]
    fn cors_accepts_valid_https_origin() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("https://example.com"),
        )]);
        let config = Config::from_map(&vars).expect("valid https origin should be accepted");
        assert_eq!(config.cors_allowed_origins, vec!["https://example.com"]);
    }

    #[test]
    fn cors_accepts_valid_http_origin_with_port() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("http://localhost:5173"),
        )]);
        let config = Config::from_map(&vars).expect("http origin with port should be accepted");
        assert_eq!(config.cors_allowed_origins, vec!["http://localhost:5173"]);
    }

    #[test]
    fn cors_accepts_multiple_valid_origins() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("https://app.example.com,https://admin.example.com"),
        )]);
        let config = Config::from_map(&vars).expect("multiple valid origins should be accepted");
        assert_eq!(
            config.cors_allowed_origins,
            vec!["https://app.example.com", "https://admin.example.com"]
        );
    }

    #[test]
    fn cors_trims_whitespace_around_origins() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("  https://app.example.com , https://admin.example.com  "),
        )]);
        let config = Config::from_map(&vars).expect("whitespace should be trimmed");
        assert_eq!(
            config.cors_allowed_origins,
            vec!["https://app.example.com", "https://admin.example.com"]
        );
    }

    #[test]
    fn cors_rejects_wildcard() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("*"),
        )]);
        let error = Config::from_map(&vars).expect_err("wildcard must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_null_origin() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("null"),
        )]);
        let error = Config::from_map(&vars).expect_err("null origin must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_missing_scheme() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("example.com"),
        )]);
        let error = Config::from_map(&vars).expect_err("origin without scheme must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_ftp_scheme() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("ftp://example.com"),
        )]);
        let error = Config::from_map(&vars).expect_err("ftp scheme must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_origin_with_path() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("https://example.com/api"),
        )]);
        let error = Config::from_map(&vars).expect_err("origin with path must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_origin_with_query_string() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("https://example.com?foo=bar"),
        )]);
        let error = Config::from_map(&vars).expect_err("origin with query string must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_origin_with_fragment() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("https://example.com#section"),
        )]);
        let error = Config::from_map(&vars).expect_err("origin with fragment must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_port_out_of_range() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("https://example.com:99999"),
        )]);
        let error = Config::from_map(&vars).expect_err("port > 65535 must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn cors_rejects_empty_list() {
        let vars = HashMap::from([(
            String::from("PREDIFI_CORS_ALLOWED_ORIGINS"),
            String::from("   "),
        )]);
        let error = Config::from_map(&vars).expect_err("empty origin list must be rejected");
        assert!(
            matches!(error, ConfigError::InvalidValue { key: "PREDIFI_CORS_ALLOWED_ORIGINS", .. }),
            "unexpected error: {error}"
        );
    }

    // ── Specific error codes for config (issue #976) ──────────────────────────

    /// `get_required_string` returns `MissingRequired` when the variable is absent.
    #[test]
    fn missing_required_error_when_variable_absent() {
        let vars = HashMap::new();
        let error = get_required_string(&vars, "PREDIFI_SECRET_KEY", "JWT signing secret")
            .expect_err("absent required variable must produce an error");

        assert!(
            matches!(
                error,
                ConfigError::MissingRequired {
                    key: "PREDIFI_SECRET_KEY",
                    ..
                }
            ),
            "unexpected error: {error}"
        );
    }

    /// `get_required_string` succeeds when the variable is present.
    #[test]
    fn required_string_succeeds_when_variable_present() {
        let vars = HashMap::from([(
            String::from("PREDIFI_SECRET_KEY"),
            String::from("supersecret"),
        )]);
        let value = get_required_string(&vars, "PREDIFI_SECRET_KEY", "JWT signing secret")
            .expect("present required variable must succeed");
        assert_eq!(value, "supersecret");
    }

    /// `MissingRequired` display message includes the variable name and description.
    #[test]
    fn missing_required_display_includes_key_and_description() {
        let error = ConfigError::MissingRequired {
            key: "PREDIFI_SECRET_KEY",
            description: "JWT signing secret",
        };
        let msg = error.to_string();
        assert!(
            msg.contains("PREDIFI_SECRET_KEY"),
            "error message must contain the variable name, got: {msg}"
        );
        assert!(
            msg.contains("JWT signing secret"),
            "error message must contain the description, got: {msg}"
        );
    }

    /// `InvalidNumber` display message includes the variable name and bad value.
    #[test]
    fn invalid_number_display_includes_key_and_value() {
        let vars = HashMap::from([(
            String::from("PREDIFI_APP_PORT"),
            String::from("not-a-port"),
        )]);
        let error = Config::from_map(&vars).expect_err("non-numeric port must fail");
        let msg = error.to_string();
        assert!(
            msg.contains("PREDIFI_APP_PORT"),
            "error message must contain the variable name, got: {msg}"
        );
        assert!(
            msg.contains("not-a-port"),
            "error message must contain the bad value, got: {msg}"
        );
    }

    /// `InvalidValue` display message includes the variable name and reason.
    #[test]
    fn invalid_value_display_includes_key_and_reason() {
        let vars = HashMap::from([
            (String::from("PREDIFI_DB_MIN_CONNECTIONS"), String::from("50")),
            (String::from("PREDIFI_DB_MAX_CONNECTIONS"), String::from("10")),
        ]);
        let error = Config::from_map(&vars).expect_err("min > max must fail");
        let msg = error.to_string();
        assert!(
            msg.contains("PREDIFI_DB_MIN_CONNECTIONS"),
            "error message must contain the variable name, got: {msg}"
        );
    }
}

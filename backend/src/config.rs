use std::{collections::HashMap, env, fmt, num::ParseIntError};

const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 3000;
const DEFAULT_DATABASE_URL: &str = "postgres://postgres:postgres@localhost:5432/predifi";
const DEFAULT_DB_MAX_CONNECTIONS: u32 = 10;
const DEFAULT_DB_MIN_CONNECTIONS: u32 = 1;
const DEFAULT_DB_ACQUIRE_TIMEOUT_SECS: u64 = 30;
const DEFAULT_LOG_LEVEL: &str = "info";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub db_max_connections: u32,
    pub db_min_connections: u32,
    pub db_acquire_timeout_secs: u64,
    pub log_level: String,
}

impl Config {
    /// Load all runtime settings from environment variables, with defaults.
    pub fn from_env() -> Result<Self, ConfigError> {
        let vars: HashMap<String, String> = env::vars().collect();
        Self::from_map(&vars)
    }

    fn from_map(vars: &HashMap<String, String>) -> Result<Self, ConfigError> {
        let host = get_string(vars, "APP_HOST", DEFAULT_HOST);
        let port = get_u16(vars, "APP_PORT", DEFAULT_PORT)?;
        let database_url = get_string(vars, "DATABASE_URL", DEFAULT_DATABASE_URL);
        let db_max_connections = get_u32(vars, "DB_MAX_CONNECTIONS", DEFAULT_DB_MAX_CONNECTIONS)?;
        let db_min_connections = get_u32(vars, "DB_MIN_CONNECTIONS", DEFAULT_DB_MIN_CONNECTIONS)?;
        let db_acquire_timeout_secs = get_u64(
            vars,
            "DB_ACQUIRE_TIMEOUT_SECS",
            DEFAULT_DB_ACQUIRE_TIMEOUT_SECS,
        )?;
        let log_level = get_string(vars, "RUST_LOG", DEFAULT_LOG_LEVEL);

        if db_min_connections > db_max_connections {
            return Err(ConfigError::InvalidValue {
                key: "DB_MIN_CONNECTIONS",
                reason: format!(
                    "must be <= DB_MAX_CONNECTIONS ({}), got {}",
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
            log_level,
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    InvalidNumber {
        key: &'static str,
        value: String,
        reason: String,
    },
    InvalidValue {
        key: &'static str,
        reason: String,
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
        }
    }
}

impl std::error::Error for ConfigError {}

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
    }

    #[test]
    fn config_rejects_non_numeric_port() {
        let vars = HashMap::from([(String::from("APP_PORT"), String::from("not-a-number"))]);
        let error = Config::from_map(&vars).expect_err("port must be numeric");

        assert!(
            matches!(
                error,
                ConfigError::InvalidNumber {
                    key: "APP_PORT",
                    ..
                }
            ),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn config_rejects_min_connections_larger_than_max() {
        let vars = HashMap::from([
            (String::from("DB_MIN_CONNECTIONS"), String::from("20")),
            (String::from("DB_MAX_CONNECTIONS"), String::from("10")),
        ]);
        let error = Config::from_map(&vars).expect_err("min > max must be rejected");

        assert!(
            matches!(
                error,
                ConfigError::InvalidValue {
                    key: "DB_MIN_CONNECTIONS",
                    ..
                }
            ),
            "unexpected error: {error}"
        );
    }
}

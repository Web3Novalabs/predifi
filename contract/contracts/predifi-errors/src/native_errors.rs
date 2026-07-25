//! Native error types re-exported for backend use.
//!
//! These types are only compiled when the `std` feature is enabled on the
//! `predifi-errors` crate so that the contract build remains `no_std`-friendly.

use std::fmt;

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

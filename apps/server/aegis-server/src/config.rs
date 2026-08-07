//! Server configuration loaded from environment variables at startup.

use std::net::SocketAddr;
use std::time::Duration;

use thiserror::Error;

/// Server configuration loaded from environment variables.
///
/// Constructed via [`Config::from_env`] in `main.rs`. Every field is
/// `pub`; the binary does no further wrapping.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub signing_key: Vec<u8>,
    pub bind_addr: SocketAddr,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Failure modes of [`Config::from_env`].
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingEnvVariable(&'static str),

    #[error("invalid value for environment variable {var}: {message}")]
    InvalidValue {
        var: &'static str,
        message: String,
    },
}

impl Config {
    /// Read every required variable from `std::env`. Returns
    /// [`ConfigError::MissingEnvVariable`] if a required variable is
    /// not set, or [`ConfigError::InvalidValue`] if a value cannot be
    /// parsed.
    pub fn from_env() -> Result<Self, ConfigError> {
        todo!("implemented in Task 3")
    }
}
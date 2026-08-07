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
        let database_url = std::env::var("AEGIS_DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnvVariable("AEGIS_DATABASE_URL"))?;

        let signing_key_hex = std::env::var("AEGIS_AUTH_SIGNING_KEY")
            .map_err(|_| ConfigError::MissingEnvVariable("AEGIS_AUTH_SIGNING_KEY"))?;
        let signing_key = hex_decode(&signing_key_hex).map_err(|message| {
            ConfigError::InvalidValue {
                var: "AEGIS_AUTH_SIGNING_KEY",
                message,
            }
        })?;
        if signing_key.len() < 32 {
            return Err(ConfigError::InvalidValue {
                var: "AEGIS_AUTH_SIGNING_KEY",
                message: format!("got {} bytes, need >= 32", signing_key.len()),
            });
        }

        let bind_addr_str = std::env::var("AEGIS_HTTP_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let bind_addr: SocketAddr = bind_addr_str.parse().map_err(|e: std::net::AddrParseError| {
            ConfigError::InvalidValue {
                var: "AEGIS_HTTP_BIND",
                message: e.to_string(),
            }
        })?;

        let access_ttl_secs = match std::env::var("AEGIS_ACCESS_TTL_SECS") {
            Ok(s) => s.parse::<u64>().map_err(|e| ConfigError::InvalidValue {
                var: "AEGIS_ACCESS_TTL_SECS",
                message: e.to_string(),
            })?,
            Err(_) => 900,
        };
        let refresh_ttl_secs = match std::env::var("AEGIS_REFRESH_TTL_SECS") {
            Ok(s) => s.parse::<u64>().map_err(|e| ConfigError::InvalidValue {
                var: "AEGIS_REFRESH_TTL_SECS",
                message: e.to_string(),
            })?,
            Err(_) => 7 * 24 * 60 * 60,
        };

        Ok(Self {
            database_url,
            signing_key,
            bind_addr,
            access_ttl: Duration::from_secs(access_ttl_secs),
            refresh_ttl: Duration::from_secs(refresh_ttl_secs),
        })
    }
}

/// Decode a hex string (lowercase or uppercase) into bytes. Rejects
/// odd-length input and any non-hex character.
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("hex string has odd length".into());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = hex_nibble(chunk[0]).ok_or_else(|| "non-hex character".to_string())?;
        let lo = hex_nibble(chunk[1]).ok_or_else(|| "non-hex character".to_string())?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set up an env-var-only test by serializing access to the
    /// process-global env. The mutex serializes parallel test
    /// threads; each test installs its vars under its own block so
    /// they are restored at the end.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Helper: set an env var, returning a guard that restores the
    /// previous value (or unsets if it was unset) on drop. Lets each
    /// test run inside a `let _g = set_env(...);` block.
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: env vars are process-global state; serializing
            // access via ENV_LOCK makes the mutation safe in tests.
            unsafe {
                match &self.prev {
                    Some(v) => std::env::set_var(self.key, v),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }
    fn set_env(key: &'static str, value: &str) -> EnvGuard {
        let prev = std::env::var(key).ok();
        // SAFETY: serialized via ENV_LOCK in tests.
        unsafe { std::env::set_var(key, value); }
        EnvGuard { key, prev }
    }

    /// Hex-encode a fixed 32-byte key for tests.
    fn sample_key_hex() -> String {
        (0..32u8).map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn from_env_succeeds_with_all_required_vars() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        let _b = set_env("AEGIS_HTTP_BIND", "127.0.0.1:9090");
        let _a = set_env("AEGIS_ACCESS_TTL_SECS", "60");
        let _r = set_env("AEGIS_REFRESH_TTL_SECS", "120");

        let cfg = Config::from_env().expect("config should parse");
        assert_eq!(cfg.database_url, "postgres://localhost/x");
        assert_eq!(cfg.signing_key.len(), 32);
        assert_eq!(cfg.bind_addr.to_string(), "127.0.0.1:9090");
        assert_eq!(cfg.access_ttl, std::time::Duration::from_secs(60));
        assert_eq!(cfg.refresh_ttl, std::time::Duration::from_secs(120));
    }

    #[test]
    fn from_env_uses_defaults_when_optional_vars_missing() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        // AEGIS_HTTP_BIND, AEGIS_ACCESS_TTL_SECS, AEGIS_REFRESH_TTL_SECS all unset.

        let cfg = Config::from_env().expect("config should parse");
        assert_eq!(cfg.bind_addr.to_string(), "0.0.0.0:8080");
        assert_eq!(cfg.access_ttl, std::time::Duration::from_secs(900));
        assert_eq!(cfg.refresh_ttl, std::time::Duration::from_secs(7 * 24 * 60 * 60));
    }

    #[test]
    fn from_env_errors_when_database_url_missing() {
        let _g = lock_env();
        // SAFETY: serialized via ENV_LOCK in tests.
        unsafe { std::env::remove_var("AEGIS_DATABASE_URL"); }
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::MissingEnvVariable("AEGIS_DATABASE_URL")));
    }

    #[test]
    fn from_env_errors_when_signing_key_missing() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        // SAFETY: serialized via ENV_LOCK in tests.
        unsafe { std::env::remove_var("AEGIS_AUTH_SIGNING_KEY"); }

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::MissingEnvVariable("AEGIS_AUTH_SIGNING_KEY")));
    }

    #[test]
    fn from_env_errors_on_short_signing_key() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        // 16 bytes -> < 32.
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &"00".repeat(16));

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_AUTH_SIGNING_KEY", .. }));
    }

    #[test]
    fn from_env_errors_on_invalid_hex_signing_key() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", "not-hex-bytes");

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_AUTH_SIGNING_KEY", .. }));
    }

    #[test]
    fn from_env_errors_on_invalid_bind_addr() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        let _b = set_env("AEGIS_HTTP_BIND", "not-an-addr");

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_HTTP_BIND", .. }));
    }

    #[test]
    fn from_env_errors_on_non_numeric_ttl() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        let _a = set_env("AEGIS_ACCESS_TTL_SECS", "not-a-number");

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_ACCESS_TTL_SECS", .. }));
    }
}
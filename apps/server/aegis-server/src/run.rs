//! Server bootstrap.
//!
//! `run` is the single entry point called by `main.rs`. It:
//! 1. Loads [`Config`] from the environment.
//! 2. Builds a Postgres pool + in-memory token-version cache.
//! 3. Wires `AuthUsecase` → `AuthServiceImpl` → `Arc<dyn AuthService>`.
//! 4. Binds the HTTP listener and serves the router returned by
//!    [`transport::http::router`].

use std::sync::Arc;
use std::time::Duration;

use apis::auth::AuthService;
use apis::user::UserService;
use auth::{
    AuthServiceImpl, AuthUsecase, AuthUsecaseConfig, DomainIdentityRepo, InMemoryTokenVersionCache,
    TokenVersionCache, UserCredentialsRepo,
};
use auth::{UserService as AuthUserService, UserServiceImpl as AuthUserServiceImpl};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use user::{UserRepo, UserServiceImpl, UserUsecase};

use crate::config::Config;
use crate::state::AppState;
use crate::transport;

/// Bootstrap the aegis-server HTTP service and run until interrupted.
///
/// `config` is normally built via [`Config::from_env`] in `main.rs`.
/// The function only returns on bind / serve failure or shutdown.
pub async fn run(config: Config) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _log_guard = init_tracing();

    let pool = build_pool(&config.database_url).await?;
    let cache: Arc<dyn TokenVersionCache> = Arc::new(InMemoryTokenVersionCache::new());

    let auth = build_auth_service(&config, pool.clone(), cache)?;
    let user = build_user_service(pool);

    let state = AppState {
        auth: auth as Arc<dyn AuthService>,
        user: user as Arc<dyn UserService>,
    };
    let app = transport::http::router(state);

    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "aegis-server listening");

    let shutdown = shutdown_signal();

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
    Ok(())
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM and then resolve.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Build the global `EnvFilter` from `AEGIS_LOG_LEVEL`. Defaults to
/// `info` when the variable is unset. The previous `RUST_LOG` escape
/// hatch is intentionally dropped — `AEGIS_LOG_LEVEL` is the only
/// knob documented in `.env`.
fn build_env_filter() -> EnvFilter {
    let level = std::env::var("AEGIS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    EnvFilter::new(level)
}

/// Initialise tracing. Writes JSON-formatted events to a daily
/// rotating file under `$AEGIS_LOG_DIR` (defaults to `./logs` if
/// unset, which only happens in tests). Returns the
/// `WorkerGuard` for the non-blocking writer — it MUST be held for
/// the lifetime of the program or the buffered writes are lost on
/// shutdown. The returned guard is `let _guard = …;` in [`run`].
///
/// `try_init` swallows the "already initialized" error, so calling
/// `init_tracing` more than once is a no-op (not a panic).
fn init_tracing() -> tracing_appender::non_blocking::WorkerGuard {
    let dir = std::env::var("AEGIS_LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    let filter = build_env_filter();

    // Daily rotation produces files named
    // `{prefix}.YYYY-MM-DD` (e.g. `aegis-server.log.2026-08-09`).
    let file_appender = tracing_appender::rolling::daily(&dir, "aegis-server.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(non_blocking)
        .try_init();

    guard
}

/// Build a Postgres connection pool with a sensible timeout /
/// size policy.
async fn build_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(16)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Wrap the auth backend (usecase + cache + repos) in an
/// `AuthServiceImpl` so the HTTP layer can consume it through
/// `Arc<dyn AuthService>`.
fn build_auth_service(
    config: &Config,
    pool: PgPool,
    cache: Arc<dyn TokenVersionCache>,
) -> Result<Arc<dyn AuthService>, Box<dyn std::error::Error + Send + Sync>> {
    let credentials = UserCredentialsRepo::new(pool.clone());
    let identities = DomainIdentityRepo::new(pool.clone());

    // The auth usecase needs an `auth::UserService` (a domain port
    // from the auth crate, not the apis port) to validate codes
    // against the user table. We bridge the apis::user::UserService
    // (built from the user Postgres repo) into the auth domain's
    // port via `AuthUserServiceImpl`.
    let apis_user: Arc<dyn UserService> = build_user_service(pool);
    let auth_user: Arc<dyn AuthUserService> = Arc::new(AuthUserServiceImpl::new(apis_user));

    let usecase = AuthUsecase::new(AuthUsecaseConfig {
        credentials,
        identities,
        user_service: auth_user,
        cache,
        signing_key: config.signing_key.clone(),
        access_ttl: config.access_ttl,
        refresh_ttl: config.refresh_ttl,
    });
    Ok(Arc::new(AuthServiceImpl::new(usecase)))
}

/// Wrap the user-crate's `UserServiceImpl` in `Arc<dyn UserService>`
/// (the apis port) so the HTTP layer can use it.
fn build_user_service(pool: PgPool) -> Arc<dyn UserService> {
    let repo = UserRepo::new(pool);
    let usecase = UserUsecase::new(repo);
    let service = UserServiceImpl::new(usecase);
    Arc::new(service)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static ENV_LOCK: Mutex<()> = Mutex::new(());
    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: env vars are process-global; ENV_LOCK serializes.
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
        // SAFETY: serialized via ENV_LOCK.
        unsafe { std::env::set_var(key, value); }
        EnvGuard { key, prev }
    }

    #[test]
    fn init_tracing_is_idempotent() {
        // Calling `init_tracing` twice shouldn't panic — the
        // `try_init` path swallows the "already initialized" error.
        // AEGIS_LOG_DIR is pointed at a temp dir so the file appender
        // does not write into the repo's working tree.
        let tmp = std::env::temp_dir().join("aegis-server-logger-test-idempotent");
        let _ = std::fs::create_dir_all(&tmp);
        let _g = lock_env();
        let _dir = set_env("AEGIS_LOG_DIR", tmp.to_str().unwrap());
        let _lvl = set_env("AEGIS_LOG_LEVEL", "info");
        let _a = init_tracing();
        let _b = init_tracing();
    }

    #[test]
    fn init_tracing_defaults_level_to_info_when_env_missing() {
        let _g = lock_env();
        // SAFETY: serialized via ENV_LOCK.
        unsafe { std::env::remove_var("AEGIS_LOG_LEVEL"); }
        let filter = build_env_filter();
        // The default directive ("info") is present somewhere in the
        // directive list, and the filter is parseable.
        assert_eq!(filter.to_string(), "info");
    }

    #[test]
    fn init_tracing_uses_aegis_log_level_when_set() {
        let _g = lock_env();
        let _lvl = set_env("AEGIS_LOG_LEVEL", "debug");
        let filter = build_env_filter();
        assert_eq!(filter.to_string(), "debug");
    }
}
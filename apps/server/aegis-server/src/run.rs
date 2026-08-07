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
    init_tracing();

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

/// Initialise tracing. Honors `RUST_LOG`; falls back to `info` for
/// the `aegis_server` crate and `warn` for everything else.
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("aegis_server=info,axum=info,sqlx=warn,tower_http=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
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

    #[test]
    fn init_tracing_is_idempotent() {
        // Calling `init_tracing` twice shouldn't panic — the
        // `try_init` path swallows the "already initialized" error.
        init_tracing();
        init_tracing();
    }
}
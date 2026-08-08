//! `OpenApiRouter` sub-router for the auth flows.
//!
//! Mirrors the [`crate::transport::http::router`] composition but
//! scoped to the `/api/auth/*` prefix. The top-level router composes
//! this via [`OpenApiRouter::nest`] so the auth namespace can grow
//! without inflating `router.rs`.
//!
//! Every session-lifecycle handler here is a `POST`. The `routes!`
//! macro registers "one HTTP method per `routes!` call" (utoipa-axum
//! 0.2.0), so we chain four single-handler calls instead of one
//! four-handler call — a single `routes!(login, login_domain,
//! refresh, logout)` would panic with "Overlapping method route".
//!
//! The `user_credential` namespace is composed via
//! [`OpenApiRouter::nest`] under `/user-credential` so the URL
//! surface reads as `/api/auth/user-credential/*`.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::auth::handlers;
use crate::transport::http::auth::user_credential;

/// Build the `/api/auth` sub-router.
///
/// The returned `OpenApiRouter<AppState>` is ready to be passed to
/// [`OpenApiRouter::nest`]. The handlers are reachable under:
///
/// - `POST  /api/auth/login`
/// - `POST  /api/auth/login-domain`
/// - `POST  /api/auth/refresh`
/// - `POST  /api/auth/logout`
/// - `PATCH /api/auth/user-credential`
///
/// (There is no `POST /api/auth/user-credential` — credential
/// creation happens out of band.)
pub fn router() -> OpenApiRouter<AppState> {
    // Each `routes!` call registers a single POST handler. The
    // `routes!` macro panics when two handlers of the same HTTP
    // method appear in the same invocation, so we issue one call
    // per handler. The chained `.routes(...)` calls accumulate the
    // handlers into a single `OpenApiRouter<AppState>`.
    OpenApiRouter::new()
        .nest("/user-credential", user_credential::router())
        .routes(routes!(handlers::login))
        .routes(routes!(handlers::login_domain))
        .routes(routes!(handlers::refresh))
        .routes(routes!(handlers::logout))
}

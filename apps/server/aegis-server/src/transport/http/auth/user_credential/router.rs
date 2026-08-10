//! `OpenApiRouter` sub-router for the user-credential management
//! namespace.
//!
//! Mounted by [`crate::transport::http::auth::router`] under the
//! `/user-credential` prefix so the URL surface reads as
//! `/api/auth/user-credential/*`. Both handlers require
//! `AuthClaims` (the per-handler `#[utoipa::path]` annotations
//! advertise the `BearerAuth` security scheme).
//!
//! Each `routes!` call registers a single handler so the
//! `utoipa-axum` "one method per call" rule is honoured. The
//! `PATCH` handler operates on the caller's own credential only,
//! with `user_code` derived from the bearer token (never from the
//! URL or body). The `POST` handler is the administrator-only
//! registration entry point that creates a new user + credential +
// domain identity through [`apis::auth::AuthService::register_user`].

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::auth::user_credential::handlers;

/// Build the `/api/auth/user-credential` sub-router.
///
/// The returned `OpenApiRouter<AppState>` is ready to be passed to
/// [`OpenApiRouter::nest`]. The handlers are reachable under:
///
/// - `POST   /api/auth/user-credential` — admin/root registration
/// - `PATCH  /api/auth/user-credential` — caller self-service rotation
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::register))
        .routes(routes!(handlers::update))
}

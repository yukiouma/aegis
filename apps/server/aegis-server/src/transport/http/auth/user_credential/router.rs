//! `OpenApiRouter` sub-router for the user-credential management
//! namespace.
//!
//! Mounted by [`crate::transport::http::auth::router`] under the
//! `/user-credential` prefix so the URL surface reads as
//! `/api/auth/user-credential/*`. Every handler here requires
//! `AuthClaims` (the per-handler `#[utoipa::path]` annotations
//! advertise the `BearerAuth` security scheme).
//!
//! Each `routes!` call registers a single handler. The `routes!`
//! macro panics when two handlers of the same HTTP method appear in
//! the same invocation, so we issue one call per handler. Today the
//! four handlers span four HTTP methods (POST on `/`, GET + PATCH +
//! DELETE on `/{code}`) — no two share an HTTP method, so each
//! registration is single-handler by construction.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::auth::user_credential::handlers;

/// Build the `/api/auth/user-credential` sub-router.
///
/// The returned `OpenApiRouter<AppState>` is ready to be passed to
/// [`OpenApiRouter::nest`]. The handlers are reachable under:
///
/// - `POST   /api/auth/user-credential`
/// - `GET    /api/auth/user-credential/{code}`
/// - `PATCH  /api/auth/user-credential/{code}`
/// - `DELETE /api/auth/user-credential/{code}`
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create))
        .routes(routes!(handlers::update))
}

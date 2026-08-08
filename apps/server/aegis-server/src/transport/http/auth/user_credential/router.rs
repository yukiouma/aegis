//! `OpenApiRouter` sub-router for the user-credential management
//! namespace.
//!
//! Mounted by [`crate::transport::http::auth::router`] under the
//! `/user-credential` prefix so the URL surface reads as
//! `/api/auth/user-credential/*`. The handler here requires
//! `AuthClaims` (the per-handler `#[utoipa::path]` annotation
//! advertises the `BearerAuth` security scheme).
//!
//! Each `routes!` call registers a single handler. Today only
//! `PATCH` is exposed — the handler operates on the caller's own
//! credential only, with `user_code` derived from the bearer token
//! (never from the URL or body). Credential creation is out of
//! scope for this HTTP surface and happens through a seed script
//! or admin tool.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::auth::user_credential::handlers;

/// Build the `/api/auth/user-credential` sub-router.
///
/// The returned `OpenApiRouter<AppState>` is ready to be passed to
/// [`OpenApiRouter::nest`]. The handler is reachable under:
///
/// - `PATCH /api/auth/user-credential`
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(handlers::update))
}
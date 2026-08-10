//! `OpenApiRouter` sub-router for the user CRUD namespace.
//!
//! Mirrors [`mod@crate::transport::http::auth::router`] composition but
//! scoped to the `/api/user/*` prefix. The top-level router composes
//! this via [`OpenApiRouter::nest`] so the user namespace can grow
//! without inflating `router.rs`.
//!
//! Each `routes!` call registers a single handler. The `routes!`
//! macro panics when two handlers of the same HTTP method appear in
//! the same invocation, so we issue one call per handler. Today the
//! four handlers span three methods (POST + GET on `/`, GET + PATCH
//! on `/{code}`) and no two share an HTTP method, so each
//! registration is single-handler by construction.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::user::handlers;

/// Build the `/api/user` sub-router.
///
/// The returned `OpenApiRouter<AppState>` is ready to be passed to
/// [`OpenApiRouter::nest`]. The handlers are reachable under:
///
/// - `POST   /api/user`
/// - `GET    /api/user`
/// - `GET    /api/user/{code}`
/// - `PATCH  /api/user/{code}`
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create))
        .routes(routes!(handlers::list))
        .routes(routes!(handlers::get_by_code))
        .routes(routes!(handlers::update))
}

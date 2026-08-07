//! Liveness probe handler.
//!
//! Returns the static string `"ok"` with HTTP 200. Used by container
//! orchestrators (and humans) to verify the server is responding to
//! HTTP at all, before any auth or DB wiring is exercised.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

/// Liveness probe. Always returns `200 OK` with a `text/plain; charset=utf-8`
/// body of `"ok"`.
#[utoipa::path(
    get,
    path = "",
    tag = "system",
    responses(
        (status = 200, description = "Liveness probe response", content_type = "text/plain", body = String),
    ),
)]
pub async fn healthz() -> Response {
    (StatusCode::OK, "ok").into_response()
}

/// Build the `/healthz` sub-router.
///
/// The sub-router is typed `OpenApiRouter<AppState>` so it can be
/// passed to [`OpenApiRouter::nest`] alongside the auth sub-router
/// (which also needs `AppState`). The handler itself does not
/// extract the state — it is ignored at request time. The path is
/// `""` because [`crate::transport::http::router::router`] mounts
/// this sub-router under `/healthz` via [`OpenApiRouter::nest`].
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(healthz))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn healthz_returns_200_ok() {
        let response = healthz().await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 16).await.unwrap();
        assert_eq!(&body[..], b"ok");
    }
}
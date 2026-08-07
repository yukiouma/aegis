//! Liveness probe handler.
//!
//! Returns the static string `"ok"` with HTTP 200. Used by container
//! orchestrators (and humans) to verify the server is responding to
//! HTTP at all, before any auth or DB wiring is exercised.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Liveness probe. Always returns `200 OK` with a `text/plain; charset=utf-8`
/// body of `"ok"`.
pub async fn healthz() -> Response {
    (StatusCode::OK, "ok").into_response()
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
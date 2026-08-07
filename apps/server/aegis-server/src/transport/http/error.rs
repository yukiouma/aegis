//! HTTP error mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ApiError(pub apis::auth::AuthApiError);

/// Stub `IntoResponse` so the scaffold compiles. Task 5 replaces
/// this with the real status-coded JSON mapping.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
    }
}
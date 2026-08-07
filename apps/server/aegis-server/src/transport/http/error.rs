//! HTTP error mapping.
//!
//! [`ApiError`] wraps [`apis::auth::AuthApiError`] and adds an HTTP
//! status code + a JSON [`ErrorBody`] shape. Every handler returns
//! `Result<Json<T>, ApiError>` and uses `?` on `AuthApiError`; the
//! [`From`] impl does the wrapping.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Stable JSON error envelope returned to clients.
///
/// `code` is a machine-readable string (e.g. `invalid_credentials`)
/// that clients should switch on. `message` is human-readable and
/// may be surfaced in a UI.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

/// Newtype around [`apis::auth::AuthApiError`] that adds an HTTP
/// status code and renders as JSON [`ErrorBody`].
#[derive(Debug, Error)]
#[error("{0}")]
pub struct ApiError(pub apis::auth::AuthApiError);

impl ApiError {
    /// HTTP status code for this error variant.
    pub fn status(&self) -> StatusCode {
        match &self.0 {
            apis::auth::AuthApiError::Validation(_) => StatusCode::BAD_REQUEST,
            apis::auth::AuthApiError::NotFound => StatusCode::NOT_FOUND,
            apis::auth::AuthApiError::Inactive => StatusCode::FORBIDDEN,
            apis::auth::AuthApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            apis::auth::AuthApiError::Verification(_) => StatusCode::UNAUTHORIZED,
            apis::auth::AuthApiError::DuplicateCode(_) => StatusCode::CONFLICT,
            apis::auth::AuthApiError::Signing(_) => StatusCode::INTERNAL_SERVER_ERROR,
            apis::auth::AuthApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable machine-readable code used as `ErrorBody.code`.
    pub fn code(&self) -> &'static str {
        match &self.0 {
            apis::auth::AuthApiError::Validation(_) => "validation_failed",
            apis::auth::AuthApiError::NotFound => "not_found",
            apis::auth::AuthApiError::Inactive => "user_inactive",
            apis::auth::AuthApiError::InvalidCredentials => "invalid_credentials",
            apis::auth::AuthApiError::Verification(_) => "token_verification_failed",
            apis::auth::AuthApiError::DuplicateCode(_) => "duplicate_code",
            apis::auth::AuthApiError::Signing(_) => "signing_failed",
            apis::auth::AuthApiError::Repository(_) => "repository_error",
        }
    }
}

impl From<apis::auth::AuthApiError> for ApiError {
    fn from(err: apis::auth::AuthApiError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(
                code = self.code(),
                error = %self.0,
                "api error",
            );
        }
        let body = ErrorBody {
            code: self.code().to_string(),
            message: self.0.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `IntoResponse::into_response` and recover the status +
    /// JSON body so each variant can be asserted directly. The body
    /// bytes are re-parsed into `ErrorBody` for a structured
    /// comparison.
    async fn render(err: apis::auth::AuthApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn validation_maps_to_400() {
        let (status, body) = render(apis::auth::AuthApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let (status, body) = render(apis::auth::AuthApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn inactive_maps_to_403() {
        let (status, body) = render(apis::auth::AuthApiError::Inactive).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.code, "user_inactive");
    }

    #[tokio::test]
    async fn invalid_credentials_maps_to_401() {
        let (status, body) = render(apis::auth::AuthApiError::InvalidCredentials).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "invalid_credentials");
    }

    #[tokio::test]
    async fn verification_maps_to_401() {
        let (status, body) = render(apis::auth::AuthApiError::Verification("bad sig".into())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "token_verification_failed");
    }

    #[tokio::test]
    async fn duplicate_code_maps_to_409() {
        let (status, body) = render(apis::auth::AuthApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn signing_maps_to_500() {
        let (status, body) = render(apis::auth::AuthApiError::Signing("boom".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "signing_failed");
    }

    #[tokio::test]
    async fn repository_maps_to_500() {
        let (status, body) = render(apis::auth::AuthApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    #[test]
    fn from_auth_api_error_wraps() {
        let inner = apis::auth::AuthApiError::NotFound;
        let outer = ApiError::from(inner);
        // `AuthApiError` does not implement `PartialEq`, so assert
        // through the rendering path which is stable for these
        // unit-style variants.
        assert_eq!(outer.status(), StatusCode::NOT_FOUND);
        assert_eq!(outer.code(), "not_found");
    }
}
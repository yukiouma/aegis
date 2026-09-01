//! HTTP error mapping.
//!
//! [`ApiError`] is an enum that wraps every apis error type the HTTP
//! layer surfaces today — [`apis::auth::AuthApiError`] and
//! [`apis::user::UserApiError`]. Every handler returns
//! `Result<Json<T>, ApiError>` and uses `?` on either inner error; the
//! [`From`] impls (derived via `#[from]`) do the wrapping.
//!
//! New apis services land as additional enum variants; the `status()`
//! and `code()` dispatch tables pick them up.

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

/// Error type returned by every HTTP handler.
///
/// Each variant wraps an apis-level error and implements
/// [`IntoResponse`] so handlers can return `Result<_, ApiError>` and
/// let the `?` operator do the conversion. The dispatch tables live
/// in private `*_status` / `*_code` helpers so the public `status()`
/// and `code()` methods stay table-shaped.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Auth(#[from] apis::auth::AuthApiError),

    #[error("{0}")]
    User(#[from] apis::user::UserApiError),

    #[error("{0}")]
    Project(#[from] apis::project::ProjectApiError),

    #[error("{0}")]
    Terminology(#[from] apis::terminology::TerminologyApiError),

    #[error("{0}")]
    DomainModel(#[from] apis::domain_model::DomainModelApiError),

    #[error("{0}")]
    Crf(#[from] apis::crf::CrfApiError),

    #[error("{0}")]
    Mission(#[from] apis::mission::MissionApiError),

    #[error("admin or root role required")]
    Forbidden,
}

impl ApiError {
    /// HTTP status code for this error variant.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Auth(e) => auth_status(e),
            Self::User(e) => user_status(e),
            Self::Project(e) => project_status(e),
            Self::Terminology(e) => terminology_status(e),
            Self::DomainModel(e) => domain_model_status(e),
            Self::Crf(e) => crf_status(e),
            Self::Mission(e) => mission_status(e),
            Self::Forbidden => StatusCode::FORBIDDEN,
        }
    }

    /// Stable machine-readable code used as `ErrorBody.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Auth(e) => auth_code(e),
            Self::User(e) => user_code(e),
            Self::Project(e) => project_code(e),
            Self::Terminology(e) => terminology_code(e),
            Self::DomainModel(e) => domain_model_code(e),
            Self::Crf(e) => crf_code(e),
            Self::Mission(e) => mission_code(e),
            Self::Forbidden => "forbidden",
        }
    }
}

fn auth_status(e: &apis::auth::AuthApiError) -> StatusCode {
    use apis::auth::AuthApiError;
    match e {
        AuthApiError::Validation(_) => StatusCode::BAD_REQUEST,
        AuthApiError::NotFound => StatusCode::NOT_FOUND,
        AuthApiError::Inactive => StatusCode::FORBIDDEN,
        AuthApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        AuthApiError::Verification(_) => StatusCode::UNAUTHORIZED,
        AuthApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        AuthApiError::Signing(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AuthApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn auth_code(e: &apis::auth::AuthApiError) -> &'static str {
    use apis::auth::AuthApiError;
    match e {
        AuthApiError::Validation(_) => "validation_failed",
        AuthApiError::NotFound => "not_found",
        AuthApiError::Inactive => "user_inactive",
        AuthApiError::InvalidCredentials => "invalid_credentials",
        AuthApiError::Verification(_) => "token_verification_failed",
        AuthApiError::DuplicateCode(_) => "duplicate_code",
        AuthApiError::Signing(_) => "signing_failed",
        AuthApiError::Repository(_) => "repository_error",
    }
}

fn user_status(e: &apis::user::UserApiError) -> StatusCode {
    use apis::user::UserApiError;
    match e {
        UserApiError::Validation(_) => StatusCode::BAD_REQUEST,
        UserApiError::NotFound => StatusCode::NOT_FOUND,
        UserApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        UserApiError::Hashing(_) => StatusCode::INTERNAL_SERVER_ERROR,
        UserApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn user_code(e: &apis::user::UserApiError) -> &'static str {
    use apis::user::UserApiError;
    match e {
        UserApiError::Validation(_) => "validation_failed",
        UserApiError::NotFound => "not_found",
        UserApiError::DuplicateCode(_) => "duplicate_code",
        UserApiError::Hashing(_) => "hashing_failed",
        UserApiError::Repository(_) => "repository_error",
    }
}

fn project_status(e: &apis::project::ProjectApiError) -> StatusCode {
    use apis::project::ProjectApiError;
    match e {
        ProjectApiError::Validation(_) => StatusCode::BAD_REQUEST,
        ProjectApiError::NotFound | ProjectApiError::UserNotFound(_) => StatusCode::NOT_FOUND,
        ProjectApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        ProjectApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn project_code(e: &apis::project::ProjectApiError) -> &'static str {
    use apis::project::ProjectApiError;
    match e {
        ProjectApiError::Validation(_) => "validation_failed",
        ProjectApiError::NotFound => "not_found",
        ProjectApiError::UserNotFound(_) => "user_not_found",
        ProjectApiError::DuplicateCode(_) => "duplicate_code",
        ProjectApiError::Repository(_) => "repository_error",
    }
}

fn terminology_status(e: &apis::terminology::TerminologyApiError) -> StatusCode {
    use apis::terminology::TerminologyApiError;
    match e {
        TerminologyApiError::Validation(_) => StatusCode::BAD_REQUEST,
        TerminologyApiError::NotFound => StatusCode::NOT_FOUND,
        TerminologyApiError::DuplicateVersion { .. }
        | TerminologyApiError::DuplicateCodeList { .. }
        | TerminologyApiError::DuplicateCodeItem { .. } => StatusCode::CONFLICT,
        TerminologyApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn terminology_code(e: &apis::terminology::TerminologyApiError) -> &'static str {
    use apis::terminology::TerminologyApiError;
    match e {
        TerminologyApiError::Validation(_) => "validation_failed",
        TerminologyApiError::NotFound => "not_found",
        TerminologyApiError::DuplicateVersion { .. } => "duplicate_terminology_version",
        TerminologyApiError::DuplicateCodeList { .. } => "duplicate_code_list",
        TerminologyApiError::DuplicateCodeItem { .. } => "duplicate_code_item",
        TerminologyApiError::Repository(_) => "repository_error",
    }
}

fn domain_model_status(e: &apis::domain_model::DomainModelApiError) -> StatusCode {
    use apis::domain_model::DomainModelApiError;
    match e {
        DomainModelApiError::Validation(_) => StatusCode::BAD_REQUEST,
        DomainModelApiError::NotFound
        | DomainModelApiError::SdtmVersionNotFound(_)
        | DomainModelApiError::SdtmDomainNotFound(_)
        | DomainModelApiError::SdtmVariableNotFound(_) => StatusCode::NOT_FOUND,
        DomainModelApiError::DuplicateSdtmVersion { .. }
        | DomainModelApiError::DuplicateSdtmDomain { .. }
        | DomainModelApiError::DuplicateSdtmVariable { .. } => StatusCode::CONFLICT,
        DomainModelApiError::FkSdtmVersionNotFound(_)
        | DomainModelApiError::FkSdtmDomainNotFound(_) => StatusCode::BAD_REQUEST,
        DomainModelApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn domain_model_code(e: &apis::domain_model::DomainModelApiError) -> &'static str {
    use apis::domain_model::DomainModelApiError;
    match e {
        DomainModelApiError::Validation(_) => "validation_failed",
        DomainModelApiError::NotFound => "not_found",
        DomainModelApiError::SdtmVersionNotFound(_) => "sdtm_version_not_found",
        DomainModelApiError::SdtmDomainNotFound(_) => "sdtm_domain_not_found",
        DomainModelApiError::SdtmVariableNotFound(_) => "sdtm_variable_not_found",
        DomainModelApiError::DuplicateSdtmVersion { .. } => "duplicate_sdtm_version",
        DomainModelApiError::DuplicateSdtmDomain { .. } => "duplicate_sdtm_domain",
        DomainModelApiError::DuplicateSdtmVariable { .. } => "duplicate_sdtm_variable",
        DomainModelApiError::FkSdtmVersionNotFound(_) => "fk_sdtm_version_not_found",
        DomainModelApiError::FkSdtmDomainNotFound(_) => "fk_sdtm_domain_not_found",
        DomainModelApiError::Repository(_) => "repository_error",
    }
}

fn crf_status(e: &apis::crf::CrfApiError) -> StatusCode {
    use apis::crf::CrfApiError;
    match e {
        CrfApiError::Validation(_) | CrfApiError::EmptySearchFragment => StatusCode::BAD_REQUEST,
        CrfApiError::NotFound
        | CrfApiError::ProjectNotFound(_)
        | CrfApiError::CrfVersionNotFound(_)
        | CrfApiError::CrfFormNotFound(_)
        | CrfApiError::CrfItemNotFound(_)
        | CrfApiError::CrfOptionNotFound(_)
        | CrfApiError::CrfUnitNotFound(_)
        | CrfApiError::DomainAnnotationNotFound(_)
        | CrfApiError::AnnotationNotFound(_) => StatusCode::NOT_FOUND,
        CrfApiError::DuplicateCrfVersion { .. }
        | CrfApiError::DuplicateCrfForm { .. }
        | CrfApiError::DuplicateCrfItem { .. }
        | CrfApiError::DuplicateDomainAnnotation { .. } => StatusCode::CONFLICT,
        CrfApiError::KindShapeViolation { .. } => StatusCode::BAD_REQUEST,
        CrfApiError::FkCrfVersionNotFound(_)
        | CrfApiError::FkCrfFormNotFound(_)
        | CrfApiError::FkCrfItemNotFound(_)
        | CrfApiError::FkCrfOptionNotFound(_)
        | CrfApiError::FkCrfUnitNotFound(_)
        | CrfApiError::FkDomainAnnotationNotFound(_) => StatusCode::BAD_REQUEST,
        CrfApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn crf_code(e: &apis::crf::CrfApiError) -> &'static str {
    use apis::crf::CrfApiError;
    match e {
        CrfApiError::Validation(_) => "validation_failed",
        CrfApiError::NotFound => "not_found",
        CrfApiError::ProjectNotFound(_) => "project_not_found",
        CrfApiError::CrfVersionNotFound(_) => "crf_version_not_found",
        CrfApiError::CrfFormNotFound(_) => "crf_form_not_found",
        CrfApiError::CrfItemNotFound(_) => "crf_item_not_found",
        CrfApiError::CrfOptionNotFound(_) => "crf_option_not_found",
        CrfApiError::CrfUnitNotFound(_) => "crf_unit_not_found",
        CrfApiError::DomainAnnotationNotFound(_) => "domain_annotation_not_found",
        CrfApiError::AnnotationNotFound(_) => "annotation_not_found",
        CrfApiError::DuplicateCrfVersion { .. } => "duplicate_crf_version",
        CrfApiError::DuplicateCrfForm { .. } => "duplicate_crf_form",
        CrfApiError::DuplicateCrfItem { .. } => "duplicate_crf_item",
        CrfApiError::DuplicateDomainAnnotation { .. } => "duplicate_domain_annotation",
        CrfApiError::FkCrfVersionNotFound(_) => "fk_crf_version_not_found",
        CrfApiError::FkCrfFormNotFound(_) => "fk_crf_form_not_found",
        CrfApiError::FkCrfItemNotFound(_) => "fk_crf_item_not_found",
        CrfApiError::FkCrfOptionNotFound(_) => "fk_crf_option_not_found",
        CrfApiError::FkCrfUnitNotFound(_) => "fk_crf_unit_not_found",
        CrfApiError::FkDomainAnnotationNotFound(_) => "fk_domain_annotation_not_found",
        CrfApiError::EmptySearchFragment => "empty_search_fragment",
        CrfApiError::KindShapeViolation { .. } => "kind_shape_violation",
        CrfApiError::Repository(_) => "repository_error",
    }
}

fn mission_status(e: &apis::mission::MissionApiError) -> StatusCode {
    use apis::mission::MissionApiError;
    match e {
        MissionApiError::Validation(_) => StatusCode::BAD_REQUEST,
        MissionApiError::NotFound | MissionApiError::AssigneeNotFound => StatusCode::NOT_FOUND,
        MissionApiError::ProjectNotFound(_) | MissionApiError::UserNotFound(_) => {
            StatusCode::NOT_FOUND
        }
        MissionApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
        MissionApiError::DuplicateMission { .. } | MissionApiError::DuplicateAssignee { .. } => {
            StatusCode::CONFLICT
        }
        MissionApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn mission_code(e: &apis::mission::MissionApiError) -> &'static str {
    use apis::mission::MissionApiError;
    match e {
        MissionApiError::Validation(_) => "validation_failed",
        MissionApiError::NotFound => "not_found",
        MissionApiError::AssigneeNotFound => "assignee_not_found",
        MissionApiError::ProjectNotFound(_) => "project_not_found",
        MissionApiError::UserNotFound(_) => "user_not_found",
        MissionApiError::Forbidden { .. } => "forbidden",
        MissionApiError::DuplicateMission { .. } => "duplicate_mission",
        MissionApiError::DuplicateAssignee { .. } => "duplicate_assignee",
        MissionApiError::Repository(_) => "repository_error",
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(
                code = self.code(),
                error = %self,
                "api error",
            );
        }
        let body = ErrorBody {
            code: self.code().to_string(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `IntoResponse::into_response` and recover the status +
    /// JSON body so each `AuthApiError` variant can be asserted
    /// directly. The body bytes are re-parsed into `ErrorBody` for a
    /// structured comparison.
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

    /// Drive `IntoResponse::into_response` for a `UserApiError` and
    /// recover the status + JSON body so each variant can be
    /// asserted directly.
    async fn render_user(err: apis::user::UserApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    // ---- AuthApiError mapping (unchanged from prior behaviour) -----

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

    // ---- UserApiError mapping (new) -----

    #[tokio::test]
    async fn user_validation_maps_to_400() {
        let (status, body) = render_user(apis::user::UserApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn user_not_found_maps_to_404() {
        let (status, body) = render_user(apis::user::UserApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn user_duplicate_code_maps_to_409() {
        let (status, body) =
            render_user(apis::user::UserApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn user_hashing_maps_to_500() {
        let (status, body) = render_user(apis::user::UserApiError::Hashing("oops".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "hashing_failed");
    }

    #[tokio::test]
    async fn user_repository_maps_to_500() {
        let (status, body) =
            render_user(apis::user::UserApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- ProjectApiError mapping -----

    async fn render_project(err: apis::project::ProjectApiError) -> (StatusCode, ErrorBody) {
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
    async fn project_validation_maps_to_400() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn project_not_found_maps_to_404() {
        let (status, body) = render_project(apis::project::ProjectApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn project_user_not_found_maps_to_404() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::UserNotFound("u1".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "user_not_found");
    }

    #[tokio::test]
    async fn project_duplicate_code_maps_to_409() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::DuplicateCode("dup".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn project_repository_maps_to_500() {
        let (status, body) =
            render_project(apis::project::ProjectApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- Forbidden (authorization) -----

    #[tokio::test]
    async fn forbidden_maps_to_403() {
        let response = ApiError::Forbidden.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.code, "forbidden");
        assert_eq!(parsed.message, "admin or root role required");
    }

    // ---- TerminologyApiError mapping -----

    async fn render_terminology(
        err: apis::terminology::TerminologyApiError,
    ) -> (StatusCode, ErrorBody) {
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
    async fn terminology_validation_maps_to_400() {
        let (status, body) = render_terminology(
            apis::terminology::TerminologyApiError::Validation("bad".into()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn terminology_not_found_maps_to_404() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn terminology_duplicate_version_maps_to_409() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::DuplicateVersion {
                kind: apis::terminology::TerminologyKind::Sdtm,
                name: "v1".into(),
            })
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_terminology_version");
    }

    #[tokio::test]
    async fn terminology_duplicate_code_list_maps_to_409() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::DuplicateCodeList {
                version_id: 1,
                code: "C66741".into(),
            })
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code_list");
    }

    #[tokio::test]
    async fn terminology_duplicate_code_item_maps_to_409() {
        let (status, body) =
            render_terminology(apis::terminology::TerminologyApiError::DuplicateCodeItem {
                codelist_id: 1,
                code: "C1".into(),
            })
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code_item");
    }

    #[tokio::test]
    async fn terminology_repository_maps_to_500() {
        let (status, body) = render_terminology(
            apis::terminology::TerminologyApiError::Repository("db".into()),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- DomainModelApiError mapping -----

    async fn render_domain_model(
        err: apis::domain_model::DomainModelApiError,
    ) -> (StatusCode, ErrorBody) {
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
    async fn domain_model_validation_maps_to_400() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::Validation("bad".into()),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn domain_model_not_found_maps_to_404() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn domain_model_sdtm_version_not_found_maps_to_404() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::SdtmVersionNotFound(42))
                .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "sdtm_version_not_found");
    }

    #[tokio::test]
    async fn domain_model_sdtm_domain_not_found_maps_to_404() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::SdtmDomainNotFound(42),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "sdtm_domain_not_found");
    }

    #[tokio::test]
    async fn domain_model_sdtm_variable_not_found_maps_to_404() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::SdtmVariableNotFound(42))
                .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "sdtm_variable_not_found");
    }

    #[tokio::test]
    async fn domain_model_duplicate_sdtm_version_maps_to_409() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::DuplicateSdtmVersion { name: "v1".into() },
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_sdtm_version");
    }

    #[tokio::test]
    async fn domain_model_duplicate_sdtm_domain_maps_to_409() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::DuplicateSdtmDomain {
                version_id: 1,
                name: "DM".into(),
            },
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_sdtm_domain");
    }

    #[tokio::test]
    async fn domain_model_duplicate_sdtm_variable_maps_to_409() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::DuplicateSdtmVariable {
                domain_id: 1,
                name: "AGE".into(),
            },
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_sdtm_variable");
    }

    #[tokio::test]
    async fn domain_model_fk_sdtm_version_not_found_maps_to_400() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::FkSdtmVersionNotFound(99))
                .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "fk_sdtm_version_not_found");
    }

    #[tokio::test]
    async fn domain_model_fk_sdtm_domain_not_found_maps_to_400() {
        let (status, body) =
            render_domain_model(apis::domain_model::DomainModelApiError::FkSdtmDomainNotFound(99))
                .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "fk_sdtm_domain_not_found");
    }

    #[tokio::test]
    async fn domain_model_repository_maps_to_500() {
        let (status, body) = render_domain_model(
            apis::domain_model::DomainModelApiError::Repository("db".into()),
        )
        .await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    // ---- CrfApiError mapping -----

    async fn render_crf(err: apis::crf::CrfApiError) -> (StatusCode, ErrorBody) {
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
    async fn crf_validation_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
    }

    #[tokio::test]
    async fn crf_empty_search_fragment_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::EmptySearchFragment).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "empty_search_fragment");
    }

    #[tokio::test]
    async fn crf_kind_shape_violation_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::KindShapeViolation {
            kind: apis::crf::CrfItemKind::Selection,
            field: "options".into(),
        })
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "kind_shape_violation");
    }

    #[tokio::test]
    async fn crf_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn crf_project_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::ProjectNotFound("P1".into())).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "project_not_found");
    }

    #[tokio::test]
    async fn crf_version_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::CrfVersionNotFound(7)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "crf_version_not_found");
    }

    #[tokio::test]
    async fn crf_form_not_found_maps_to_404() {
        let (status, body) = render_crf(apis::crf::CrfApiError::CrfFormNotFound(7)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "crf_form_not_found");
    }

    #[tokio::test]
    async fn crf_duplicate_version_maps_to_409() {
        let (status, body) = render_crf(apis::crf::CrfApiError::DuplicateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_crf_version");
    }

    #[tokio::test]
    async fn crf_duplicate_form_maps_to_409() {
        let (status, body) = render_crf(apis::crf::CrfApiError::DuplicateCrfForm {
            version_id: 1,
            code: "F1".into(),
        })
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_crf_form");
    }

    #[tokio::test]
    async fn crf_fk_version_not_found_maps_to_400() {
        let (status, body) = render_crf(apis::crf::CrfApiError::FkCrfVersionNotFound(99)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "fk_crf_version_not_found");
    }

    #[tokio::test]
    async fn crf_repository_maps_to_500() {
        let (status, body) = render_crf(apis::crf::CrfApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }
}

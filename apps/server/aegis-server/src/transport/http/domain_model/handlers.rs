//! HTTP handlers for the domain-model namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from `dto`) into an apis DTO.
//! 2. Calls the corresponding [`DomainModelService`](apis::domain_model::DomainModelService)
//!    method on `AppState`.
//! 3. Translates the apis response back into a wire DTO.
//!
//! `DomainModelApiError` is funnelled through [`ApiError::from`] so
//! each route returns `Result<Json<T>, ApiError>` and the error
//! mapping in `transport::http::error` does the rest.
//!
//! The role policy lives in
//! [`crate::transport::http::auth::middleware::require_admin_or_root`];
//! every write handler (POST / PUT / DELETE) calls it before
//! dispatching to the usecase, matching the terminology module's
//! policy.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::{AuthClaims, require_admin_or_root};
use crate::transport::http::dto;
use crate::transport::http::error::ApiError;

// ---- SdtmVersion ----

/// `POST /api/domain-model/versions` — create an SDTM version.
#[utoipa::path(
    post, path = "/versions", tag = "domain-model",
    operation_id = "domain_model_create_version",
    request_body = dto::CreateSdtmVersionRequest,
    responses(
        (status = 201, description = "Version created", body = dto::SdtmVersionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate sdtm version", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateSdtmVersionRequest>,
) -> Result<(StatusCode, Json<dto::SdtmVersionViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .create_version(req.into())
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/domain-model/versions` — list SDTM versions.
#[utoipa::path(
    get, path = "/versions", tag = "domain-model",
    operation_id = "domain_model_list_versions",
    responses(
        (status = 200, description = "Versions list", body = Vec<dto::SdtmVersionViewResponse>),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_versions(
    State(state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<Vec<dto::SdtmVersionViewResponse>>, ApiError> {
    let views = state.domain_model.list_versions().await?;
    let out = views.into_iter().map(Into::into).collect();
    Ok(Json(out))
}

/// `PUT /api/domain-model/versions/{id}` — partial update of an SDTM version.
#[utoipa::path(
    put, path = "/versions/{id}", tag = "domain-model",
    operation_id = "domain_model_update_version",
    params(
        ("id" = i64, Path, description = "SDTM version id"),
    ),
    request_body = dto::UpdateSdtmVersionRequest,
    responses(
        (status = 200, description = "Version updated", body = dto::SdtmVersionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateSdtmVersionRequest>,
) -> Result<Json<dto::SdtmVersionViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .update_version(apis::domain_model::UpdateSdtmVersionRequest {
            id,
            name: req.name,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/domain-model/versions/{id}` — hard delete an SDTM version.
#[utoipa::path(
    delete, path = "/versions/{id}", tag = "domain-model",
    operation_id = "domain_model_delete_version",
    params(
        ("id" = i64, Path, description = "SDTM version id"),
    ),
    responses(
        (status = 204, description = "Version deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.domain_model.delete_version(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmDomain ----

/// `POST /api/domain-model/domains` — create an SDTM domain.
#[utoipa::path(
    post, path = "/domains", tag = "domain-model",
    operation_id = "domain_model_create_domain",
    request_body = dto::CreateSdtmDomainRequest,
    responses(
        (status = 201, description = "Domain created", body = dto::SdtmDomainViewResponse),
        (status = 400, description = "Validation failed or parent version not found", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate sdtm domain", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_domain(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateSdtmDomainRequest>,
) -> Result<(StatusCode, Json<dto::SdtmDomainViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state.domain_model.create_domain(req.into()).await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/domain-model/domains/{id}` — fetch an SDTM domain by id.
#[utoipa::path(
    get, path = "/domains/{id}", tag = "domain-model",
    operation_id = "domain_model_get_domain_by_id",
    params(
        ("id" = i64, Path, description = "SDTM domain id"),
    ),
    responses(
        (status = 200, description = "Domain found", body = dto::SdtmDomainViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_domain_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::SdtmDomainViewResponse>, ApiError> {
    let view = state.domain_model.get_domain_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `GET /api/domain-model/versions/{version_id}/domains` — list domains under a version.
#[utoipa::path(
    get, path = "/versions/{version_id}/domains", tag = "domain-model",
    operation_id = "domain_model_list_domains_by_version",
    params(
        ("version_id" = i64, Path, description = "SDTM version id"),
    ),
    responses(
        (status = 200, description = "Domains list", body = Vec<dto::SdtmDomainViewResponse>),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_domains_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(version_id): Path<i64>,
) -> Result<Json<Vec<dto::SdtmDomainViewResponse>>, ApiError> {
    let views = state.domain_model.list_domains_by_version(version_id).await?;
    let out = views.into_iter().map(Into::into).collect();
    Ok(Json(out))
}

/// `PUT /api/domain-model/domains/{id}` — partial update of an SDTM domain.
#[utoipa::path(
    put, path = "/domains/{id}", tag = "domain-model",
    operation_id = "domain_model_update_domain",
    params(
        ("id" = i64, Path, description = "SDTM domain id"),
    ),
    request_body = dto::UpdateSdtmDomainRequest,
    responses(
        (status = 200, description = "Domain updated", body = dto::SdtmDomainViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_domain(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateSdtmDomainRequest>,
) -> Result<Json<dto::SdtmDomainViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .update_domain(apis::domain_model::UpdateSdtmDomainRequest {
            id,
            name: req.name,
            category: req.category.map(Into::into),
            descriptions: req
                .descriptions
                .map(|ds| ds.into_iter().map(Into::into).collect()),
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/domain-model/domains/{id}` — hard delete an SDTM domain.
#[utoipa::path(
    delete, path = "/domains/{id}", tag = "domain-model",
    operation_id = "domain_model_delete_domain",
    params(
        ("id" = i64, Path, description = "SDTM domain id"),
    ),
    responses(
        (status = 204, description = "Domain deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_domain(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.domain_model.delete_domain(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmVariable ----

/// `POST /api/domain-model/variables` — create an SDTM variable.
#[utoipa::path(
    post, path = "/variables", tag = "domain-model",
    operation_id = "domain_model_create_variable",
    request_body = dto::CreateSdtmVariableRequest,
    responses(
        (status = 201, description = "Variable created", body = dto::SdtmVariableViewResponse),
        (status = 400, description = "Validation failed or parent domain not found", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate sdtm variable", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_variable(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateSdtmVariableRequest>,
) -> Result<(StatusCode, Json<dto::SdtmVariableViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state.domain_model.create_variable(req.into()).await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/domain-model/variables/{id}` — fetch an SDTM variable by id.
#[utoipa::path(
    get, path = "/variables/{id}", tag = "domain-model",
    operation_id = "domain_model_get_variable_by_id",
    params(
        ("id" = i64, Path, description = "SDTM variable id"),
    ),
    responses(
        (status = 200, description = "Variable found", body = dto::SdtmVariableViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Variable not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_variable_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::SdtmVariableViewResponse>, ApiError> {
    let view = state.domain_model.get_variable_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `GET /api/domain-model/domains/{domain_id}/variables` — list variables under a domain.
#[utoipa::path(
    get, path = "/domains/{domain_id}/variables", tag = "domain-model",
    operation_id = "domain_model_list_variables_by_domain",
    params(
        ("domain_id" = i64, Path, description = "SDTM domain id"),
    ),
    responses(
        (status = 200, description = "Variables list", body = Vec<dto::SdtmVariableViewResponse>),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_variables_by_domain(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(domain_id): Path<i64>,
) -> Result<Json<Vec<dto::SdtmVariableViewResponse>>, ApiError> {
    let views = state.domain_model.list_variables_by_domain(domain_id).await?;
    let out = views.into_iter().map(Into::into).collect();
    Ok(Json(out))
}

/// `PUT /api/domain-model/variables/{id}` — partial update of an SDTM variable.
#[utoipa::path(
    put, path = "/variables/{id}", tag = "domain-model",
    operation_id = "domain_model_update_variable",
    params(
        ("id" = i64, Path, description = "SDTM variable id"),
    ),
    request_body = dto::UpdateSdtmVariableRequest,
    responses(
        (status = 200, description = "Variable updated", body = dto::SdtmVariableViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Variable not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_variable(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateSdtmVariableRequest>,
) -> Result<Json<dto::SdtmVariableViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .update_variable(apis::domain_model::UpdateSdtmVariableRequest {
            id,
            name: req.name,
            variable_controlled: req.variable_controlled,
            variable_type: req.variable_type.map(Into::into),
            variable_core: req.variable_core.map(Into::into),
            variable_role: req
                .variable_role
                .map(|inner| inner.map(Into::into)),
            variable_sequence: req.variable_sequence,
            descriptions: req
                .descriptions
                .map(|ds| ds.into_iter().map(Into::into).collect()),
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/domain-model/variables/{id}` — hard delete an SDTM variable.
#[utoipa::path(
    delete, path = "/variables/{id}", tag = "domain-model",
    operation_id = "domain_model_delete_variable",
    params(
        ("id" = i64, Path, description = "SDTM variable id"),
    ),
    responses(
        (status = 204, description = "Variable deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_variable(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.domain_model.delete_variable(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
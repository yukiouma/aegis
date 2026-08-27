//! HTTP handlers for the CRF namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from `dto`) into an apis DTO.
//! 2. Calls the corresponding [`apis::crf::CrfService`] method on
//!    `AppState`.
//! 3. Translates the apis response back into a wire DTO.
//!
//! `CrfApiError` is funnelled through [`ApiError::from`] so each
//! route returns `Result<Json<T>, ApiError>` and the error mapping
//! in `transport::http::error` does the rest.
//!
//! The role policy lives in
//! [`crate::transport::http::auth::middleware::require_admin_or_root`];
//! every write handler (POST / PATCH / DELETE) calls it before
//! dispatching to the usecase, matching the terminology module's
//! policy.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::{AuthClaims, require_admin_or_root};
use crate::transport::http::dto::{self, CrfFragmentQuery, CrfPathId, ProjectPathCode};
use crate::transport::http::error::ApiError;

// ---- CrfVersion ----

/// `POST /api/crf/projects/{project_code}/versions` — create a CRF
/// version under a project.
#[utoipa::path(
    post, path = "/projects/{project_code}/versions", tag = "crf",
    operation_id = "crf_create_version",
    params(
        ("project_code" = String, Path, description = "Owning project code"),
    ),
    request_body = dto::CreateCrfVersionRequest,
    responses(
        (status = 201, description = "Version created", body = dto::CrfVersionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate CRF version", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(ProjectPathCode { project_code }): Path<ProjectPathCode>,
    Json(req): Json<dto::CreateCrfVersionRequest>,
) -> Result<(StatusCode, Json<dto::CrfVersionViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .create_version(apis::crf::CreateCrfVersionRequest {
            project_code,
            name: req.name,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/crf/projects/{project_code}/versions` — list every CRF
/// version attached to the project, ordered by id ASC.
#[utoipa::path(
    get, path = "/projects/{project_code}/versions", tag = "crf",
    operation_id = "crf_list_versions_by_project",
    params(
        ("project_code" = String, Path, description = "Owning project code"),
    ),
    responses(
        (status = 200, description = "Versions list", body = dto::CrfVersionListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_versions_by_project(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(ProjectPathCode { project_code }): Path<ProjectPathCode>,
) -> Result<Json<dto::CrfVersionListResponse>, ApiError> {
    let views = state
        .crf
        .list_versions_by_project(apis::crf::ListCrfVersionsByProjectRequest { project_code })
        .await?;
    let versions = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfVersionListResponse { versions }))
}

/// `GET /api/crf/versions/{id}` — fetch a CRF version by id.
#[utoipa::path(
    get, path = "/versions/{id}", tag = "crf",
    operation_id = "crf_get_version_by_id",
    params(
        ("id" = i32, Path, description = "CRF version id"),
    ),
    responses(
        (status = 200, description = "Version found", body = dto::CrfVersionViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_version_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfVersionViewResponse>, ApiError> {
    let view = state
        .crf
        .get_version_by_id(apis::crf::GetCrfVersionByIdRequest { id })
        .await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/crf/versions/{id}` — partial update of a CRF version.
#[utoipa::path(
    patch, path = "/versions/{id}", tag = "crf",
    operation_id = "crf_update_version",
    params(
        ("id" = i32, Path, description = "CRF version id"),
    ),
    request_body = dto::UpdateCrfVersionRequest,
    responses(
        (status = 200, description = "Version updated", body = dto::CrfVersionViewResponse),
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
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::UpdateCrfVersionRequest>,
) -> Result<Json<dto::CrfVersionViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .update_version(apis::crf::UpdateCrfVersionRequest { id, name: req.name })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/crf/versions/{id}` — hard delete a CRF version.
#[utoipa::path(
    delete, path = "/versions/{id}", tag = "crf",
    operation_id = "crf_delete_version",
    params(
        ("id" = i32, Path, description = "CRF version id"),
    ),
    responses(
        (status = 204, description = "Version deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.crf.delete_version(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- CrfForm ----

/// `POST /api/crf/versions/{version_id}/forms` — create a CRF form
/// under a version.
#[utoipa::path(
    post, path = "/versions/{version_id}/forms", tag = "crf",
    operation_id = "crf_create_form",
    params(
        ("version_id" = i32, Path, description = "Owning CRF version id"),
    ),
    request_body = dto::CreateCrfFormRequest,
    responses(
        (status = 201, description = "Form created", body = dto::CrfFormViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "CRF version not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate CRF form", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_form(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id: version_id }): Path<CrfPathId>,
    Json(req): Json<dto::CreateCrfFormRequest>,
) -> Result<(StatusCode, Json<dto::CrfFormViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .create_form(apis::crf::CreateCrfFormRequest {
            version_id,
            code: req.code,
            name: req.name,
            order: req.order,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/crf/versions/{version_id}/forms` — list every CRF form
/// under the given version, ordered by `order ASC, id ASC`.
#[utoipa::path(
    get, path = "/versions/{version_id}/forms", tag = "crf",
    operation_id = "crf_list_forms_by_version",
    params(
        ("version_id" = i32, Path, description = "Owning CRF version id"),
    ),
    responses(
        (status = 200, description = "Forms list", body = dto::CrfFormListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_forms_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: version_id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfFormListResponse>, ApiError> {
    let views = state
        .crf
        .list_forms_by_version(apis::crf::ListCrfFormsByVersionRequest { version_id })
        .await?;
    let forms = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfFormListResponse { forms }))
}

/// `GET /api/crf/versions/{version_id}/forms/search?fragment=...` —
/// version-scoped substring search on form code / name.
#[utoipa::path(
    get, path = "/versions/{version_id}/forms/search", tag = "crf",
    operation_id = "crf_search_forms_by_version",
    params(
        ("version_id" = i32, Path, description = "Owning CRF version id"),
        ("fragment" = String, Query, description = "Required non-empty text fragment"),
    ),
    responses(
        (status = 200, description = "Forms list", body = dto::CrfFormListResponse),
        (status = 400, description = "Empty search fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn search_forms_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: version_id }): Path<CrfPathId>,
    Query(CrfFragmentQuery { fragment }): Query<CrfFragmentQuery>,
) -> Result<Json<dto::CrfFormListResponse>, ApiError> {
    let views = state
        .crf
        .search_forms_by_version(apis::crf::SearchCrfFormsByVersionRequest {
            version_id,
            fragment,
        })
        .await?;
    let forms = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfFormListResponse { forms }))
}

/// `GET /api/crf/forms/{id}` — fetch a CRF form by id.
#[utoipa::path(
    get, path = "/forms/{id}", tag = "crf",
    operation_id = "crf_get_form_by_id",
    params(
        ("id" = i32, Path, description = "CRF form id"),
    ),
    responses(
        (status = 200, description = "Form found", body = dto::CrfFormViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Form not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_form_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfFormViewResponse>, ApiError> {
    let view = state
        .crf
        .get_form_by_id(apis::crf::GetCrfFormByIdRequest { id })
        .await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/crf/forms/{id}` — partial update of a CRF form.
#[utoipa::path(
    patch, path = "/forms/{id}", tag = "crf",
    operation_id = "crf_update_form",
    params(
        ("id" = i32, Path, description = "CRF form id"),
    ),
    request_body = dto::UpdateCrfFormRequest,
    responses(
        (status = 200, description = "Form updated", body = dto::CrfFormViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Form not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate CRF form", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_form(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::UpdateCrfFormRequest>,
) -> Result<Json<dto::CrfFormViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .update_form(apis::crf::UpdateCrfFormRequest {
            id,
            code: req.code,
            name: req.name,
            order: req.order,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/crf/forms/{id}` — hard delete a CRF form.
#[utoipa::path(
    delete, path = "/forms/{id}", tag = "crf",
    operation_id = "crf_delete_form",
    params(
        ("id" = i32, Path, description = "CRF form id"),
    ),
    responses(
        (status = 204, description = "Form deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Form not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_form(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.crf.delete_form(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- CrfItem ----

/// `POST /api/crf/forms/{form_id}/items` — create a CRF item under a
/// form. Kind-shape validation runs at the usecase.
#[utoipa::path(
    post, path = "/forms/{form_id}/items", tag = "crf",
    operation_id = "crf_create_item",
    params(
        ("form_id" = i32, Path, description = "Owning CRF form id"),
    ),
    request_body = dto::CreateCrfItemRequest,
    responses(
        (status = 201, description = "Item created", body = dto::CrfItemViewResponse),
        (status = 400, description = "Validation failed / kind-shape violation", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "CRF form not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate CRF item", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_item(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id: form_id }): Path<CrfPathId>,
    Json(req): Json<dto::CreateCrfItemRequest>,
) -> Result<(StatusCode, Json<dto::CrfItemViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .create_item(apis::crf::CreateCrfItemRequest {
            form_id,
            code: req.code,
            name: req.name,
            kind: req.kind.into(),
            order: req.order,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/crf/forms/{form_id}/items` — list every CRF item under
/// the given form, ordered by `order ASC, id ASC`.
#[utoipa::path(
    get, path = "/forms/{form_id}/items", tag = "crf",
    operation_id = "crf_list_items_by_form",
    params(
        ("form_id" = i32, Path, description = "Owning CRF form id"),
    ),
    responses(
        (status = 200, description = "Items list", body = dto::CrfItemListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_items_by_form(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: form_id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfItemListResponse>, ApiError> {
    let views = state
        .crf
        .list_items_by_form(apis::crf::ListCrfItemsByFormRequest { form_id })
        .await?;
    let items = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfItemListResponse { items }))
}

/// `GET /api/crf/forms/{form_id}/items/search?fragment=...` —
/// version-scoped substring search on item code / name. The version
/// is derived from the form at the usecase layer.
#[utoipa::path(
    get, path = "/forms/{form_id}/items/search", tag = "crf",
    operation_id = "crf_search_items_by_version",
    params(
        ("form_id" = i32, Path, description = "Owning CRF form id; the version is derived from this row"),
        ("fragment" = String, Query, description = "Required non-empty text fragment"),
    ),
    responses(
        (status = 200, description = "Items list", body = dto::CrfItemListResponse),
        (status = 400, description = "Empty search fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn search_items_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: form_id }): Path<CrfPathId>,
    Query(CrfFragmentQuery { fragment }): Query<CrfFragmentQuery>,
) -> Result<Json<dto::CrfItemListResponse>, ApiError> {
    // Resolve the form -> version_id at the apis layer.
    let form = state
        .crf
        .get_form_by_id(apis::crf::GetCrfFormByIdRequest { id: form_id })
        .await?;
    let views = state
        .crf
        .search_items_by_version(apis::crf::SearchCrfItemsByVersionRequest {
            version_id: form.version_id,
            fragment,
        })
        .await?;
    let items = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfItemListResponse { items }))
}

/// `GET /api/crf/items/{id}` — fetch a CRF item by id.
#[utoipa::path(
    get, path = "/items/{id}", tag = "crf",
    operation_id = "crf_get_item_by_id",
    params(
        ("id" = i32, Path, description = "CRF item id"),
    ),
    responses(
        (status = 200, description = "Item found", body = dto::CrfItemViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Item not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_item_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfItemViewResponse>, ApiError> {
    let view = state
        .crf
        .get_item_by_id(apis::crf::GetCrfItemByIdRequest { id })
        .await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/crf/items/{id}` — partial update of a CRF item.
#[utoipa::path(
    patch, path = "/items/{id}", tag = "crf",
    operation_id = "crf_update_item",
    params(
        ("id" = i32, Path, description = "CRF item id"),
    ),
    request_body = dto::UpdateCrfItemRequest,
    responses(
        (status = 200, description = "Item updated", body = dto::CrfItemViewResponse),
        (status = 400, description = "Validation failed / kind-shape violation", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Item not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_item(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::UpdateCrfItemRequest>,
) -> Result<Json<dto::CrfItemViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .update_item(apis::crf::UpdateCrfItemRequest {
            id,
            code: req.code,
            name: req.name,
            kind: req.kind.map(Into::into),
            order: req.order,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/crf/items/{id}` — hard delete a CRF item.
#[utoipa::path(
    delete, path = "/items/{id}", tag = "crf",
    operation_id = "crf_delete_item",
    params(
        ("id" = i32, Path, description = "CRF item id"),
    ),
    responses(
        (status = 204, description = "Item deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Item not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_item(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.crf.delete_item(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- CrfOption ----

/// `POST /api/crf/items/{item_id}/options` — create a CRF option
/// under an item.
#[utoipa::path(
    post, path = "/items/{item_id}/options", tag = "crf",
    operation_id = "crf_create_option",
    params(
        ("item_id" = i32, Path, description = "Owning CRF item id"),
    ),
    request_body = dto::CreateCrfOptionRequest,
    responses(
        (status = 201, description = "Option created", body = dto::CrfOptionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "CRF item not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_option(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id: item_id }): Path<CrfPathId>,
    Json(req): Json<dto::CreateCrfOptionRequest>,
) -> Result<(StatusCode, Json<dto::CrfOptionViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .create_option(apis::crf::CreateCrfOptionRequest {
            item_id,
            value: req.value,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/crf/items/{item_id}/options` — list every CRF option
/// under the given item, ordered by id ASC.
#[utoipa::path(
    get, path = "/items/{item_id}/options", tag = "crf",
    operation_id = "crf_list_options_by_item",
    params(
        ("item_id" = i32, Path, description = "Owning CRF item id"),
    ),
    responses(
        (status = 200, description = "Options list", body = dto::CrfOptionListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_options_by_item(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: item_id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfOptionListResponse>, ApiError> {
    let views = state
        .crf
        .list_options_by_item(apis::crf::ListCrfOptionsByItemRequest { item_id })
        .await?;
    let options = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfOptionListResponse { options }))
}

/// `GET /api/crf/items/{item_id}/options/search?fragment=...` —
/// version-scoped substring search on option value. The version is
/// derived from item -> form -> version at the usecase layer.
#[utoipa::path(
    get, path = "/items/{item_id}/options/search", tag = "crf",
    operation_id = "crf_search_options_by_version",
    params(
        ("item_id" = i32, Path, description = "Owning CRF item id; the version is derived from this row"),
        ("fragment" = String, Query, description = "Required non-empty text fragment"),
    ),
    responses(
        (status = 200, description = "Options list", body = dto::CrfOptionListResponse),
        (status = 400, description = "Empty search fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn search_options_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: item_id }): Path<CrfPathId>,
    Query(CrfFragmentQuery { fragment }): Query<CrfFragmentQuery>,
) -> Result<Json<dto::CrfOptionListResponse>, ApiError> {
    let item = state
        .crf
        .get_item_by_id(apis::crf::GetCrfItemByIdRequest { id: item_id })
        .await?;
    let form = state
        .crf
        .get_form_by_id(apis::crf::GetCrfFormByIdRequest { id: item.form_id })
        .await?;
    let views = state
        .crf
        .search_options_by_version(apis::crf::SearchCrfOptionsByVersionRequest {
            version_id: form.version_id,
            fragment,
        })
        .await?;
    let options = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfOptionListResponse { options }))
}

/// `GET /api/crf/options/{id}` — fetch a CRF option by id.
#[utoipa::path(
    get, path = "/options/{id}", tag = "crf",
    operation_id = "crf_get_option_by_id",
    params(
        ("id" = i32, Path, description = "CRF option id"),
    ),
    responses(
        (status = 200, description = "Option found", body = dto::CrfOptionViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Option not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_option_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfOptionViewResponse>, ApiError> {
    let view = state
        .crf
        .get_option_by_id(apis::crf::GetCrfOptionByIdRequest { id })
        .await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/crf/options/{id}` — partial update of a CRF option.
#[utoipa::path(
    patch, path = "/options/{id}", tag = "crf",
    operation_id = "crf_update_option",
    params(
        ("id" = i32, Path, description = "CRF option id"),
    ),
    request_body = dto::UpdateCrfOptionRequest,
    responses(
        (status = 200, description = "Option updated", body = dto::CrfOptionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Option not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_option(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::UpdateCrfOptionRequest>,
) -> Result<Json<dto::CrfOptionViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .update_option(apis::crf::UpdateCrfOptionRequest {
            id,
            value: req.value,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/crf/options/{id}` — hard delete a CRF option.
#[utoipa::path(
    delete, path = "/options/{id}", tag = "crf",
    operation_id = "crf_delete_option",
    params(
        ("id" = i32, Path, description = "CRF option id"),
    ),
    responses(
        (status = 204, description = "Option deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Option not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_option(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.crf.delete_option(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- CrfUnit ----

/// `POST /api/crf/items/{item_id}/units` — create a CRF unit under
/// an item.
#[utoipa::path(
    post, path = "/items/{item_id}/units", tag = "crf",
    operation_id = "crf_create_unit",
    params(
        ("item_id" = i32, Path, description = "Owning CRF item id"),
    ),
    request_body = dto::CreateCrfUnitRequest,
    responses(
        (status = 201, description = "Unit created", body = dto::CrfUnitViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "CRF item not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_unit(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id: item_id }): Path<CrfPathId>,
    Json(req): Json<dto::CreateCrfUnitRequest>,
) -> Result<(StatusCode, Json<dto::CrfUnitViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .create_unit(apis::crf::CreateCrfUnitRequest {
            item_id,
            value: req.value,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/crf/items/{item_id}/units` — list every CRF unit under
/// the given item, ordered by id ASC.
#[utoipa::path(
    get, path = "/items/{item_id}/units", tag = "crf",
    operation_id = "crf_list_units_by_item",
    params(
        ("item_id" = i32, Path, description = "Owning CRF item id"),
    ),
    responses(
        (status = 200, description = "Units list", body = dto::CrfUnitListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_units_by_item(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: item_id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfUnitListResponse>, ApiError> {
    let views = state
        .crf
        .list_units_by_item(apis::crf::ListCrfUnitsByItemRequest { item_id })
        .await?;
    let units = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfUnitListResponse { units }))
}

/// `GET /api/crf/items/{item_id}/units/search?fragment=...` —
/// version-scoped substring search on unit value. The version is
/// derived from item -> form -> version at the usecase layer.
#[utoipa::path(
    get, path = "/items/{item_id}/units/search", tag = "crf",
    operation_id = "crf_search_units_by_version",
    params(
        ("item_id" = i32, Path, description = "Owning CRF item id; the version is derived from this row"),
        ("fragment" = String, Query, description = "Required non-empty text fragment"),
    ),
    responses(
        (status = 200, description = "Units list", body = dto::CrfUnitListResponse),
        (status = 400, description = "Empty search fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn search_units_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: item_id }): Path<CrfPathId>,
    Query(CrfFragmentQuery { fragment }): Query<CrfFragmentQuery>,
) -> Result<Json<dto::CrfUnitListResponse>, ApiError> {
    let item = state
        .crf
        .get_item_by_id(apis::crf::GetCrfItemByIdRequest { id: item_id })
        .await?;
    let form = state
        .crf
        .get_form_by_id(apis::crf::GetCrfFormByIdRequest { id: item.form_id })
        .await?;
    let views = state
        .crf
        .search_units_by_version(apis::crf::SearchCrfUnitsByVersionRequest {
            version_id: form.version_id,
            fragment,
        })
        .await?;
    let units = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CrfUnitListResponse { units }))
}

/// `GET /api/crf/units/{id}` — fetch a CRF unit by id.
#[utoipa::path(
    get, path = "/units/{id}", tag = "crf",
    operation_id = "crf_get_unit_by_id",
    params(
        ("id" = i32, Path, description = "CRF unit id"),
    ),
    responses(
        (status = 200, description = "Unit found", body = dto::CrfUnitViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Unit not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_unit_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::CrfUnitViewResponse>, ApiError> {
    let view = state
        .crf
        .get_unit_by_id(apis::crf::GetCrfUnitByIdRequest { id })
        .await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/crf/units/{id}` — partial update of a CRF unit.
#[utoipa::path(
    patch, path = "/units/{id}", tag = "crf",
    operation_id = "crf_update_unit",
    params(
        ("id" = i32, Path, description = "CRF unit id"),
    ),
    request_body = dto::UpdateCrfUnitRequest,
    responses(
        (status = 200, description = "Unit updated", body = dto::CrfUnitViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Unit not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_unit(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::UpdateCrfUnitRequest>,
) -> Result<Json<dto::CrfUnitViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .update_unit(apis::crf::UpdateCrfUnitRequest {
            id,
            value: req.value,
            not_submitted: req.not_submitted,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/crf/units/{id}` — hard delete a CRF unit.
#[utoipa::path(
    delete, path = "/units/{id}", tag = "crf",
    operation_id = "crf_delete_unit",
    params(
        ("id" = i32, Path, description = "CRF unit id"),
    ),
    responses(
        (status = 204, description = "Unit deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Unit not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_unit(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.crf.delete_unit(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- DomainAnnotation ----

/// `POST /api/crf/forms/{form_id}/domain-annotations` — create a
/// domain annotation under a form.
#[utoipa::path(
    post, path = "/forms/{form_id}/domain-annotations", tag = "crf",
    operation_id = "crf_create_domain_annotation",
    params(
        ("form_id" = i32, Path, description = "Owning CRF form id"),
    ),
    request_body = dto::CreateDomainAnnotationRequest,
    responses(
        (status = 201, description = "Domain annotation created", body = dto::DomainAnnotationViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "CRF form not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate domain annotation", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_domain_annotation(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id: form_id }): Path<CrfPathId>,
    Json(req): Json<dto::CreateDomainAnnotationRequest>,
) -> Result<(StatusCode, Json<dto::DomainAnnotationViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .create_domain_annotation(apis::crf::CreateDomainAnnotationRequest {
            form_id,
            name: req.name,
            description: req.description,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/crf/forms/{form_id}/domain-annotations` — list every
/// domain annotation attached to the given form.
#[utoipa::path(
    get, path = "/forms/{form_id}/domain-annotations", tag = "crf",
    operation_id = "crf_list_domain_annotations_by_form",
    params(
        ("form_id" = i32, Path, description = "Owning CRF form id"),
    ),
    responses(
        (status = 200, description = "Domain annotations list", body = dto::DomainAnnotationListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_domain_annotations_by_form(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: form_id }): Path<CrfPathId>,
) -> Result<Json<dto::DomainAnnotationListResponse>, ApiError> {
    let views = state
        .crf
        .list_domain_annotations_by_form(apis::crf::ListDomainAnnotationsByFormRequest { form_id })
        .await?;
    let domain_annotations = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::DomainAnnotationListResponse {
        domain_annotations,
    }))
}

/// `GET /api/crf/versions/{version_id}/domain-annotations/search?fragment=...` —
/// version-scoped substring search on domain annotation name /
/// description. The search walks every form under the version.
#[utoipa::path(
    get, path = "/versions/{version_id}/domain-annotations/search", tag = "crf",
    operation_id = "crf_search_domain_annotations_by_version",
    params(
        ("version_id" = i32, Path, description = "Owning CRF version id"),
        ("fragment" = String, Query, description = "Required non-empty text fragment"),
    ),
    responses(
        (status = 200, description = "Domain annotations list", body = dto::DomainAnnotationListResponse),
        (status = 400, description = "Empty search fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn search_domain_annotations_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: version_id }): Path<CrfPathId>,
    Query(CrfFragmentQuery { fragment }): Query<CrfFragmentQuery>,
) -> Result<Json<dto::DomainAnnotationListResponse>, ApiError> {
    let views = state
        .crf
        .search_domain_annotations_by_version(apis::crf::SearchDomainAnnotationsByVersionRequest {
            version_id,
            fragment,
        })
        .await?;
    let domain_annotations = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::DomainAnnotationListResponse {
        domain_annotations,
    }))
}

/// `GET /api/crf/domain-annotations/{id}` — fetch a domain annotation by id.
#[utoipa::path(
    get, path = "/domain-annotations/{id}", tag = "crf",
    operation_id = "crf_get_domain_annotation_by_id",
    params(
        ("id" = i32, Path, description = "Domain annotation id"),
    ),
    responses(
        (status = 200, description = "Domain annotation found", body = dto::DomainAnnotationViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain annotation not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_domain_annotation_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::DomainAnnotationViewResponse>, ApiError> {
    let view = state
        .crf
        .get_domain_annotation_by_id(apis::crf::GetDomainAnnotationByIdRequest { id })
        .await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/crf/domain-annotations/{id}` — partial update of a
/// domain annotation.
#[utoipa::path(
    patch, path = "/domain-annotations/{id}", tag = "crf",
    operation_id = "crf_update_domain_annotation",
    params(
        ("id" = i32, Path, description = "Domain annotation id"),
    ),
    request_body = dto::UpdateDomainAnnotationRequest,
    responses(
        (status = 200, description = "Domain annotation updated", body = dto::DomainAnnotationViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain annotation not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate domain annotation", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_domain_annotation(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::UpdateDomainAnnotationRequest>,
) -> Result<Json<dto::DomainAnnotationViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .update_domain_annotation(apis::crf::UpdateDomainAnnotationRequest {
            id,
            name: req.name,
            description: req.description,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/crf/domain-annotations/{id}` — hard delete a domain
/// annotation.
#[utoipa::path(
    delete, path = "/domain-annotations/{id}", tag = "crf",
    operation_id = "crf_delete_domain_annotation",
    params(
        ("id" = i32, Path, description = "Domain annotation id"),
    ),
    responses(
        (status = 204, description = "Domain annotation deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Domain annotation not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_domain_annotation(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.crf.delete_domain_annotation(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- Annotation ----

/// `POST /api/crf/annotations` — create an annotation. The
/// polymorphic owner lives in the request body.
#[utoipa::path(
    post, path = "/annotations", tag = "crf",
    operation_id = "crf_create_annotation",
    request_body = dto::CreateAnnotationRequest,
    responses(
        (status = 201, description = "Annotation created", body = dto::AnnotationViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Owner or domain annotation not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_annotation(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateAnnotationRequest>,
) -> Result<(StatusCode, Json<dto::AnnotationViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .create_annotation(apis::crf::CreateAnnotationRequest {
            domain_annotation_id: req.domain_annotation_id,
            content: req.content,
            assign: req.assign,
            owner: req.owner.into(),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/crf/forms/{form_id}/annotations` — list every annotation
/// attached directly to the given form.
#[utoipa::path(
    get, path = "/forms/{form_id}/annotations", tag = "crf",
    operation_id = "crf_list_annotations_by_form",
    params(
        ("form_id" = i32, Path, description = "Owning CRF form id"),
    ),
    responses(
        (status = 200, description = "Annotations list", body = dto::AnnotationListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_annotations_by_form(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: form_id }): Path<CrfPathId>,
) -> Result<Json<dto::AnnotationListResponse>, ApiError> {
    let views = state
        .crf
        .list_annotations_by_form(apis::crf::ListAnnotationsByFormRequest { form_id })
        .await?;
    let annotations = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::AnnotationListResponse { annotations }))
}

/// `GET /api/crf/items/{item_id}/annotations` — list every annotation
/// attached directly to the given item.
#[utoipa::path(
    get, path = "/items/{item_id}/annotations", tag = "crf",
    operation_id = "crf_list_annotations_by_item",
    params(
        ("item_id" = i32, Path, description = "Owning CRF item id"),
    ),
    responses(
        (status = 200, description = "Annotations list", body = dto::AnnotationListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_annotations_by_item(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: item_id }): Path<CrfPathId>,
) -> Result<Json<dto::AnnotationListResponse>, ApiError> {
    let views = state
        .crf
        .list_annotations_by_item(apis::crf::ListAnnotationsByItemRequest { item_id })
        .await?;
    let annotations = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::AnnotationListResponse { annotations }))
}

/// `GET /api/crf/options/{option_id}/annotations` — list every
/// annotation attached directly to the given option.
#[utoipa::path(
    get, path = "/options/{option_id}/annotations", tag = "crf",
    operation_id = "crf_list_annotations_by_option",
    params(
        ("option_id" = i32, Path, description = "Owning CRF option id"),
    ),
    responses(
        (status = 200, description = "Annotations list", body = dto::AnnotationListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_annotations_by_option(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: option_id }): Path<CrfPathId>,
) -> Result<Json<dto::AnnotationListResponse>, ApiError> {
    let views = state
        .crf
        .list_annotations_by_option(apis::crf::ListAnnotationsByOptionRequest { option_id })
        .await?;
    let annotations = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::AnnotationListResponse { annotations }))
}

/// `GET /api/crf/units/{unit_id}/annotations` — list every annotation
/// attached directly to the given unit.
#[utoipa::path(
    get, path = "/units/{unit_id}/annotations", tag = "crf",
    operation_id = "crf_list_annotations_by_unit",
    params(
        ("unit_id" = i32, Path, description = "Owning CRF unit id"),
    ),
    responses(
        (status = 200, description = "Annotations list", body = dto::AnnotationListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_annotations_by_unit(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: unit_id }): Path<CrfPathId>,
) -> Result<Json<dto::AnnotationListResponse>, ApiError> {
    let views = state
        .crf
        .list_annotations_by_unit(apis::crf::ListAnnotationsByUnitRequest { unit_id })
        .await?;
    let annotations = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::AnnotationListResponse { annotations }))
}

/// `GET /api/crf/versions/{version_id}/annotations/search?fragment=...` —
/// version-scoped substring search on annotation content. The search
/// UNIONs every annotation chain under the version.
#[utoipa::path(
    get, path = "/versions/{version_id}/annotations/search", tag = "crf",
    operation_id = "crf_search_annotations_by_version",
    params(
        ("version_id" = i32, Path, description = "Owning CRF version id"),
        ("fragment" = String, Query, description = "Required non-empty text fragment"),
    ),
    responses(
        (status = 200, description = "Annotations list", body = dto::AnnotationListResponse),
        (status = 400, description = "Empty search fragment", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn search_annotations_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id: version_id }): Path<CrfPathId>,
    Query(CrfFragmentQuery { fragment }): Query<CrfFragmentQuery>,
) -> Result<Json<dto::AnnotationListResponse>, ApiError> {
    let views = state
        .crf
        .search_annotations_by_version(apis::crf::SearchAnnotationsByVersionRequest {
            version_id,
            fragment,
        })
        .await?;
    let annotations = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::AnnotationListResponse { annotations }))
}

/// `GET /api/crf/annotations/{id}` — fetch an annotation by id.
#[utoipa::path(
    get, path = "/annotations/{id}", tag = "crf",
    operation_id = "crf_get_annotation_by_id",
    params(
        ("id" = i32, Path, description = "Annotation id"),
    ),
    responses(
        (status = 200, description = "Annotation found", body = dto::AnnotationViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Annotation not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_annotation_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<Json<dto::AnnotationViewResponse>, ApiError> {
    let view = state
        .crf
        .get_annotation_by_id(apis::crf::GetAnnotationByIdRequest { id })
        .await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/crf/annotations/{id}` — partial update of an
/// annotation. The owner is fixed at create time and not
/// patchable.
#[utoipa::path(
    patch, path = "/annotations/{id}", tag = "crf",
    operation_id = "crf_update_annotation",
    params(
        ("id" = i32, Path, description = "Annotation id"),
    ),
    request_body = dto::UpdateAnnotationRequest,
    responses(
        (status = 200, description = "Annotation updated", body = dto::AnnotationViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Annotation not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_annotation(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
    Json(req): Json<dto::UpdateAnnotationRequest>,
) -> Result<Json<dto::AnnotationViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .crf
        .update_annotation(apis::crf::UpdateAnnotationRequest {
            id,
            content: req.content,
            assign: req.assign,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/crf/annotations/{id}` — hard delete an annotation.
#[utoipa::path(
    delete, path = "/annotations/{id}", tag = "crf",
    operation_id = "crf_delete_annotation",
    params(
        ("id" = i32, Path, description = "Annotation id"),
    ),
    responses(
        (status = 204, description = "Annotation deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Annotation not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_annotation(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(CrfPathId { id }): Path<CrfPathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.crf.delete_annotation(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

//! HTTP handlers for the terminology namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from `dto`) into an apis DTO.
//! 2. Calls the corresponding [`TerminologyService`](apis::terminology::TerminologyService)
//!    method on `AppState`.
//! 3. Translates the apis response back into a wire DTO.
//!
//! `TerminologyApiError` is funnelled through [`ApiError::from`] so
//! each route returns `Result<Json<T>, ApiError>` and the error
//! mapping in `transport::http::error` does the rest.
//!
//! The role policy lives in
//! [`crate::transport::http::auth::middleware::require_admin_or_root`];
//! every write handler (POST / PATCH / DELETE) calls it before
//! dispatching to the usecase, matching the project module's policy.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::{AuthClaims, require_admin_or_root};
use crate::transport::http::dto::{
    self, CodeItemByVersionAndCodeQuery, CodeItemListQuery, CodeListListQuery,
};
use crate::transport::http::error::ApiError;

// ---- TerminologyVersion ----

/// `POST /api/terminology/versions` — create a terminology version.
#[utoipa::path(
    post, path = "/versions", tag = "terminology",
    operation_id = "terminology_create_version",
    request_body = dto::CreateTerminologyVersionRequest,
    responses(
        (status = 201, description = "Version created", body = dto::TerminologyVersionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate terminology version", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateTerminologyVersionRequest>,
) -> Result<(StatusCode, Json<dto::TerminologyVersionViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .terminology
        .create_version(apis::terminology::CreateTerminologyVersionRequest {
            kind: req.kind.into(),
            name: req.name,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/terminology/versions` — list terminology versions.
#[utoipa::path(
    get, path = "/versions", tag = "terminology",
    operation_id = "terminology_list_versions",
    responses(
        (status = 200, description = "Versions list", body = dto::TerminologyVersionListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_versions(
    State(state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<dto::TerminologyVersionListResponse>, ApiError> {
    let views = state.terminology.list_versions().await?;
    let versions = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::TerminologyVersionListResponse { versions }))
}

/// `GET /api/terminology/versions/{id}` — fetch a terminology version by id.
#[utoipa::path(
    get, path = "/versions/{id}", tag = "terminology",
    operation_id = "terminology_get_version_by_id",
    params(
        ("id" = i64, Path, description = "Terminology version id"),
    ),
    responses(
        (status = 200, description = "Version found", body = dto::TerminologyVersionViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_version_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::TerminologyVersionViewResponse>, ApiError> {
    let view = state.terminology.get_version_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/terminology/versions/{id}` — partial update of a
/// terminology version.
#[utoipa::path(
    patch, path = "/versions/{id}", tag = "terminology",
    operation_id = "terminology_update_version",
    params(
        ("id" = i64, Path, description = "Terminology version id"),
    ),
    request_body = dto::UpdateTerminologyVersionRequest,
    responses(
        (status = 200, description = "Version updated", body = dto::TerminologyVersionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate terminology version", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateTerminologyVersionRequest>,
) -> Result<Json<dto::TerminologyVersionViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .terminology
        .update_version(apis::terminology::UpdateTerminologyVersionRequest {
            id,
            kind: req.kind.map(Into::into),
            name: req.name,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/terminology/versions/{id}` — hard delete a version.
#[utoipa::path(
    delete, path = "/versions/{id}", tag = "terminology",
    operation_id = "terminology_delete_version",
    params(
        ("id" = i64, Path, description = "Terminology version id"),
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
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.terminology.delete_version(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- CodeList ----

/// `POST /api/terminology/code-lists` — create a codelist.
#[utoipa::path(
    post, path = "/code-lists", tag = "terminology",
    operation_id = "terminology_create_code_list",
    request_body = dto::CreateCodeListRequest,
    responses(
        (status = 201, description = "Codelist created", body = dto::CodeListViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Version not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate codelist", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_code_list(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateCodeListRequest>,
) -> Result<(StatusCode, Json<dto::CodeListViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .terminology
        .create_code_list(apis::terminology::CreateCodeListRequest {
            version_id: req.version_id,
            code: req.code,
            extensible: req.extensible,
            name: req.name,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/terminology/code-lists?…` — unified list+search for
/// codelists under a version. `fragment = Some(_)` runs the FTS
/// prefix-match path; `fragment = None` (or absent) returns a plain
/// `ORDER BY id ASC` list. `offset` / `limit` paginate the result.
#[utoipa::path(
    get, path = "/code-lists", tag = "terminology",
    operation_id = "terminology_list_code_lists",
    params(
        ("versionId" = i64, Query, description = "Owning terminology version id"),
        ("fragment" = Option<String>, Query, description = "Text fragment for FTS prefix-match (omit or empty for plain list)"),
        ("offset" = Option<u32>, Query, description = "Page offset (0-based, default 0)"),
        ("limit" = Option<u32>, Query, description = "Page size (0 = server default 50, max 500)"),
    ),
    responses(
        (status = 200, description = "Paged codelists list", body = dto::PagedCodeListListResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = []),
    ),
)]
pub async fn list_code_lists(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(CodeListListQuery {
        version_id,
        fragment,
        offset,
        limit,
    }): Query<CodeListListQuery>,
) -> Result<Json<dto::PagedCodeListListResponse>, ApiError> {
    let page = state
        .terminology
        .list_code_lists(apis::terminology::CodeListListQuery {
            version_id,
            fragment,
            offset,
            limit,
        })
        .await?;
    Ok(Json(page.into()))
}

/// `GET /api/terminology/code-lists/{id}` — fetch a codelist by id.
#[utoipa::path(
    get, path = "/code-lists/{id}", tag = "terminology",
    operation_id = "terminology_get_code_list_by_id",
    params(
        ("id" = i64, Path, description = "Codelist id"),
    ),
    responses(
        (status = 200, description = "Codelist found", body = dto::CodeListViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Codelist not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_code_list_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<Json<dto::CodeListViewResponse>, ApiError> {
    let view = state.terminology.get_code_list_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `PATCH /api/terminology/code-lists/{id}` — partial update of a
/// codelist.
#[utoipa::path(
    patch, path = "/code-lists/{id}", tag = "terminology",
    operation_id = "terminology_update_code_list",
    params(
        ("id" = i64, Path, description = "Codelist id"),
    ),
    request_body = dto::UpdateCodeListRequest,
    responses(
        (status = 200, description = "Codelist updated", body = dto::CodeListViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Codelist not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate codelist", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_code_list(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateCodeListRequest>,
) -> Result<Json<dto::CodeListViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .terminology
        .update_code_list(apis::terminology::UpdateCodeListRequest {
            id,
            code: req.code,
            extensible: req.extensible,
            name: req.name,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/terminology/code-lists/{id}` — hard delete a codelist.
#[utoipa::path(
    delete, path = "/code-lists/{id}", tag = "terminology",
    operation_id = "terminology_delete_code_list",
    params(
        ("id" = i64, Path, description = "Codelist id"),
    ),
    responses(
        (status = 204, description = "Codelist deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Codelist not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_code_list(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.terminology.delete_code_list(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/terminology/code-lists/search?…` — removed; unified into
/// `GET /api/terminology/code-lists` (see [`list_code_lists`]).
/// This stub is intentionally absent from the router.

// ---- CodeItem ----

/// `POST /api/terminology/code-items` — create a code item.
#[utoipa::path(
    post, path = "/code-items", tag = "terminology",
    operation_id = "terminology_create_code_item",
    request_body = dto::CreateCodeItemRequest,
    responses(
        (status = 201, description = "Item created", body = dto::CodeItemViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Codelist not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate code item", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_code_item(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateCodeItemRequest>,
) -> Result<(StatusCode, Json<dto::CodeItemViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .terminology
        .create_code_item(apis::terminology::CreateCodeItemRequest {
            codelist_id: req.codelist_id,
            version_id: req.version_id,
            code: req.code,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `POST /api/terminology/code-items/batch` — create several code items
/// in a single SQL statement.
#[utoipa::path(
    post, path = "/code-items/batch", tag = "terminology",
    operation_id = "terminology_batch_create_code_items",
    request_body = dto::BatchCreateCodeItemsRequest,
    responses(
        (status = 201, description = "Items created", body = dto::BatchCreateCodeItemsResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn batch_create_code_items(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::BatchCreateCodeItemsRequest>,
) -> Result<(StatusCode, Json<dto::BatchCreateCodeItemsResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let resp = state
        .terminology
        .batch_create_code_items(apis::terminology::BatchCreateCodeItemsRequest {
            codelist_id: req.codelist_id,
            version_id: req.version_id,
            items: req
                .items
                .into_iter()
                .map(|e| apis::terminology::BatchCodeItemEntry {
                    code: e.code,
                    submission_value: e.submission_value,
                    synonym: e.synonym,
                    definition: e.definition,
                    nci_preferred_term: e.nci_preferred_term,
                })
                .collect(),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(dto::BatchCreateCodeItemsResponse {
        count: resp.count,
        codelist_id: resp.codelist_id,
        version_id: resp.version_id,
    })))
}

/// `GET /api/terminology/code-items?…` — unified list+search for
/// code items under a codelist. `fragment = Some(_)` runs the FTS
/// prefix-match path; `fragment = None` (or absent) returns a plain
/// `ORDER BY id ASC` list. `offset` / `limit` paginate the result.
#[utoipa::path(
    get, path = "/code-items", tag = "terminology",
    operation_id = "terminology_list_code_items",
    params(
        ("codelistId" = i64, Query, description = "Owning codelist id"),
        ("fragment" = Option<String>, Query, description = "Text fragment for FTS prefix-match (omit or empty for plain list)"),
        ("offset" = Option<u32>, Query, description = "Page offset (0-based, default 0)"),
        ("limit" = Option<u32>, Query, description = "Page size (0 = server default 50, max 500)"),
    ),
    responses(
        (status = 200, description = "Paged code items list", body = dto::PagedCodeItemListResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_code_items(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(CodeItemListQuery {
        codelist_id,
        fragment,
        offset,
        limit,
    }): Query<CodeItemListQuery>,
) -> Result<Json<dto::PagedCodeItemListResponse>, ApiError> {
    let page = state
        .terminology
        .list_code_items(apis::terminology::CodeItemListQuery {
            codelist_id,
            fragment,
            offset,
            limit,
        })
        .await?;
    Ok(Json(page.into()))
}

/// `GET /api/terminology/code-items/by-version-and-code?…` — natural-key
/// lookup on the `code_items` table.
#[utoipa::path(
    get, path = "/code-items/by-version-and-code", tag = "terminology",
    operation_id = "terminology_list_code_items_by_version_and_code",
    params(
        ("versionId" = i64, Query, description = "Owning terminology version id"),
        ("code" = String, Query, description = "Item value code"),
    ),
    responses(
        (status = 200, description = "Code items list", body = dto::CodeItemListResponse),
        (status = 400, description = "Empty code supplied", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_code_items_by_version_and_code(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Query(q): Query<CodeItemByVersionAndCodeQuery>,
) -> Result<Json<dto::CodeItemListResponse>, ApiError> {
    let views = state
        .terminology
        .list_code_items_by_version_and_code(q.version_id, &q.code)
        .await?;
    let items = views.into_iter().map(Into::into).collect();
    Ok(Json(dto::CodeItemListResponse { items }))
}

/// `PATCH /api/terminology/code-items/{id}` — partial update of a
/// code item.
#[utoipa::path(
    patch, path = "/code-items/{id}", tag = "terminology",
    operation_id = "terminology_update_code_item",
    params(
        ("id" = i64, Path, description = "Code item id"),
    ),
    request_body = dto::UpdateCodeItemRequest,
    responses(
        (status = 200, description = "Item updated", body = dto::CodeItemViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Admin or root required", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Item not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_code_item(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
    Json(req): Json<dto::UpdateCodeItemRequest>,
) -> Result<Json<dto::CodeItemViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .terminology
        .update_code_item(apis::terminology::UpdateCodeItemRequest {
            id,
            code: req.code,
            submission_value: req.submission_value,
            synonym: req.synonym,
            definition: req.definition,
            nci_preferred_term: req.nci_preferred_term,
        })
        .await?;
    Ok(Json(view.into()))
}

/// `DELETE /api/terminology/code-items/{id}` — hard delete a code item.
#[utoipa::path(
    delete, path = "/code-items/{id}", tag = "terminology",
    operation_id = "terminology_delete_code_item",
    params(
        ("id" = i64, Path, description = "Code item id"),
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
pub async fn delete_code_item(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(dto::PathId { id }): Path<dto::PathId>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.terminology.delete_code_item(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/terminology/code-items/search?…` — removed; unified into
/// `GET /api/terminology/code-items` (see [`list_code_items`]).

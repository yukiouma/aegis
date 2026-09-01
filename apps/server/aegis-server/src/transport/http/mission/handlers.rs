//! HTTP handlers for the mission namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from `dto`) into an apis DTO.
//! 2. Calls the corresponding [`apis::mission::MissionService`] method
//!    on `AppState`.
//! 3. Translates the apis response back into a wire DTO.
//!
//! `MissionApiError` is funnelled through [`ApiError::from`] so each
//! route returns `Result<_, ApiError>` and the error mapping in
//! `transport::http::error` does the rest.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use apis::mission::{
    Actor, AssigneeData, CreateMissionRequest, ListMissionsByProjectRequest,
    ListMissionsByUserRequest,
};

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto;
use crate::transport::http::error::ApiError;

fn to_actor(claims: &AuthClaims) -> Actor {
    Actor {
        user_code: claims.0.code.clone(),
    }
}

fn assignee_data(d: dto::AssigneeDataRequest) -> AssigneeData {
    AssigneeData {
        user_code: d.user_code,
        role: d.role.into(),
    }
}

/// `POST /api/mission` — create a mission.
#[utoipa::path(
    post, path = "", tag = "mission",
    operation_id = "mission_create",
    request_body = dto::CreateMissionRequest,
    responses(
        (status = 201, description = "mission created", body = dto::MissionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the project's leader set", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project / user not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Mission / assignee already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_mission(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateMissionRequest>,
) -> Result<(StatusCode, Json<dto::MissionViewResponse>), ApiError> {
    let view = state
        .mission
        .create_mission(
            &to_actor(&claims),
            CreateMissionRequest {
                project_code: req.project_code,
                mission_kind: req.mission_kind.into(),
                mission_code: req.mission_code,
                assignees: req.assignees.into_iter().map(assignee_data).collect(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/mission/{id}` — fetch a mission by id.
#[utoipa::path(
    get, path = "/{id}", tag = "mission",
    operation_id = "mission_get_by_id",
    params(
        ("id" = i64, Path, description = "Mission id"),
    ),
    responses(
        (status = 200, description = "mission found", body = dto::MissionViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_mission_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(id): Path<i64>,
) -> Result<Json<dto::MissionViewResponse>, ApiError> {
    let view = state.mission.get_mission_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `GET /api/mission/by-project/{project_code}` — list missions for a
/// project. Optional `?kind=crf|sdtm|adam|tfl` filter.
#[utoipa::path(
    get, path = "/by-project/{project_code}", tag = "mission",
    operation_id = "mission_list_by_project",
    params(
        ("project_code" = String, Path, description = "Project code"),
        ("kind" = Option<String>, Query, description = "Filter by mission kind"),
    ),
    responses(
        (status = 200, description = "missions list", body = dto::MissionListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_missions_by_project(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(project_code): Path<String>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<dto::MissionListResponse>, ApiError> {
    let kind = q
        .get("kind")
        .map(|s| s.parse::<apis::mission::MissionKind>())
        .transpose()
        .map_err(|e| {
            ApiError::Mission(apis::mission::MissionApiError::Validation(e.to_string()))
        })?;
    let views = state
        .mission
        .list_missions_by_project(ListMissionsByProjectRequest { project_code, kind })
        .await?;
    Ok(Json(dto::MissionListResponse {
        missions: views.into_iter().map(Into::into).collect(),
    }))
}

/// `GET /api/mission/by-user/{user_code}` — list missions the user
/// appears on (across roles).
#[utoipa::path(
    get, path = "/by-user/{user_code}", tag = "mission",
    operation_id = "mission_list_by_user",
    params(
        ("user_code" = String, Path, description = "User code"),
    ),
    responses(
        (status = 200, description = "missions list", body = dto::MissionListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_missions_by_user(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(user_code): Path<String>,
) -> Result<Json<dto::MissionListResponse>, ApiError> {
    let views = state
        .mission
        .list_missions_by_user(ListMissionsByUserRequest { user_code })
        .await?;
    Ok(Json(dto::MissionListResponse {
        missions: views.into_iter().map(Into::into).collect(),
    }))
}

/// `DELETE /api/mission/{id}` — hard delete; cascades to assignees.
#[utoipa::path(
    delete, path = "/{id}", tag = "mission",
    operation_id = "mission_delete",
    params(
        ("id" = i64, Path, description = "Mission id"),
    ),
    responses(
        (status = 204, description = "mission deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the mission's project", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_mission(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state.mission.delete_mission(&to_actor(&claims), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/mission/{mission_id}/assignee` — add an assignee.
#[utoipa::path(
    post, path = "/{mission_id}/assignee", tag = "mission",
    operation_id = "mission_add_assignee",
    params(
        ("mission_id" = i64, Path, description = "Mission id"),
    ),
    request_body = dto::AssigneeDataRequest,
    responses(
        (status = 201, description = "assignee added", body = dto::AssigneeViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the mission's project", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission / user not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Assignee already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn add_assignee(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(mission_id): Path<i64>,
    Json(req): Json<dto::AssigneeDataRequest>,
) -> Result<(StatusCode, Json<dto::AssigneeViewResponse>), ApiError> {
    let view = state
        .mission
        .add_assignee(&to_actor(&claims), mission_id, assignee_data(req))
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `DELETE /api/mission/{mission_id}/assignee/{assignee_id}` — remove an
/// assignee.
#[utoipa::path(
    delete, path = "/{mission_id}/assignee/{assignee_id}", tag = "mission",
    operation_id = "mission_remove_assignee",
    params(
        ("mission_id" = i64, Path, description = "Mission id"),
        ("assignee_id" = i64, Path, description = "Assignee id"),
    ),
    responses(
        (status = 204, description = "assignee removed"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the mission's project", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission / assignee not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn remove_assignee(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path((mission_id, assignee_id)): Path<(i64, i64)>,
) -> Result<StatusCode, ApiError> {
    state
        .mission
        .remove_assignee(&to_actor(&claims), mission_id, assignee_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

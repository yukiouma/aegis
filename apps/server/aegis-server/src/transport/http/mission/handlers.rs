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
    Actor, AssigneeData, CloseIssueRequest, CreateIssueRequest, CreateMissionRequest,
    ListIssuesByMissionRequest, ListMissionsByProjectRequest, ListMissionsByUserRequest,
    ReopenIssueRequest, UpdateIssueDescriptionRequest,
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

// ===========================================================================
// Mission-issue routes (new)
//
// Each route maps a single `MissionService` issue method onto a
// `/api/mission/by-mission/{mission_id}/issues/...` or
// `/api/mission/issues/{issue_id}/...` URL. State changes (close /
// reopen) accept an optional `?state=opened|closed` query so the same
// PATCH path serves both directions.
// ===========================================================================

/// `GET /api/mission/by-mission/{mission_id}/issues` — list issues for
/// a mission. Optional `?state=opened|closed` filter.
#[utoipa::path(
    get, path = "/by-mission/{mission_id}/issues", tag = "mission",
    operation_id = "mission_list_issues_by_mission",
    params(
        ("mission_id" = i64, Path, description = "Mission id"),
        ("state" = Option<String>, Query, description = "Filter by issue state"),
    ),
    responses(
        (status = 200, description = "issues list", body = dto::IssueListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_issues_by_mission(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(mission_id): Path<i64>,
    axum::extract::Query(q): axum::extract::Query<dto::IssueListQuery>,
) -> Result<Json<dto::IssueListResponse>, ApiError> {
    let state_filter = q
        .state
        .map(|s| match s.as_str() {
            "opened" => Ok(apis::mission::IssueState::Opened),
            "closed" => Ok(apis::mission::IssueState::Closed),
            other => Err(ApiError::Mission(
                apis::mission::MissionApiError::Validation(format!(
                    "unknown issue state: {}",
                    other
                )),
            )),
        })
        .transpose()?;
    let views = state
        .mission
        .list_issues_by_mission(ListIssuesByMissionRequest {
            mission_id,
            state: state_filter,
        })
        .await?;
    Ok(Json(dto::IssueListResponse {
        issues: views.into_iter().map(Into::into).collect(),
    }))
}

/// `POST /api/mission/by-mission/{mission_id}/issues` — open a new
/// issue against a mission. Caller must be the project's leader or
/// a mission QC.
#[utoipa::path(
    post, path = "/by-mission/{mission_id}/issues", tag = "mission",
    operation_id = "mission_create_issue",
    params(
        ("mission_id" = i64, Path, description = "Mission id"),
    ),
    request_body = dto::CreateIssueRequest,
    responses(
        (status = 201, description = "issue created", body = dto::IssueViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the mission's project or a mission QC", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_issue(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(mission_id): Path<i64>,
    Json(req): Json<dto::CreateIssueRequest>,
) -> Result<(StatusCode, Json<dto::IssueViewResponse>), ApiError> {
    let view = state
        .mission
        .create_issue(
            &to_actor(&claims),
            CreateIssueRequest {
                mission_id,
                target_item: req.target_item,
                description: req.description,
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `PATCH /api/mission/issues/{issue_id}/state?state=closed|opened` —
/// flip an issue's open/closed state. Empty body. The
/// `?state=closed` query selects the close path; `?state=opened`
/// selects the reopen path. Anything else surfaces as `400`.
#[utoipa::path(
    patch, path = "/issues/{issue_id}/state", tag = "mission",
    operation_id = "mission_patch_issue_state",
    params(
        ("issue_id" = i64, Path, description = "Issue id"),
        ("state" = String, Query, description = "Target state — `opened` or `closed`"),
    ),
    request_body = dto::PatchIssueStateRequest,
    responses(
        (status = 200, description = "issue updated", body = dto::IssueViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not authorised to flip state on this issue", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Issue not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn patch_issue_state(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(issue_id): Path<i64>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<dto::IssueViewResponse>, ApiError> {
    let target = q.get("state").map(|s| s.as_str()).ok_or_else(|| {
        ApiError::Mission(apis::mission::MissionApiError::Validation(
            "missing `state` query parameter".into(),
        ))
    })?;
    let view = match target {
        "closed" => {
            state
                .mission
                .close_issue(&to_actor(&claims), CloseIssueRequest::default(), issue_id)
                .await?
        }
        "opened" => {
            state
                .mission
                .reopen_issue(&to_actor(&claims), ReopenIssueRequest::default(), issue_id)
                .await?
        }
        other => {
            return Err(ApiError::Mission(
                apis::mission::MissionApiError::Validation(format!(
                    "unknown issue state: {}",
                    other
                )),
            ));
        }
    };
    Ok(Json(view.into()))
}

/// `PATCH /api/mission/issues/{issue_id}/description` — replace the
/// issue's description. Allowed on issues in any state.
#[utoipa::path(
    patch, path = "/issues/{issue_id}/description", tag = "mission",
    operation_id = "mission_update_issue_description",
    params(
        ("issue_id" = i64, Path, description = "Issue id"),
    ),
    request_body = dto::UpdateIssueDescriptionRequest,
    responses(
        (status = 200, description = "issue updated", body = dto::IssueViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not authorised to update this issue", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Issue not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update_issue_description(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(issue_id): Path<i64>,
    Json(req): Json<dto::UpdateIssueDescriptionRequest>,
) -> Result<Json<dto::IssueViewResponse>, ApiError> {
    let view = state
        .mission
        .update_issue_description(
            &to_actor(&claims),
            issue_id,
            UpdateIssueDescriptionRequest {
                description: req.description,
            },
        )
        .await?;
    Ok(Json(view.into()))
}

/// `POST /api/mission/issues/{issue_id}/comments` — append a comment
/// to an issue's thread.
#[utoipa::path(
    post, path = "/issues/{issue_id}/comments", tag = "mission",
    operation_id = "mission_append_comment",
    params(
        ("issue_id" = i64, Path, description = "Issue id"),
    ),
    request_body = dto::AppendCommentRequest,
    responses(
        (status = 201, description = "comment appended", body = dto::IssueViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not authorised to comment on this issue", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Issue not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn append_comment(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(issue_id): Path<i64>,
    Json(req): Json<dto::AppendCommentRequest>,
) -> Result<(StatusCode, Json<dto::IssueViewResponse>), ApiError> {
    let view = state
        .mission
        .append_comment(
            &to_actor(&claims),
            issue_id,
            apis::mission::AppendCommentRequest {
                content: req.content,
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

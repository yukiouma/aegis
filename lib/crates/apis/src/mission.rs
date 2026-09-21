//! Outbound port for the mission service.
//!
//! Mirrors the surface of `mission::usecase::MissionUsecase` so
//! adapters in any backend (in-memory, PostgreSQL, …) can adapt
//! their own types to the shared contract defined here. All
//! supporting DTOs (request shapes, view projections, enums, and
//! [`MissionApiError`]) live alongside the trait so a single
//! `use apis::mission::*;` brings the whole contract into scope.

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Mission flavour — what kind of clinical-programming work the
/// mission is for.
///
/// Mirrors `mission::domain::MissionKind`. The two enums are kept
/// in sync layer by layer — adapter implementations convert
/// losslessly via the matching `From` impls in the mission crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

impl FromStr for MissionKind {
    type Err = MissionApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "crf" => Ok(MissionKind::Crf),
            "sdtm" => Ok(MissionKind::Sdtm),
            "adam" => Ok(MissionKind::Adam),
            "tfl" => Ok(MissionKind::Tfl),
            other => Err(MissionApiError::Validation(format!(
                "unknown mission kind: {}",
                other
            ))),
        }
    }
}

/// Role the assignee plays on the mission.
///
/// Mirrors `mission::domain::MissionRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionRole {
    Dev,
    Qc,
}

/// Error surface returned by every [`MissionService`] method.
///
/// Adapters translate backend-specific errors (e.g.
/// `mission::UsecaseError`) into this type at the implementation
/// boundary.
#[derive(Debug, Clone, Error)]
pub enum MissionApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("assignee not found")]
    AssigneeNotFound,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("forbidden: user {user_code} is not a leader of project {project_code}")]
    Forbidden {
        user_code: String,
        project_code: String,
    },

    #[error("mission already exists for {project_code}/{mission_kind:?}/{mission_code}")]
    DuplicateMission {
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
    },

    #[error("assignee already exists for mission {mission_id}/{user_code}/{role:?}")]
    DuplicateAssignee {
        mission_id: i64,
        user_code: String,
        role: MissionRole,
    },

    #[error("issue not found")]
    IssueNotFound,

    #[error("mission not found for issue {0}")]
    MissionNotFoundForIssue(i64),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Open / closed state of a `MissionIssue`. Mirrors
/// `mission::domain::IssueState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueState {
    Opened,
    Closed,
}

/// One entry in a `MissionIssue`'s append-only comment thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueCommentView {
    pub user: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Safe projection of a `MissionIssue` aggregate — comments are
/// hydrated to `Vec<IssueCommentView>` on read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueView {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueCommentView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input to [`MissionService::create_issue`].
#[derive(Debug, Clone)]
pub struct CreateIssueRequest {
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub description: String,
}

/// Empty marker request for [`MissionService::close_issue`].
/// The PATCH `/api/mission/issues/{issue_id}/state` handler
/// dispatches on `body.state`; this empty struct is what the
/// apis trait accepts when the body says `"state": "closed"`.
#[derive(Debug, Clone, Default)]
pub struct CloseIssueRequest {}

/// Empty marker request for [`MissionService::reopen_issue`].
#[derive(Debug, Clone, Default)]
pub struct ReopenIssueRequest {}

/// Input to [`MissionService::update_issue_description`].
#[derive(Debug, Clone)]
pub struct UpdateIssueDescriptionRequest {
    pub description: String,
}

/// Input to [`MissionService::append_comment`].
#[derive(Debug, Clone)]
pub struct AppendCommentRequest {
    pub content: String,
}

/// Query for [`MissionService::list_issues_by_mission`].
#[derive(Debug, Clone)]
pub struct ListIssuesByMissionRequest {
    pub mission_id: i64,
    pub state: Option<IssueState>,
}

/// Safe projection of an `Assignee` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssigneeView {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `Mission` aggregate — assignees are
/// hydrated to `Vec<AssigneeView>` on read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionView {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input DTO for [`MissionService::create_mission`].
#[derive(Debug, Clone)]
pub struct CreateMissionRequest {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeData>,
}

/// One assignee entry inside a [`CreateMissionRequest`] or a
/// standalone [`MissionService::add_assignee`] call.
#[derive(Debug, Clone)]
pub struct AssigneeData {
    pub user_code: String,
    pub role: MissionRole,
}

/// Query for [`MissionService::list_missions_by_project`].
#[derive(Debug, Clone)]
pub struct ListMissionsByProjectRequest {
    pub project_code: String,
    pub kind: Option<MissionKind>,
}

/// Query for [`MissionService::list_missions_by_user`].
#[derive(Debug, Clone)]
pub struct ListMissionsByUserRequest {
    pub user_code: String,
}

/// Shared actor type for any port that authorizes on behalf of an
/// authenticated user. Built by the transport layer from the JWT
/// subject (`AuthClaims.code`); passed to every write method.
#[derive(Debug, Clone)]
pub struct Actor {
    pub user_code: String,
}

/// Outbound port for mission lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn MissionService>` can be shared
/// state in an async server (axum, tarpc, …). Object-safe: no
/// generic methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `mission::MissionUsecase`) into this contract, translating
/// between backend-specific DTOs / errors and the `apis` types
/// defined above.
#[async_trait]
pub trait MissionService: Send + Sync {
    async fn create_mission(
        &self,
        actor: &Actor,
        req: CreateMissionRequest,
    ) -> Result<MissionView, MissionApiError>;

    async fn get_mission_by_id(&self, id: i64) -> Result<MissionView, MissionApiError>;

    async fn list_missions_by_project(
        &self,
        req: ListMissionsByProjectRequest,
    ) -> Result<Vec<MissionView>, MissionApiError>;

    async fn list_missions_by_user(
        &self,
        req: ListMissionsByUserRequest,
    ) -> Result<Vec<MissionView>, MissionApiError>;

    async fn delete_mission(&self, actor: &Actor, id: i64) -> Result<(), MissionApiError>;

    async fn add_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        data: AssigneeData,
    ) -> Result<AssigneeView, MissionApiError>;

    async fn remove_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        assignee_id: i64,
    ) -> Result<(), MissionApiError>;

    async fn list_issues_by_mission(
        &self,
        req: ListIssuesByMissionRequest,
    ) -> Result<Vec<IssueView>, MissionApiError>;

    async fn create_issue(
        &self,
        actor: &Actor,
        req: CreateIssueRequest,
    ) -> Result<IssueView, MissionApiError>;

    async fn close_issue(
        &self,
        actor: &Actor,
        req: CloseIssueRequest,
        issue_id: i64,
    ) -> Result<IssueView, MissionApiError>;

    async fn reopen_issue(
        &self,
        actor: &Actor,
        req: ReopenIssueRequest,
        issue_id: i64,
    ) -> Result<IssueView, MissionApiError>;

    async fn update_issue_description(
        &self,
        actor: &Actor,
        issue_id: i64,
        req: UpdateIssueDescriptionRequest,
    ) -> Result<IssueView, MissionApiError>;

    async fn append_comment(
        &self,
        actor: &Actor,
        issue_id: i64,
        req: AppendCommentRequest,
    ) -> Result<IssueView, MissionApiError>;
}

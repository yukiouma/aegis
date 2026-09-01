//! Outbound port for the mission service.
//!
//! Mirrors the surface of `mission::usecase::MissionUsecase` so
//! adapters in any backend (in-memory, PostgreSQL, …) can adapt
//! their own types to the shared contract defined here. All
//! supporting DTOs (request shapes, view projections, enums, and
//! [`MissionApiError`]) live alongside the trait so a single
//! `use apis::mission::*;` brings the whole contract into scope.

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

    #[error("repository error: {0}")]
    Repository(String),
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

    async fn delete_mission(
        &self,
        actor: &Actor,
        id: i64,
    ) -> Result<(), MissionApiError>;

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
}
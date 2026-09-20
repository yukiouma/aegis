//! Outbound port for project lifecycle operations.
//!
//! See [`ProjectService`] for the trait surface. All supporting types
//! (`ProjectApiError`, `ProjectView`, `ProjectConfigurationView`,
//! `ProjectMemberView`, `UserSummaryView`, `TagData`, `TagView`,
//! `*Request`) are defined alongside the trait so a single
//! `use apis::project::*;` brings the whole contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error surface returned by every [`ProjectService`] method.
///
/// Adapters map backend-specific errors (e.g. `project::UsecaseError`)
/// into this type at the implementation boundary.
#[derive(Debug, Clone, Error)]
pub enum ProjectApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Locale hint. Carried on the project configuration so a project
/// can pin its UI to English or Simplified Chinese. The wire code
/// is the lowercase IETF tag (`"en"` or `"zh-CN"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectLanguage {
    English,
    SimplifiedChinese,
}

impl ProjectLanguage {
    /// Stable wire code. New locales land here as new variants.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectLanguage::English => "en",
            ProjectLanguage::SimplifiedChinese => "zh-CN",
        }
    }
}

/// Wire-shaped tag data. `key` and `value` are both required and
/// non-empty; the backend enforces that contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagData {
    pub key: String,
    pub value: String,
}

/// Server-side projection of a tag. Same shape as [`TagData`]; kept
/// as a distinct type so the wire DTO can diverge later without
/// breaking the projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagView {
    pub key: String,
    pub value: String,
}

/// Request-side configuration: optional locale plus the tag list.
/// Carries `TagData` because it travels into the backend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationData {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagData>,
}

/// Server-side projection of the configuration. Carries `TagView`
/// because it travels back to clients.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationView {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagView>,
}

/// Safe projection of a project: membership lists are hydrated to
/// `Vec<UserSummaryView>`; the configuration (locale + tags) is
/// projected to `ProjectConfigurationView`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub configurations: ProjectConfigurationView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummaryView {
    pub code: String,
    pub name: String,
}

/// Wire-shaped membership data. `leaders` and `workers` are user codes
/// (not full user records); the backend hydrates them to
/// `UserSummaryView` on read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectMemberData {
    pub leaders: Vec<String>,
    pub workers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    /// Optional. Omit (or pass an empty `ProjectMemberData`) to create
    /// the project with no membership rows; the shell can be filled in
    /// via a later `update_project` call.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    /// Optional. `None` and `Some(empty)` both mean "default empty
    /// configuration (no language, no tags)".
    pub configurations: Option<ProjectConfigurationData>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProjectRequest {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    /// `None` = leave configuration unchanged; `Some(config)` =
    /// whole-configuration replace.
    pub configurations: Option<ProjectConfigurationData>,
}

/// Outbound port for project lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn ProjectService>` can be shared state in
/// an async server (axum, tarpc, etc.).
#[async_trait]
pub trait ProjectService: Send + Sync {
    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError>;
    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError>;
    async fn update_project(
        &self,
        req: UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;
}
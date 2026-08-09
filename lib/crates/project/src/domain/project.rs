use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::project_member::ProjectMember;

#[derive(Clone, PartialEq, Eq)]
pub struct Project {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product_id: i32,
    pub members: ProjectMember,
    pub unblind_members: ProjectMember,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        code: String,
        description: String,
        product_id: i32,
        members: ProjectMember,
        unblind_members: ProjectMember,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if product_id == 0 {
            return Err(DomainError::ZeroProductId);
        }
        Ok(Self {
            id,
            code,
            description,
            product_id,
            members,
            unblind_members,
            active,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i32,
        code: String,
        description: String,
        product_id: i32,
        members: ProjectMember,
        unblind_members: ProjectMember,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            code,
            description,
            product_id,
            members,
            unblind_members,
            active,
            created_at,
            updated_at,
        }
    }
}

impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Project")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("description", &self.description)
            .field("product_id", &self.product_id)
            .field("members", &self.members)
            .field("unblind_members", &self.unblind_members)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Input DTO for `ProjectRepository::create`.
#[derive(Debug, Clone)]
pub struct ProjectNew {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    /// Optional. `None` and `Some(empty)` are equivalent — neither
    /// inserts any `project_members` rows for that team. Letting the
    /// field be absent keeps the "create shell, add members later"
    /// flow ergonomic.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}

/// Input DTO for `ProjectRepository::update`. Every field is optional
/// so the usecase can pass only the fields that actually changed.
#[derive(Debug, Clone, Default)]
pub struct ProjectUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub product_id: Option<i32>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe that
    /// team's rows. The two are distinct on update.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}

/// Outbound port for persistence of `Project` aggregates.
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError>;
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError>;
    async fn list(&self) -> Result<Vec<Project>, DomainError>;
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError>;
}

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A published Case Report Form release, identified by
/// `(project_code, name)`. The name is free-form (typically a
/// `yyyy-mm-dd` workbook sheet suffix but stored as `String`
/// rather than parsed as a `NaiveDate` so a future sheet with a
/// non-date name round-trips intact).
#[derive(Clone, PartialEq, Eq)]
pub struct CrfVersion {
    pub id: i32,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for CrfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrfVersion")
            .field("id", &self.id)
            .field("project_code", &self.project_code)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl CrfVersion {
    /// Validating constructor used by the domain layer.
    /// Rejects empty / whitespace `project_code` and `name`.
    pub fn new(project_code: String, name: String) -> Result<Self, DomainError> {
        if project_code.trim().is_empty() {
            return Err(DomainError::EmptyProjectCode);
        }
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            project_code,
            name,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i32,
        project_code: String,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            project_code,
            name,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CrfVersionRepository::create`.
#[derive(Debug, Clone)]
pub struct CrfVersionNew {
    pub project_code: String,
    pub name: String,
}

/// Input DTO for `CrfVersionRepository::update`. Only `name`
/// is mutable on a version; `id` identifies the row.
#[derive(Debug, Clone, Default)]
pub struct CrfVersionUpdate {
    pub id: i32,
    pub name: Option<String>,
}

/// Persistence port for the `CrfVersion` aggregate.
#[async_trait]
pub trait CrfVersionRepository: Send + Sync {
    async fn create(&self, input: CrfVersionNew) -> Result<CrfVersion, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<CrfVersion, DomainError>;
    async fn list_by_project(&self, project_code: &str) -> Result<Vec<CrfVersion>, DomainError>;
    async fn update(&self, input: CrfVersionUpdate) -> Result<CrfVersion, DomainError>;
    async fn delete(&self, id: i32) -> Result<(), DomainError>;
    async fn search_by_version(
        &self,
        project_code: &str,
        fragment: &str,
    ) -> Result<Vec<CrfVersion>, DomainError>;
}

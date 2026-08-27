use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A form within a `CrfVersion`. Forms are the outer grouping
/// of items, options, units, domain-annotations, and
/// annotations. Cascade-deleted with their parent version.
#[derive(Clone, PartialEq, Eq)]
pub struct CrfForm {
    pub id: i32,
    pub version_id: i32,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for CrfForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrfForm")
            .field("id", &self.id)
            .field("version_id", &self.version_id)
            .field("code", &self.code)
            .field("name", &self.name)
            .field("order", &self.order)
            .field("not_submitted", &self.not_submitted)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl CrfForm {
    /// Validating constructor used by the domain layer.
    /// Rejects empty / whitespace `code` and `name`.
    pub fn new(
        version_id: i32,
        code: String,
        name: String,
        order: i32,
        not_submitted: bool,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            version_id,
            code,
            name,
            order,
            not_submitted,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer
    /// when materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i32,
        version_id: i32,
        code: String,
        name: String,
        order: i32,
        not_submitted: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            version_id,
            code,
            name,
            order,
            not_submitted,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CrfFormRepository::create`.
#[derive(Debug, Clone)]
pub struct CrfFormNew {
    pub version_id: i32,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
}

/// Input DTO for `CrfFormRepository::update`. Every field
/// except `id` is optional.
#[derive(Debug, Clone, Default)]
pub struct CrfFormUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub order: Option<i32>,
    pub not_submitted: Option<bool>,
}

/// Persistence port for the `CrfForm` aggregate.
#[async_trait]
pub trait CrfFormRepository: Send + Sync {
    async fn create(&self, input: CrfFormNew) -> Result<CrfForm, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<CrfForm, DomainError>;
    async fn list_by_version(&self, version_id: i32) -> Result<Vec<CrfForm>, DomainError>;
    async fn update(&self, input: CrfFormUpdate) -> Result<CrfForm, DomainError>;
    async fn delete(&self, id: i32) -> Result<(), DomainError>;
    async fn search_by_version(
        &self,
        version_id: i32,
        fragment: &str,
    ) -> Result<Vec<CrfForm>, DomainError>;
}

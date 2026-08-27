use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A named label attached to a `CrfForm`. Version-scoped pool
/// (one domain annotation belongs to exactly one form; the
/// `version_id` is reachable through `crf_forms.version_id`).
///
/// `UNIQUE (form_id, name)` enforces the natural key
/// "label names are unique within a form".
#[derive(Clone, PartialEq, Eq)]
pub struct DomainAnnotation {
    pub id: i64,
    pub form_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for DomainAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainAnnotation")
            .field("id", &self.id)
            .field("form_id", &self.form_id)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl DomainAnnotation {
    /// Validating constructor used by the domain layer.
    /// Rejects empty / whitespace `name`.
    pub fn new(form_id: i64, name: String, description: String) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            form_id,
            name,
            description,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer
    /// when materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        form_id: i64,
        name: String,
        description: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            form_id,
            name,
            description,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `DomainAnnotationRepository::create`.
#[derive(Debug, Clone)]
pub struct DomainAnnotationNew {
    pub form_id: i64,
    pub name: String,
    pub description: String,
}

/// Input DTO for `DomainAnnotationRepository::update`.
#[derive(Debug, Clone, Default)]
pub struct DomainAnnotationUpdate {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Persistence port for the `DomainAnnotation` aggregate.
#[async_trait]
pub trait DomainAnnotationRepository: Send + Sync {
    async fn create(&self, input: DomainAnnotationNew) -> Result<DomainAnnotation, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<DomainAnnotation, DomainError>;
    async fn list_by_form(&self, form_id: i64) -> Result<Vec<DomainAnnotation>, DomainError>;
    async fn update(&self, input: DomainAnnotationUpdate) -> Result<DomainAnnotation, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<DomainAnnotation>, DomainError>;
}

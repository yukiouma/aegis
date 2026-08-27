use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::crf_item_kind::CrfItemKind;
use super::error::DomainError;

/// A single data-collection cell within a form. The `kind`
/// discriminant governs shape (e.g. `Selection` carries
/// options; `Label` carries nothing).
#[derive(Clone, PartialEq, Eq)]
pub struct CrfItem {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for CrfItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrfItem")
            .field("id", &self.id)
            .field("form_id", &self.form_id)
            .field("code", &self.code)
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("order", &self.order)
            .field("not_submitted", &self.not_submitted)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl CrfItem {
    /// Validating constructor used by the domain layer.
    /// Rejects empty / whitespace `code` and `name`.
    pub fn new(
        form_id: i64,
        code: String,
        name: String,
        kind: CrfItemKind,
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
            form_id,
            code,
            name,
            kind,
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
        id: i64,
        form_id: i64,
        code: String,
        name: String,
        kind: CrfItemKind,
        order: i32,
        not_submitted: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            form_id,
            code,
            name,
            kind,
            order,
            not_submitted,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CrfItemRepository::create`.
#[derive(Debug, Clone)]
pub struct CrfItemNew {
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
}

/// Input DTO for `CrfItemRepository::update`. Every field
/// except `id` is optional.
#[derive(Debug, Clone, Default)]
pub struct CrfItemUpdate {
    pub id: i64,
    pub code: Option<String>,
    pub name: Option<String>,
    pub order: Option<i32>,
    pub not_submitted: Option<bool>,
}

/// Persistence port for the `CrfItem` aggregate.
#[async_trait]
pub trait CrfItemRepository: Send + Sync {
    async fn create(&self, input: CrfItemNew) -> Result<CrfItem, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CrfItem, DomainError>;
    async fn list_by_form(&self, form_id: i64) -> Result<Vec<CrfItem>, DomainError>;
    async fn update(&self, input: CrfItemUpdate) -> Result<CrfItem, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    /// Search by code/name through the version chain. The
    /// caller must already have validated that the fragment is
    /// non-empty (the usecase does this).
    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfItem>, DomainError>;
}

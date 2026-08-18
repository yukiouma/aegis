use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A single permissible value inside a `CodeList`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeItem {
    pub id: i64,
    pub codelist_id: i64,
    /// Denormalised copy of the parent codelist's `version_id`.
    /// Lets the repository answer
    /// `list_by_version_and_codelist_code` without a self-join,
    /// and lets consumers read the owning version off the item.
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CodeItem {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `code`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        codelist_id: i64,
        version_id: i64,
        code: String,
        submission_value: String,
        synonym: String,
        definition: String,
        nci_preferred_term: String,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        Ok(Self {
            id: 0,
            codelist_id,
            version_id,
            code,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        codelist_id: i64,
        version_id: i64,
        code: String,
        submission_value: String,
        synonym: String,
        definition: String,
        nci_preferred_term: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            codelist_id,
            version_id,
            code,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CodeItemRepository::create`.
#[derive(Debug, Clone)]
pub struct CodeItemNew {
    pub codelist_id: i64,
    pub version_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Input DTO for `CodeItemRepository::update`. Every field is
/// optional so the usecase can pass only what actually changed.
#[derive(Debug, Clone, Default)]
pub struct CodeItemUpdate {
    pub id: i64,
    pub code: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

/// Query for `CodeItemRepository::search`. Mirrors
/// [`CodeListSearchQuery`].
#[derive(Debug, Clone)]
pub struct CodeItemSearchQuery {
    pub version_id: i64,
    pub fragment: String,
    /// Default 50. Hard cap 500 (clamped, not rejected).
    pub limit: u32,
}

/// One hit from `CodeItemRepository::search`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeItemSearchHit {
    pub item: CodeItem,
}

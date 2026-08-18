use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A CDISC codelist and the items that belong to it. The
/// in-memory shape mirrors the workbook; the persisted shape
/// keeps the items in a separate `code_items` table referenced
/// by `codelist_id`.
#[derive(Clone, PartialEq, Eq)]
pub struct CodeList {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for CodeList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeList")
            .field("id", &self.id)
            .field("version_id", &self.version_id)
            .field("code", &self.code)
            .field("extensible", &self.extensible)
            .field("name", &self.name)
            .field("submission_value", &self.submission_value)
            .field("synonym", &self.synonym)
            .field("definition", &self.definition)
            .field("nci_preferred_term", &self.nci_preferred_term)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl CodeList {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `code` (the NCI C-code).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version_id: i64,
        code: String,
        extensible: bool,
        name: String,
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
            version_id,
            code,
            extensible,
            name,
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
        version_id: i64,
        code: String,
        extensible: bool,
        name: String,
        submission_value: String,
        synonym: String,
        definition: String,
        nci_preferred_term: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            version_id,
            code,
            extensible,
            name,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CodeListRepository::create`.
#[derive(Debug, Clone)]
pub struct CodeListNew {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Input DTO for `CodeListRepository::update`. Every field is
/// optional so the usecase can pass only what actually changed.
#[derive(Debug, Clone, Default)]
pub struct CodeListUpdate {
    pub id: i64,
    pub code: Option<String>,
    pub extensible: Option<bool>,
    pub name: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

/// Query for `CodeListRepository::search`. Search is scoped to a
/// single `(kind, version_id)` pair so callers cannot accidentally
/// cross-releases.
#[derive(Debug, Clone)]
pub struct CodeListSearchQuery {
    pub version_id: i64,
    pub text: String,
    /// Default 50. Hard cap 500 (clamped, not rejected).
    pub limit: u32,
}

/// One hit from `CodeListRepository::search`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeListSearchHit {
    pub codelist: CodeList,
    pub score: f32,
}

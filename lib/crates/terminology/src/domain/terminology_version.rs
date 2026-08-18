use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::terminology_kind::TerminologyKind;

/// A published CDISC terminology release, identified by its
/// `(kind, name)` pair. `name` is the `yyyy-mm-dd` suffix of the
/// matched workbook sheet and is stored as `String` (not parsed
/// into a `NaiveDate`) so a future sheet with a non-date name
/// round-trips intact.
#[derive(Clone, PartialEq, Eq)]
pub struct TerminologyVersion {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for TerminologyVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminologyVersion")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl TerminologyVersion {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    pub fn new(kind: TerminologyKind, name: String) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0, // placeholder; for_repository overwrites it
            kind,
            name,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        kind: TerminologyKind,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            kind,
            name,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `TerminologyVersionRepository::create`.
#[derive(Debug, Clone)]
pub struct TerminologyVersionNew {
    pub kind: TerminologyKind,
    pub name: String,
}

/// Input DTO for `TerminologyVersionRepository::update`. Every
/// field is optional so the usecase can pass only what actually
/// changed.
#[derive(Debug, Clone, Default)]
pub struct TerminologyVersionUpdate {
    pub id: i64,
    pub kind: Option<TerminologyKind>,
    pub name: Option<String>,
}
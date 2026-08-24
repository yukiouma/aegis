use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A published SDTM release, identified by `name`. Typically
/// a `yyyy-mm-dd` workbook sheet suffix; stored as `String`
/// (not parsed as a `NaiveDate`) so a future sheet with a
/// non-date name round-trips intact.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmVersion {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmVersion")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmVersion {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    pub fn new(name: String) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            name,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmVersionRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmVersionNew {
    pub name: String,
}

/// Input DTO for `SdtmVersionRepository::update`. Only `name`
/// is mutable on a version; `id` identifies the row.
#[derive(Debug, Clone, Default)]
pub struct SdtmVersionUpdate {
    pub id: i64,
    pub name: Option<String>,
}

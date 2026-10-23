use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A project's own SDTM model version. Identified by
/// `(project_code, version_name)`, where `project_code` references
/// `Project.code` in the `project` crate. The reference is validated at
/// the application level (see [`ProjectLookup`](super::project_lookup::ProjectLookup)),
/// not by a database foreign key, so the two crates' schemas stay
/// independent.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectVersion {
    pub id: i64,
    pub project_code: String,
    pub version_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmProjectVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmProjectVersion")
            .field("id", &self.id)
            .field("project_code", &self.project_code)
            .field("version_name", &self.version_name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmProjectVersion {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `project_code` and `version_name`.
    pub fn new(project_code: String, version_name: String) -> Result<Self, DomainError> {
        if project_code.trim().is_empty() {
            return Err(DomainError::EmptyProjectCode);
        }
        if version_name.trim().is_empty() {
            return Err(DomainError::EmptyVersionName);
        }
        Ok(Self {
            id: 0,
            project_code,
            version_name,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    // Reserved for the adapter layer. The postgres adapters that
    // call this land with the persistence task; until then the only
    // caller is the in-memory test fakes, which are cfg(test)-only.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        project_code: String,
        version_name: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            project_code,
            version_name,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmProjectVersionRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectVersionNew {
    pub project_code: String,
    pub version_name: String,
}

/// Input DTO for `SdtmProjectVersionRepository::update`. Only
/// `version_name` is mutable — `project_code` is immutable so a
/// version cannot be moved between projects and orphan its tree.
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectVersionUpdate {
    pub id: i64,
    pub version_name: Option<String>,
}

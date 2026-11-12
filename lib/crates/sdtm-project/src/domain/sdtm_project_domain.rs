use chrono::{DateTime, Utc};

use super::domain_category::DomainCategory;
use super::error::DomainError;
use super::sdtm_domain_description::SdtmDomainDescription;

/// A domain a project declares inside one of its SDTM model versions.
///
/// The aggregate is a **complete copy** of the domain: it carries the
/// category and the descriptions itself, rather than pointing at a
/// standard domain and patching it.
///
/// `key_variables` is an **ordered** collection: the order the caller
/// supplies is meaningful and is preserved through the JSONB column
/// and out to the wire. It still has set semantics in one respect —
/// duplicates collapse to their first occurrence. Every code it names
/// must be a variable of this same domain; the usecase enforces that.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectDomain {
    pub id: i64,
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmProjectDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmProjectDomain")
            .field("id", &self.id)
            .field("sdtm_project_version_id", &self.sdtm_project_version_id)
            .field("domain_code", &self.domain_code)
            .field("category", &self.category)
            .field("descriptions", &self.descriptions)
            .field("key_variables", &self.key_variables)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmProjectDomain {
    /// Validating constructor used by the domain layer. Rejects an
    /// empty / whitespace `domain_code` and a blank key-variable code.
    pub fn new(
        sdtm_project_version_id: i64,
        domain_code: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        key_variables: Vec<String>,
    ) -> Result<Self, DomainError> {
        if domain_code.trim().is_empty() {
            return Err(DomainError::EmptyDomainCode);
        }
        let key_variables = Self::normalize_key_variables(&key_variables)?;
        Ok(Self {
            id: 0,
            sdtm_project_version_id,
            domain_code,
            category,
            descriptions,
            key_variables,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Normalises a caller-supplied key-variable list: every entry is
    /// trimmed, a blank entry is rejected, duplicates collapse to their
    /// first occurrence, and the caller's order is preserved.
    ///
    /// Never re-sorts — the order is the caller's, by design.
    pub fn normalize_key_variables(raw: &[String]) -> Result<Vec<String>, DomainError> {
        let mut out: Vec<String> = Vec::with_capacity(raw.len());
        for entry in raw {
            let trimmed = entry.trim();
            if trimmed.is_empty() {
                return Err(DomainError::EmptyKeyVariable);
            }
            let trimmed = trimmed.to_string();
            if !out.contains(&trimmed) {
                out.push(trimmed);
            }
        }
        Ok(out)
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        sdtm_project_version_id: i64,
        domain_code: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        key_variables: Vec<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            sdtm_project_version_id,
            domain_code,
            category,
            descriptions,
            key_variables,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmProjectDomainRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectDomainNew {
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
}

/// Input DTO for `SdtmProjectDomainRepository::update`.
/// `sdtm_project_version_id` is immutable — a domain always belongs to
/// the version it was created under.
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectDomainUpdate {
    pub id: i64,
    pub domain_code: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
    pub key_variables: Option<Vec<String>>,
}

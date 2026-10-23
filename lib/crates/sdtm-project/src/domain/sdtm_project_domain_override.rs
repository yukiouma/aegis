use chrono::{DateTime, Utc};

use super::domain_category::DomainCategory;
use super::error::DomainError;
use super::sdtm_domain_description::SdtmDomainDescription;

/// A project's own override for one of its domains. Each domain holds
/// at most one override, enforced by `UNIQUE (domain_id)` on
/// `sdtm_project_domain_overrides`.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainOverride {
    pub id: i64,
    pub domain_id: i64,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmProjectDomainOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmProjectDomainOverride")
            .field("id", &self.id)
            .field("domain_id", &self.domain_id)
            .field("category", &self.category)
            .field("descriptions", &self.descriptions)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmProjectDomainOverride {
    /// Validating constructor used by the domain layer. There are no
    /// free-form string fields to reject, so this only builds the
    /// aggregate.
    pub fn new(
        domain_id: i64,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: 0,
            domain_id,
            category,
            descriptions,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    // Used by the adapter layer and the in-memory test fakes; both land
    // in later tasks.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        domain_id: i64,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            domain_id,
            category,
            descriptions,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmProjectDomainOverrideRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectDomainOverrideNew {
    pub domain_id: i64,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// Input DTO for `SdtmProjectDomainOverrideRepository::update`. The
/// override is keyed on its **parent** id — a 1:1 sub-resource's
/// natural key — so the caller never has to know the surrogate id.
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectDomainOverrideUpdate {
    pub domain_id: i64,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

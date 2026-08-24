use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::domain_category::DomainCategory;
use super::error::DomainError;

/// Localised description of an SDTM domain. Carried on the
/// `SdtmDomain` aggregate and persisted as a single JSONB
/// column on `sdtm_domains`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

/// A single SDTM domain (e.g. `AE`, `DM`, `VS`) attached to
/// a `SdtmVersion` and described in one or more languages.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmDomain {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmDomain")
            .field("id", &self.id)
            .field("version_id", &self.version_id)
            .field("name", &self.name)
            .field("category", &self.category)
            .field("descriptions", &self.descriptions)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmDomain {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    pub fn new(
        version_id: i64,
        name: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            version_id,
            name,
            category,
            descriptions,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        version_id: i64,
        name: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            version_id,
            name,
            category,
            descriptions,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmDomainRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmDomainNew {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// Input DTO for `SdtmDomainRepository::update`. Every field
/// except `id` is optional so the usecase can pass only what
/// actually changed. `descriptions: None` means "don't touch",
/// `Some(vec)` means "replace with this list" (use an empty
/// `vec![]` to clear the column).
#[derive(Debug, Clone, Default)]
pub struct SdtmDomainUpdate {
    pub id: i64,
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

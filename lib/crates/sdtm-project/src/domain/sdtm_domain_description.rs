use serde::{Deserialize, Serialize};

/// Localised description of a domain. Carried on the
/// `SdtmProjectDomainOverride` aggregate and persisted as a single
/// JSONB column on `sdtm_project_domain_overrides`.
///
/// Copied from `domain_model::SdtmDomainDescription` — see the note on
/// [`DomainCategory`](super::domain_category::DomainCategory) for why
/// the two crates share no types.
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

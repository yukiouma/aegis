use serde::{Deserialize, Serialize};

/// Localised description of a variable. Carried on the
/// `SdtmProjectVariableOverride` aggregate and persisted as a single
/// JSONB column on `sdtm_project_variable_overrides`.
///
/// Copied from `domain_model::SdtmVariableDescription` — see the note on
/// [`DomainCategory`](super::domain_category::DomainCategory) for why
/// the two crates share no types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

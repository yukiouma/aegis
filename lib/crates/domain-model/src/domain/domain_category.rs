use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// SDTM domain category. The string form (`"Special Purpose"`,
/// `"Interventions"`, ...) is the wire shape consumed by the
/// postgres adapter (CHECK constraint + JSONB round-trip) and
/// by the apis port DTOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

impl DomainCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainCategory::SpecialPurpose => "Special Purpose",
            DomainCategory::Interventions => "Interventions",
            DomainCategory::Events => "Events",
            DomainCategory::Findings => "Findings",
            DomainCategory::TrialDesign => "Trial Design",
            DomainCategory::Relationships => "Relationships",
            DomainCategory::StudyReference => "Study Reference",
        }
    }
}

impl TryFrom<&str> for DomainCategory {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Special Purpose" => Ok(DomainCategory::SpecialPurpose),
            "Interventions" => Ok(DomainCategory::Interventions),
            "Events" => Ok(DomainCategory::Events),
            "Findings" => Ok(DomainCategory::Findings),
            "Trial Design" => Ok(DomainCategory::TrialDesign),
            "Relationships" => Ok(DomainCategory::Relationships),
            "Study Reference" => Ok(DomainCategory::StudyReference),
            other => Err(DomainError::InvalidDomainCategory(other.to_string())),
        }
    }
}

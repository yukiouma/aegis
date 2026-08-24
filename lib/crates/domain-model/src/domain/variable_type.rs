use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

impl SdtmVariableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmVariableType::Numeric => "Numeric",
            SdtmVariableType::Character => "Character",
        }
    }
}

impl TryFrom<&str> for SdtmVariableType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Numeric" => Ok(SdtmVariableType::Numeric),
            "Character" => Ok(SdtmVariableType::Character),
            other => Err(DomainError::InvalidVariableType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

impl SdtmVariableCore {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmVariableCore::Req => "Req",
            SdtmVariableCore::Exp => "Exp",
            SdtmVariableCore::Perm => "Perm",
            SdtmVariableCore::Supp => "Supp",
        }
    }
}

impl TryFrom<&str> for SdtmVariableCore {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Req" => Ok(SdtmVariableCore::Req),
            "Exp" => Ok(SdtmVariableCore::Exp),
            "Perm" => Ok(SdtmVariableCore::Perm),
            "Supp" => Ok(SdtmVariableCore::Supp),
            other => Err(DomainError::InvalidVariableCore(other.to_string())),
        }
    }
}

/// SDTM variable role. The string form is consumed by the
/// postgres adapter (`sdtm_variables.variable_role` column +
/// CHECK constraint) and by the apis port DTOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

impl SdtmRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmRole::Identifier => "Identifier",
            SdtmRole::Topic => "Topic",
            SdtmRole::Timing => "Timing",
            SdtmRole::RecordQualifier => "Record Qualifier",
            SdtmRole::SynonymQualifier => "Synonym Qualifier",
            SdtmRole::VariableQualifier => "Variable Qualifier",
            SdtmRole::GroupingQualifier => "Grouping Qualifier",
            SdtmRole::Rule => "Rule",
        }
    }
}

impl TryFrom<&str> for SdtmRole {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Identifier" => Ok(SdtmRole::Identifier),
            "Topic" => Ok(SdtmRole::Topic),
            "Timing" => Ok(SdtmRole::Timing),
            "Record Qualifier" => Ok(SdtmRole::RecordQualifier),
            "Synonym Qualifier" => Ok(SdtmRole::SynonymQualifier),
            "Variable Qualifier" => Ok(SdtmRole::VariableQualifier),
            "Grouping Qualifier" => Ok(SdtmRole::GroupingQualifier),
            "Rule" => Ok(SdtmRole::Rule),
            other => Err(DomainError::InvalidVariableRole(other.to_string())),
        }
    }
}

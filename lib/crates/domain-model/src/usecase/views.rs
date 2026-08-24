use chrono::{DateTime, Utc};

use crate::domain::{
    DomainCategory, SdtmDomain, SdtmDomainDescription, SdtmRole, SdtmVariable, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableType, SdtmVersion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVersionView {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmVersion> for SdtmVersionView {
    fn from(v: SdtmVersion) -> Self {
        Self {
            id: v.id,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmDomainView {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmDomain> for SdtmDomainView {
    fn from(d: SdtmDomain) -> Self {
        Self {
            id: d.id,
            version_id: d.version_id,
            name: d.name,
            category: d.category,
            descriptions: d.descriptions,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVariableView {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmVariable> for SdtmVariableView {
    fn from(v: SdtmVariable) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            name: v.name,
            variable_controlled: v.variable_controlled,
            variable_type: v.variable_type,
            variable_core: v.variable_core,
            variable_role: v.variable_role,
            variable_sequence: v.variable_sequence,
            descriptions: v.descriptions,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

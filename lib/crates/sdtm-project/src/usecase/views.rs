use chrono::{DateTime, Utc};

use crate::domain::{
    ConversionRule, DomainCategory, SdtmDomainDescription, SdtmProjectDomain, SdtmProjectVariable,
    SdtmProjectVersion, SdtmRole, SdtmVariableControlled, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVersionView {
    pub id: i64,
    pub project_code: String,
    pub version_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectVersion> for SdtmProjectVersionView {
    fn from(v: SdtmProjectVersion) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            version_name: v.version_name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectDomainView {
    pub id: i64,
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectDomain> for SdtmProjectDomainView {
    fn from(d: SdtmProjectDomain) -> Self {
        Self {
            id: d.id,
            sdtm_project_version_id: d.sdtm_project_version_id,
            domain_code: d.domain_code,
            category: d.category,
            descriptions: d.descriptions,
            key_variables: d.key_variables,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableView {
    pub id: i64,
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmProjectVariable> for SdtmProjectVariableView {
    fn from(v: SdtmProjectVariable) -> Self {
        Self {
            id: v.id,
            sdtm_project_domain_id: v.sdtm_project_domain_id,
            variable_code: v.variable_code,
            variable_controlled: v.variable_controlled,
            variable_type: v.variable_type,
            variable_core: v.variable_core,
            variable_role: v.variable_role,
            variable_sequence: v.variable_sequence,
            descriptions: v.descriptions,
            conversion_rules: v.conversion_rules,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

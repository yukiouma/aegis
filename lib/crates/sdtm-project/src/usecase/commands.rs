use crate::domain::{
    ConversionRule, DomainCategory, SdtmDomainDescription, SdtmRole, SdtmVariableControlled,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableType,
};

// SdtmProjectVersion

#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVersion {
    pub project_code: String,
    pub version_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectVersion {
    pub id: i64,
    pub version_name: Option<String>,
}

// SdtmProjectDomain

#[derive(Debug, Clone)]
pub struct CreateSdtmProjectDomain {
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectDomain {
    pub id: i64,
    pub domain_code: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
    pub key_variables: Option<Vec<String>>,
}

// SdtmProjectVariable

#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVariable {
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
}

/// `variable_controlled` and `variable_role` are three-state: outer
/// `None` = leave unchanged, `Some(None)` = clear the column to NULL,
/// `Some(Some(role))` = replace.
#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectVariable {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
    pub conversion_rules: Option<Vec<ConversionRule>>,
}

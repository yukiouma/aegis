use crate::domain::{
    DomainCategory, SdtmDomainDescription, SdtmRole, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableType,
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
    pub version_id: i64,
    pub domain_code: String,
    pub key_variables: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectDomain {
    pub id: i64,
    pub domain_code: Option<String>,
    pub key_variables: Option<Vec<String>>,
}

// SdtmProjectDomainOverride

#[derive(Debug, Clone)]
pub struct CreateSdtmProjectDomainOverride {
    pub domain_id: i64,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// Keyed on the parent `domain_id` — a 1:1 sub-resource's natural key
/// — rather than on the override's own surrogate id.
#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectDomainOverride {
    pub domain_id: i64,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

// SdtmProjectVariable

#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVariable {
    pub domain_id: i64,
    pub variable_code: String,
    pub variable_sequence: i64,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectVariable {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_sequence: Option<i64>,
}

// SdtmProjectVariableOverride

#[derive(Debug, Clone)]
pub struct CreateSdtmProjectVariableOverride {
    pub variable_id: i64,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// Keyed on the parent `variable_id` — a 1:1 sub-resource's natural
/// key — rather than on the override's own surrogate id.
///
/// `variable_role` is three-state: outer `None` = leave unchanged,
/// `Some(None)` = clear the column to NULL, `Some(Some(role))` =
/// replace.
#[derive(Debug, Clone, Default)]
pub struct UpdateSdtmProjectVariableOverride {
    pub variable_id: i64,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

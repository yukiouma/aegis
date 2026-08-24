use crate::domain::{
    DomainCategory, SdtmDomainDescription, SdtmRole, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableType,
};

// SdtmVersion

pub struct CreateSdtmVersion {
    pub name: String,
}

#[derive(Default)]
pub struct UpdateSdtmVersion {
    pub id: i64,
    pub name: Option<String>,
}

// SdtmDomain

pub struct CreateSdtmDomain {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

#[derive(Default)]
pub struct UpdateSdtmDomain {
    pub id: i64,
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

// SdtmVariable

pub struct CreateSdtmVariable {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

#[derive(Default)]
pub struct UpdateSdtmVariable {
    pub id: i64,
    pub name: Option<String>,
    /// `None` = don't change. `Some(None)` = clear the field.
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    /// `None` = don't change. `Some(None)` = clear the field.
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

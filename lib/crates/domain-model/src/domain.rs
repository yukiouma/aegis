mod domain_category;
mod error;
mod repository;
mod sdtm_domain;
mod sdtm_variable;
mod sdtm_version;
mod variable_type;

#[cfg(test)]
mod tests;

pub use domain_category::DomainCategory;
pub use error::DomainError;
pub use repository::{SdtmDomainRepository, SdtmVariableRepository, SdtmVersionRepository};
pub use sdtm_domain::{
    SdtmDomain, SdtmDomainDescription, SdtmDomainDescriptionDetail, SdtmDomainNew, SdtmDomainUpdate,
};
pub use sdtm_variable::{
    SdtmVariable, SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableNew,
    SdtmVariableUpdate,
};
pub use sdtm_version::{SdtmVersion, SdtmVersionNew, SdtmVersionUpdate};
pub use variable_type::{SdtmRole, SdtmVariableCore, SdtmVariableType};

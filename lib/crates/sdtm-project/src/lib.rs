//! # sdtm-project crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for the **per-project SDTM model tree** — the domains and variables a
//! project declares on top of the CDISC SDTM standard — and an async
//! `SdtmProjectUsecase` that orchestrates them.
//!
//! The crate deliberately shares no types with `domain-model`: the
//! standard and the project overlay are allowed to drift independently,
//! so the value objects (`DomainCategory`, `SdtmVariableType`,
//! `SdtmVariableCore`, `SdtmRole`, `SdtmVariableControlled` and the
//! description DTOs) are copied rather than imported.
//!
//! A project domain and a project variable are **complete copies** of
//! the corresponding standard domain / variable: they carry their own
//! category, descriptions, type, core, role and controlled terminology
//! rather than pointing at the standard and patching it. A variable also
//! carries its `conversion_rules` — where its value comes from and how
//! it is derived.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::SdtmProjectServiceImpl;
pub use adapter::persistence::postgres::{
    sdtm_project_domain_repo::SdtmProjectDomainRepoPg,
    sdtm_project_variable_repo::SdtmProjectVariableRepoPg,
    sdtm_project_version_repo::SdtmProjectVersionRepoPg,
};
pub use adapter::service::project::ProjectLookupImpl;
pub use domain::{
    ConversionRule, DomainCategory, DomainError, FromCrf, ProjectLookup, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmProjectDomain, SdtmProjectDomainNew,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectVariable,
    SdtmProjectVariableNew, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionRepository,
    SdtmProjectVersionUpdate, SdtmRole, SdtmVariableControlled, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableType, VariableSource,
};
pub use usecase::{
    CreateSdtmProjectDomain, CreateSdtmProjectVariable, CreateSdtmProjectVersion,
    SdtmProjectDomainView, SdtmProjectUsecase, SdtmProjectUsecaseConfig, SdtmProjectVariableView,
    SdtmProjectVersionView, UpdateSdtmProjectDomain, UpdateSdtmProjectVariable,
    UpdateSdtmProjectVersion, UsecaseError,
};

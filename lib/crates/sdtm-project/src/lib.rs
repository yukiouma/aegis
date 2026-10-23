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
//! `SdtmVariableCore`, `SdtmRole` and the description DTOs) are copied
//! rather than imported.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::persistence::postgres::{
    sdtm_project_domain_override_repo::SdtmProjectDomainOverrideRepoPg,
    sdtm_project_domain_repo::SdtmProjectDomainRepoPg,
    sdtm_project_variable_override_repo::SdtmProjectVariableOverrideRepoPg,
    sdtm_project_variable_repo::SdtmProjectVariableRepoPg,
    sdtm_project_version_repo::SdtmProjectVersionRepoPg,
};
pub use domain::{
    DomainCategory, DomainError, ProjectLookup, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainOverride,
    SdtmProjectDomainOverrideNew, SdtmProjectDomainOverrideRepository,
    SdtmProjectDomainOverrideUpdate, SdtmProjectDomainRepository, SdtmProjectDomainUpdate,
    SdtmProjectVariable, SdtmProjectVariableNew, SdtmProjectVariableOverride,
    SdtmProjectVariableOverrideNew, SdtmProjectVariableOverrideRepository,
    SdtmProjectVariableOverrideUpdate, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionRepository,
    SdtmProjectVersionUpdate, SdtmRole, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableType,
};
pub use usecase::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion, SdtmProjectDomainOverrideView,
    SdtmProjectDomainView, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
    SdtmProjectVariableOverrideView, SdtmProjectVariableView, SdtmProjectVersionView,
    UpdateSdtmProjectDomain, UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable,
    UpdateSdtmProjectVariableOverride, UpdateSdtmProjectVersion, UsecaseError,
};

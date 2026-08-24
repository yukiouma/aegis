//! # domain-model crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC SDTM domain model aggregates
//! and an async `DomainModelUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmDomainNew, SdtmDomainRepository, SdtmDomainUpdate, SdtmRole, SdtmVariable,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableNew,
    SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate, SdtmVersion, SdtmVersionNew,
    SdtmVersionRepository, SdtmVersionUpdate,
};
pub use usecase::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion, DomainModelUsecase,
    DomainModelUsecaseConfig, SdtmDomainView, SdtmVariableView, SdtmVersionView, UpdateSdtmDomain,
    UpdateSdtmVariable, UpdateSdtmVersion, UsecaseError,
};

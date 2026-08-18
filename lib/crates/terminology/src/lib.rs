//! # terminology crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC terminology aggregates and an async
//! `TerminologyUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery,
    CodeItemUpdate, CodeList, CodeListNew, CodeListRepository, CodeListSearchHit,
    CodeListSearchQuery, CodeListUpdate, DomainError, TerminologyKind,
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
pub use usecase::{
    CodeItemView, CodeListView, CreateCodeItem, CreateCodeList, CreateTerminologyVersion,
    TerminologyUsecase, TerminologyUsecaseConfig, TerminologyVersionView, UpdateCodeItem,
    UpdateCodeList, UpdateTerminologyVersion, UsecaseError,
};